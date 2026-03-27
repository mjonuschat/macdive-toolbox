//! SeaORM-based cache for iNaturalist taxon lookups.
//!
//! Avoids redundant API calls by storing taxon JSON in a local SQLite database.

use entity::taxon_cache;
use sea_orm::prelude::*;
use sea_orm::{QueryTrait, Set, sea_query::OnConflict};
use tracing::instrument;

use crate::error::{Error, Result};

use super::types::Taxon;

/// Key for looking up a cached taxon -- either by numeric ID or matched name.
pub(crate) enum CacheLookupKey<'a> {
    Id(i32),
    Name(&'a str),
}

/// Insert or update a taxon in the local cache.
///
/// Uses the taxon ID as the conflict key. If `matched_name` is `None`,
/// falls back to the taxon's own `name` field.
#[instrument(name = "cache-taxon", skip(db, taxon))]
pub(crate) async fn cache_taxon(
    db: &DbConn,
    taxon: &Taxon,
    matched_name: Option<&str>,
) -> Result<()> {
    let matched_name = matched_name
        .or(taxon.name.as_deref())
        .ok_or_else(|| Error::INaturalist("No name information available".to_string()))?;

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

/// Look up a taxon in the local cache by ID or name.
///
/// Returns `None` if no matching record exists or deserialization fails.
pub(crate) async fn cached_taxon(db: &DbConn, key: CacheLookupKey<'_>) -> Result<Option<Taxon>> {
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
