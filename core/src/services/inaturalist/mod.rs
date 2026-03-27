//! iNaturalist API service: taxon lookups with local caching.
//!
//! Public functions check the local SeaORM cache first and fall back to the
//! iNaturalist API v2 when a cache miss occurs (unless running in offline mode).

mod cache;
mod client;
pub mod types;

use std::collections::HashSet;
use std::sync::LazyLock;

use entity::taxon_cache;
use itertools::Itertools;
use sea_orm::QuerySelect;
use sea_orm::prelude::*;
use tracing::instrument;

use crate::error::{Error, Result};
use crate::util::rate_limit::{ApiRateLimiter, create_rate_limiter};

use cache::{CacheLookupKey, cache_taxon, cached_taxon};
use client::{lookup_taxon_by_id, lookup_taxon_by_ids, lookup_taxon_by_name};
pub use types::Taxon;

/// Rate limiter for iNaturalist API calls (60 requests per minute).
static INAT_API_LIMIT: LazyLock<ApiRateLimiter> = LazyLock::new(|| create_rate_limiter(60));

/// Look up a single taxon by ID, consulting the cache first.
///
/// In offline mode, returns an error if the taxon is not in the cache.
///
/// # Arguments
///
/// * `db` - Database connection for the cache
/// * `id` - iNaturalist taxon ID
/// * `offline` - When true, skip API calls and only use cached data
#[instrument(name = "inat-lookup", skip(db, offline))]
pub async fn get_taxon_by_id(db: &DbConn, id: i32, offline: bool) -> Result<Taxon> {
    match cached_taxon(db, CacheLookupKey::Id(id)).await? {
        Some(taxon) => Ok(taxon),
        None => {
            if offline {
                return Err(Error::INaturalist(
                    "Running in offline mode - taxon lookup disabled".to_string(),
                ));
            }
            let taxon = lookup_taxon_by_id(id, &INAT_API_LIMIT).await?;
            cache_taxon(db, &taxon, None).await?;
            Ok(taxon)
        }
    }
}

/// Look up a single taxon by scientific name, consulting the cache first.
///
/// In offline mode, returns an error if the taxon is not in the cache.
///
/// # Arguments
///
/// * `db` - Database connection for the cache
/// * `scientific_name` - Species scientific name to search for
/// * `offline` - When true, skip API calls and only use cached data
#[instrument(name = "inat-lookup", skip(db, offline))]
pub async fn get_taxon_by_name(db: &DbConn, scientific_name: &str, offline: bool) -> Result<Taxon> {
    match cached_taxon(db, CacheLookupKey::Name(scientific_name)).await? {
        Some(taxon) => Ok(taxon),
        None => {
            if offline {
                return Err(Error::INaturalist(
                    "Running in offline mode - taxon lookup disabled".to_string(),
                ));
            }
            let taxon = lookup_taxon_by_name(scientific_name, &INAT_API_LIMIT).await?;
            cache_taxon(db, &taxon, Some(scientific_name)).await?;
            Ok(taxon)
        }
    }
}

/// Bulk-fetch taxa by ID, using the cache where possible.
///
/// Any IDs not already in the cache are fetched from the iNaturalist API
/// in chunks of 25 and stored in the cache before returning.
///
/// # Arguments
///
/// * `db` - Database connection for the cache
/// * `ids` - Slice of iNaturalist taxon IDs to fetch
#[instrument(name = "inat-lookup-bulk", skip(db))]
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
            let taxa = lookup_taxon_by_ids(&ids, &INAT_API_LIMIT).await?;
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
                .map_err(|e| Error::INaturalist(format!("Error deserializing cached taxon: {e}")))
        })
        .collect::<Result<Vec<Taxon>>>()
}

/// Pre-fetch and cache all species and their ancestors.
///
/// Looks up each species by name (fetching from the API if not already cached)
/// then bulk-fetches any ancestor taxa that are missing from the cache.
///
/// # Arguments
///
/// * `db` - Database connection for the cache
/// * `species` - Slice of scientific names to look up
/// * `offline` - When true, skip API calls and only use cached data
#[instrument(name = "inat-cache-species", skip_all)]
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
                    .unwrap_or_else(|| (*name).to_string()),
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
