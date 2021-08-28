use crate::inaturalist::{
    types::ResultsTaxa, types::Taxon, types::TAXON_FIELDS, INATURALIST_CACHE, INAT_API_LIMIT,
};

use std::collections::HashSet;
use std::time::Duration;

use crate::inaturalist::types::TaxaAutocompleteQuery;
use anyhow::Result;
use governor::Jitter;
use itertools::Itertools;
use surf::{http::mime, RequestBuilder};

const INAT_TAXON_CACHE_TREE: &str = "taxa";
const INAT_NAME_CACHE_TREE: &str = "names";

fn cache_taxon(taxon: &Taxon, original_name: Option<&str>) -> Result<()> {
    let taxon_cache = INATURALIST_CACHE.open_tree(INAT_TAXON_CACHE_TREE)?;
    let name_cache = INATURALIST_CACHE.open_tree(INAT_NAME_CACHE_TREE)?;

    let taxon_id = taxon.id.to_le_bytes();
    let taxon_name = taxon.name.clone().map_or_else(
        || Err(anyhow::anyhow!("No name found")),
        |v| Ok(v.to_lowercase()),
    )?;

    taxon_cache.insert(taxon_id, rmp_serde::encode::to_vec(&taxon)?)?;
    name_cache.insert(&taxon_name, &taxon_id)?;

    if let Some(name) = original_name {
        name_cache.insert(name.trim().to_lowercase(), &taxon_id)?;
    }

    Ok(())
}

pub async fn cache_species(species: &[&str]) -> Result<Vec<String>> {
    let mut normalized_names: Vec<String> = Vec::new();
    let mut ancestor_ids: HashSet<i32> = HashSet::new();
    for name in species {
        if let Ok(taxon) = get_taxon_by_name(name).await {
            normalized_names.push(
                taxon
                    .name
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| name.to_string()),
            );
            if let Some(ids) = taxon.ancestor_ids {
                ancestor_ids.extend(&ids);
            }
        } else {
            eprintln!("No taxon found for {}", name)
        }
    }
    let ancestor_ids: Vec<i32> = ancestor_ids.into_iter().collect();
    get_taxon_by_ids(&ancestor_ids).await?;

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
        .map_err(|e| anyhow::anyhow!("Error decoding json: {}", e))?;

    Ok(taxa.results)
}

async fn lookup_taxon_by_id(id: i32) -> Result<Taxon> {
    lookup_taxon_by_ids(&[id])
        .await?
        .first()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No taxon found: {}", id))
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
        // .header("Authorization", API_TOKEN)
        .content_type(mime::JSON)
        .body(TAXON_FIELDS.clone());

    lookup_taxon(request).await
}

async fn lookup_taxon_by_name(name: &str) -> Result<Taxon> {
    // TODO: Debug logging
    let request = surf::post("https://api.inaturalist.org/v2/taxa/autocomplete")
        .header("X-HTTP-Method-Override", "GET")
        // .header("Authorization", API_TOKEN)
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
        .ok_or_else(|| anyhow::anyhow!("No taxon found: {}", name))
}

pub async fn get_taxon_by_ids(ids: &[i32]) -> Result<Vec<Taxon>> {
    let taxon_cache = INATURALIST_CACHE.open_tree(INAT_TAXON_CACHE_TREE)?;

    let wanted_ids = ids.iter().copied().collect::<HashSet<i32>>();

    let cache_ids = wanted_ids
        .iter()
        .filter_map(|id| match taxon_cache.contains_key(id.to_le_bytes()) {
            Ok(true) => Some(*id),
            Ok(false) => None,
            Err(e) => {
                eprintln!("Error looking up id {} in iNaturalist cache: {}", id, e);
                None
            }
        })
        .collect::<HashSet<i32>>();

    let missing_ids: Vec<_> = wanted_ids.difference(&cache_ids).copied().collect();

    if !missing_ids.is_empty() {
        for chunk in &missing_ids.iter().chunks(50) {
            let ids: Vec<i32> = chunk.copied().collect();
            let taxa = lookup_taxon_by_ids(&ids).await?;
            for taxon in taxa {
                cache_taxon(&taxon, None)?;
            }
        }
    }

    let result = wanted_ids
        .iter()
        .filter_map(|v| taxon_cache.get(v.to_le_bytes()).ok())
        .filter_map(|v| {
            v.map(|v| {
                let result: Result<Taxon> = rmp_serde::from_read_ref(&v).map_err(|e| e.into());
                result
            })
        })
        .collect::<Result<Vec<Taxon>>>()?;

    Ok(result)
}

pub async fn get_taxon_by_id(id: i32) -> Result<Taxon> {
    let taxon_cache = INATURALIST_CACHE.open_tree(INAT_TAXON_CACHE_TREE)?;

    match taxon_cache.get(id.to_le_bytes())? {
        Some(ref buf) => rmp_serde::from_read_ref(buf).map_err(|e| e.into()),
        None => {
            let taxon = lookup_taxon_by_id(id).await?;
            cache_taxon(&taxon, None)?;
            Ok(taxon)
        }
    }
}

pub async fn get_taxon_by_name(scientific_name: &str) -> Result<Taxon> {
    let taxon_cache = INATURALIST_CACHE.open_tree(INAT_TAXON_CACHE_TREE)?;
    let name_cache = INATURALIST_CACHE.open_tree(INAT_NAME_CACHE_TREE)?;

    let name_key = scientific_name.trim().to_lowercase();
    let result: Option<Taxon> = match name_cache.get(name_key)? {
        Some(taxon_key) => {
            let buf = taxon_cache.get(taxon_key)?;
            match buf {
                Some(ref buf) => rmp_serde::from_read_ref(buf)?,
                None => None,
            }
        }
        None => None,
    };

    match result {
        Some(taxon) => Ok(taxon),
        None => {
            let taxon = lookup_taxon_by_name(scientific_name).await?;
            cache_taxon(&taxon, Some(scientific_name))?;
            Ok(taxon)
        }
    }
}
