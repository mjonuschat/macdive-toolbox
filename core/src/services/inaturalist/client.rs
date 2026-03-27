//! HTTP client for the iNaturalist API v2.
//!
//! All functions accept a rate limiter reference to throttle outgoing requests.
//! Uses `reqwest` for HTTP transport.

use std::sync::LazyLock;

use tracing::instrument;

use crate::error::{Error, Result};
use crate::util::rate_limit::{ApiRateLimiter, wait_for_permit};

use super::types::{ResultsTaxa, TAXON_FIELDS, TaxaAutocompleteQuery, Taxon};

/// Shared HTTP client reused across all iNaturalist API requests.
///
/// Creating a new `reqwest::Client` per request is wasteful because it allocates a new
/// connection pool each time. This static instance reuses the pool for the process lifetime.
static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

/// Perform a rate-limited POST request (with GET override) and decode the taxa results.
async fn lookup_taxon(
    request: reqwest::RequestBuilder,
    limiter: &ApiRateLimiter,
) -> Result<Vec<Taxon>> {
    wait_for_permit(limiter).await;

    let res = request.send().await?;
    let taxa: ResultsTaxa = res.json().await?;

    Ok(taxa.results)
}

/// Fetch a single taxon by its numeric ID.
///
/// Returns an error if no taxon matches the given ID.
#[instrument(name = "inat-fetch-by-id", skip(limiter))]
pub(crate) async fn lookup_taxon_by_id(id: i32, limiter: &ApiRateLimiter) -> Result<Taxon> {
    lookup_taxon_by_ids(&[id], limiter)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| Error::INaturalist(format!("No taxon found for id: {id}")))
}

/// Fetch multiple taxa by their numeric IDs in a single request.
///
/// # Errors
///
/// Returns an error if `ids` is empty or the API request fails.
pub(crate) async fn lookup_taxon_by_ids(
    ids: &[i32],
    limiter: &ApiRateLimiter,
) -> Result<Vec<Taxon>> {
    if ids.is_empty() {
        return Err(Error::INaturalist(
            "Need at least one Taxon ID to look up".to_string(),
        ));
    }

    let id_str = ids
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<String>>()
        .join(",");

    let url = format!("https://api.inaturalist.org/v2/taxa/{id_str}");

    let request = HTTP_CLIENT
        .post(&url)
        .header("X-HTTP-Method-Override", "GET")
        .header("Content-Type", "application/json")
        .json(&*TAXON_FIELDS);

    lookup_taxon(request, limiter).await
}

/// Fetch a single taxon by scientific name via the autocomplete API.
///
/// Returns the best (first) match, or an error if no match is found.
#[instrument(name = "inat-fetch-by-name", skip(limiter))]
pub(crate) async fn lookup_taxon_by_name(name: &str, limiter: &ApiRateLimiter) -> Result<Taxon> {
    let request = HTTP_CLIENT
        .post("https://api.inaturalist.org/v2/taxa/autocomplete")
        .header("X-HTTP-Method-Override", "GET")
        .header("Content-Type", "application/json")
        .query(&TaxaAutocompleteQuery {
            q: name.to_string(),
        })
        .json(&*TAXON_FIELDS);

    lookup_taxon(request, limiter)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| Error::INaturalist(format!("No taxon found for name: {name}")))
}
