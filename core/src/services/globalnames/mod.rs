//! GlobalNames verifier service: species name normalization with local caching.
//!
//! The public API is a single [`normalize`] function that resolves a species
//! name to its current canonical form. Results are cached locally for
//! 90 days to avoid repeated API calls.

mod cache;
mod client;

use std::sync::LazyLock;

use sea_orm::prelude::*;
use tracing::instrument;

use crate::error::{Error, Result};
use crate::util::rate_limit::{ApiRateLimiter, create_rate_limiter};

use cache::{cache_verified_name, cached_verified_name};
use client::verify_name;

/// Rate limiter for GlobalNames verifier API calls (60 requests per minute).
static VERIFIER_API_LIMIT: LazyLock<ApiRateLimiter> = LazyLock::new(|| create_rate_limiter(60));

/// Resolve a species name to its current canonical form via the GlobalNames verifier.
///
/// Checks the local cache first (entries valid for 90 days), then falls back to
/// the remote API on a cache miss. Successful API results are written back to the
/// cache before returning.
///
/// Returns the original `name` unchanged when the API returns no matches, so
/// callers always receive a usable string.
///
/// # Arguments
///
/// * `db` - Database connection used for the local verification cache
/// * `name` - Species name string to normalize
///
/// # Errors
///
/// Returns [`Error::GlobalNames`] if the verifier API call fails, or
/// [`Error::Database`] if a cache read or write fails.
#[instrument(name = "globalnames-normalize")]
pub async fn normalize(db: &DbConn, name: &str) -> Result<String> {
    if let Some(cached) = cached_verified_name(db, name).await? {
        return Ok(cached);
    }

    let response = verify_name(name, &VERIFIER_API_LIMIT).await?;

    match response.names.into_iter().next() {
        None => Ok(name.to_string()),
        Some(record) => match record.results.into_iter().next() {
            None => Err(Error::GlobalNames(
                "Matched name without result in response".to_string(),
            )),
            Some(data) => {
                cache_verified_name(db, name, &data).await?;
                Ok(data.current_canonical_simple)
            }
        },
    }
}
