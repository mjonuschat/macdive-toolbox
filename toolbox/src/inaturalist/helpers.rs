use std::collections::HashSet;
use std::time::Duration;

use crate::inaturalist::{
    INAT_API_LIMIT, types::ResultsTaxa, types::TAXON_FIELDS, types::TaxaAutocompleteQuery,
    types::Taxon,
};
use anyhow::{Result, anyhow, bail};
use entity::taxon_cache;
use governor::Jitter;
use itertools::Itertools;
use sea_orm::prelude::*;
use sea_orm::{QuerySelect, QueryTrait, Set, sea_query::OnConflict};
use surf::{RequestBuilder, http::mime};
use tracing::instrument;

enum CacheLookupKey<'a> {
    Id(i32),
    Name(&'a str),
}

#[instrument(name = "cache-taxon", skip(db, taxon))]
async fn cache_taxon(db: &DbConn, taxon: &Taxon, matched_name: Option<&str>) -> Result<()> {
    let matched_name = matched_name
        .or(taxon.name.as_deref())
        .ok_or(anyhow!("No name information available"))?;

    let cache_record = taxon_cache::ActiveModel {
        taxon_id: Set(taxon.id),
        matched_name: Set(matched_name.to_string()),
        taxon: Set(serde_json::to_value(taxon)?),
        downloaded_at: Set(chrono::Utc::now()),
        ..Default::default()
    };
    taxon_cache::Entity::insert(cache_record)
        .on_conflict(
            OnConflict::columns([taxon_cache::Column::TaxonId])
                .update_columns([
                    taxon_cache::Column::MatchedName,
                    taxon_cache::Column::Taxon,
                    taxon_cache::Column::DownloadedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await?;
    Ok(())
}

async fn cached_taxon(db: &DbConn, key: CacheLookupKey<'_>) -> Result<Option<Taxon>> {
    let (id, name) = match key {
        CacheLookupKey::Id(id) => (Some(id), None),
        CacheLookupKey::Name(name) => (None, Some(name)),
    };

    let result = taxon_cache::Entity::find()
        .apply_if(id, |query, v| {
            query.filter(taxon_cache::Column::TaxonId.eq(v))
        })
        .apply_if(name, |query, v| {
            query.filter(taxon_cache::Column::MatchedName.eq(v))
        })
        .one(db)
        .await?;

    Ok(result.and_then(|record| serde_json::from_value(record.taxon).ok()))
}

/// Pre-fetch and cache all species and their ancestors.
///
/// Looks up each species by name (fetching from the API if not already cached)
/// then bulk-fetches any ancestor taxa that are missing from the cache.
#[instrument(name = "cache-species", skip_all)]
pub async fn cache_species(db: &DbConn, species: &[&str], offline: bool) -> Result<Vec<String>> {
    let mut normalized_names: Vec<String> = Vec::new();
    let mut ancestor_ids: HashSet<i32> = HashSet::new();
    for name in species {
        let result = get_taxon_by_name(db, name, offline).await;
        if let Ok(taxon) = result {
            normalized_names.push(
                taxon
                    .name
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| ToString::to_string(&name)),
            );
            if let Some(ids) = taxon.ancestor_ids {
                ancestor_ids.extend(&ids);
            }
        }
    }
    let ancestor_ids: Vec<i32> = ancestor_ids.into_iter().collect();
    get_taxon_by_ids(db, &ancestor_ids).await?;

    Ok(normalized_names)
}

async fn lookup_taxon(request: RequestBuilder) -> Result<Vec<Taxon>> {
    INAT_API_LIMIT
        .until_ready_with_jitter(Jitter::new(
            Duration::from_millis(50),
            Duration::from_millis(250),
        ))
        .await;
    let mut res = request
        .await
        .map_err(|e| anyhow::anyhow!("Error talking to server: {}", e))?;

    let taxa: ResultsTaxa = res
        .body_json()
        .await
        .map_err(|e| anyhow::anyhow!("Error decoding json: {e}"))?;

    Ok(taxa.results)
}

#[instrument(name = "fetch")]
async fn lookup_taxon_by_id(id: i32) -> Result<Taxon> {
    lookup_taxon_by_ids(&[id])
        .await?
        .first()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No taxon found for id: {}", id))
}

async fn lookup_taxon_by_ids(ids: &[i32]) -> Result<Vec<Taxon>> {
    if ids.is_empty() {
        anyhow::bail!("Need at least one Taxon ID to look up");
    }

    let id_str = ids
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<String>>()
        .join(",");

    let url = format!("https://api.inaturalist.org/v2/taxa/{ids}", ids = id_str);

    let request = surf::post(url)
        .header("X-HTTP-Method-Override", "GET")
        .content_type(mime::JSON)
        .body(TAXON_FIELDS.clone());

    lookup_taxon(request).await
}

#[instrument(name = "fetch")]
async fn lookup_taxon_by_name(name: &str) -> Result<Taxon> {
    let request = surf::post("https://api.inaturalist.org/v2/taxa/autocomplete")
        .header("X-HTTP-Method-Override", "GET")
        .content_type(mime::JSON)
        .body(TAXON_FIELDS.clone())
        .query(&TaxaAutocompleteQuery {
            q: name.to_string(),
        })
        .map_err(|_| anyhow::anyhow!("Error parsing query params"))?;

    lookup_taxon(request)
        .await?
        .first()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No taxon found for name: {}", name))
}

/// Bulk-fetch taxa by ID, using the cache where possible.
///
/// Any IDs not already in the cache are fetched from the iNaturalist API
/// in chunks of 25 and stored in the cache before returning.
#[instrument(name = "lookup-bulk")]
pub async fn get_taxon_by_ids(db: &DbConn, ids: &[i32]) -> Result<Vec<Taxon>> {
    let cache_ids: HashSet<i32> = taxon_cache::Entity::find()
        .select_only()
        .column(taxon_cache::Column::TaxonId)
        .filter(taxon_cache::Column::TaxonId.is_in(ids.to_vec()))
        .into_tuple()
        .all(db)
        .await?
        .iter()
        .map(|(id,)| *id)
        .collect();

    let wanted_ids = ids.iter().copied().collect::<HashSet<i32>>();
    let missing_ids: Vec<_> = wanted_ids.difference(&cache_ids).copied().collect();

    if !missing_ids.is_empty() {
        for chunk in &missing_ids.iter().chunks(25) {
            let ids: Vec<i32> = chunk.copied().collect();
            let taxa = lookup_taxon_by_ids(&ids).await?;
            for taxon in taxa {
                cache_taxon(db, &taxon, None).await?;
            }
        }
    }

    taxon_cache::Entity::find()
        .filter(taxon_cache::Column::TaxonId.is_in(ids.to_vec()))
        .all(db)
        .await?
        .into_iter()
        .map(|model| {
            serde_json::from_value(model.taxon)
                .map_err(|e| anyhow!("Error deserializing cached taxon data: {e}"))
        })
        .collect::<Result<Vec<Taxon>>>()
}

/// Look up a single taxon by ID, consulting the cache first.
///
/// In offline mode, returns an error if the taxon is not in the cache.
#[instrument(name = "lookup", skip(db, offline))]
pub async fn get_taxon_by_id(db: &DbConn, id: i32, offline: bool) -> Result<Taxon> {
    match cached_taxon(db, CacheLookupKey::Id(id)).await? {
        Some(taxon) => Ok(taxon),
        None => {
            if offline {
                bail!("Running in offline mode - taxon lookup disabled");
            }
            let taxon = lookup_taxon_by_id(id).await?;
            cache_taxon(db, &taxon, None).await?;
            Ok(taxon)
        }
    }
}

/// Look up a single taxon by scientific name, consulting the cache first.
///
/// In offline mode, returns an error if the taxon is not in the cache.
#[instrument(name = "lookup", skip(db, offline))]
pub async fn get_taxon_by_name(db: &DbConn, scientific_name: &str, offline: bool) -> Result<Taxon> {
    match cached_taxon(db, CacheLookupKey::Name(scientific_name)).await? {
        Some(taxon) => Ok(taxon),
        None => {
            if offline {
                bail!("Running in offline mode - taxon lookup disabled");
            }
            let taxon = lookup_taxon_by_name(scientific_name).await?;
            cache_taxon(db, &taxon, Some(scientific_name)).await?;
            Ok(taxon)
        }
    }
}
