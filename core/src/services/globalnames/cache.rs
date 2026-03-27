//! SeaORM-based cache for GlobalNames verification results.
//!
//! Avoids redundant API calls by storing verified name mappings in a local
//! SQLite database. Cache entries are considered valid for 90 days.

use entity::{prelude::VerifiedName, verified_name};
use sea_orm::prelude::*;
use sea_orm::{Set, sea_query::OnConflict};
use tracing::instrument;

use crate::error::Result;

use super::client::VerificationResultData;

/// Cache TTL: verification results older than this many days are considered stale.
const CACHE_TTL_DAYS: i64 = 90;

/// Look up a previously verified name in the local cache.
///
/// Returns the cached canonical name if a record for `name` exists and was
/// verified within the last [`CACHE_TTL_DAYS`] days.
///
/// # Arguments
///
/// * `db` - Database connection for the cache
/// * `name` - The species name string to look up
#[instrument(name = "globalnames-cache-lookup", skip(db))]
pub(crate) async fn cached_verified_name(db: &DbConn, name: &str) -> Result<Option<String>> {
    let cutoff = chrono::Utc::now() - chrono::Duration::days(CACHE_TTL_DAYS);

    let record: Option<verified_name::Model> = VerifiedName::find()
        .filter(verified_name::Column::MatchedName.eq(name))
        .filter(verified_name::Column::VerifiedAt.gte(cutoff))
        .one(db)
        .await?;

    Ok(record.map(|r| r.current_name))
}

/// Insert or update a verified name mapping in the local cache.
///
/// Uses `matched_name` as the conflict key; on conflict the `current_name`
/// and `verified_at` columns are updated to reflect the latest result.
///
/// # Arguments
///
/// * `db` - Database connection for the cache
/// * `name` - The original species name string that was submitted for verification
/// * `data` - The best verification result returned by the API
#[instrument(name = "globalnames-cache-store", skip(db, data))]
pub(crate) async fn cache_verified_name(
    db: &DbConn,
    name: &str,
    data: &VerificationResultData,
) -> Result<()> {
    let cache_record = verified_name::ActiveModel {
        matched_name: Set(name.to_string()),
        current_name: Set(data.current_canonical_simple.clone()),
        verified_at: Set(chrono::Utc::now()),
        ..Default::default()
    };

    verified_name::Entity::insert(cache_record)
        .on_conflict(
            OnConflict::column(verified_name::Column::MatchedName)
                .update_columns([
                    verified_name::Column::CurrentName,
                    verified_name::Column::VerifiedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await?;

    Ok(())
}
