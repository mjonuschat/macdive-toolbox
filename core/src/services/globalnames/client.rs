//! HTTP client for the GlobalNames verifier API.
//!
//! Sends species name strings to the GlobalNames verifier and returns the
//! structured verification response. Uses `reqwest` for HTTP transport.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::util::rate_limit::{ApiRateLimiter, wait_for_permit};

/// GlobalNames data source ID for WoRMS (World Register of Marine Species).
pub(crate) const SOURCE_WORMS: usize = 9;

/// GlobalNames data source ID for GBIF (Global Biodiversity Information Facility).
pub(crate) const SOURCE_GBIF: usize = 11;

const VERIFIER_URL: &str = "https://verifier.globalnames.org/api/v1/verifications";

/// Shared HTTP client reused across all GlobalNames API requests.
///
/// Creating a new `reqwest::Client` per request is wasteful because it allocates a new
/// connection pool each time. This static instance reuses the pool for the process lifetime.
static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

/// How closely a returned name matches the submitted name string.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) enum MatchType {
    NoMatch,
    PartialFuzzy,
    PartialExact,
    Fuzzy,
    #[default]
    Exact,
    Virus,
    FacetedSearch,
}

/// Request body sent to the GlobalNames verifier endpoint.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerificationRequest {
    name_strings: Vec<String>,
    data_sources: Vec<usize>,
    with_all_matches: bool,
    with_capitalization: bool,
    with_species_group: bool,
    with_stats: bool,
    main_taxon_threshold: f32,
}

/// A single match entry within a `VerifiedNameData` result.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VerificationResultData {
    pub(crate) data_source_id: i32,
    pub(crate) record_id: String,
    pub(crate) sort_score: f32,
    pub(crate) matched_name: String,
    pub(crate) matched_canonical_simple: String,
    pub(crate) current_name: String,
    pub(crate) current_canonical_simple: String,
    pub(crate) match_type: MatchType,
}

/// Verification results for a single submitted name string.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VerifiedNameData {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) match_type: MatchType,
    pub(crate) results: Vec<VerificationResultData>,
    pub(crate) data_sources_num: i32,
}

/// Top-level response from the GlobalNames verifier API.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VerificationResponse {
    pub(crate) names: Vec<VerifiedNameData>,
}

/// Submit a single species name to the GlobalNames verifier and return the response.
///
/// Waits for a rate-limit permit before sending the request. Queries both the
/// WoRMS and GBIF data sources and requests all matches.
///
/// # Arguments
///
/// * `name` - The species name string to verify
/// * `limiter` - Rate limiter to throttle outgoing requests
///
/// # Errors
///
/// Returns [`Error::GlobalNames`] if the HTTP request or JSON decoding fails.
#[instrument(name = "globalnames-verify", skip(limiter))]
pub(crate) async fn verify_name(
    name: &str,
    limiter: &ApiRateLimiter,
) -> Result<VerificationResponse> {
    wait_for_permit(limiter).await;

    let response = HTTP_CLIENT
        .post(VERIFIER_URL)
        .json(&VerificationRequest {
            name_strings: vec![name.to_string()],
            data_sources: vec![SOURCE_WORMS, SOURCE_GBIF],
            with_all_matches: true,
            with_capitalization: true,
            ..Default::default()
        })
        .send()
        .await
        .map_err(|e| Error::GlobalNames(format!("Request failed: {e}")))?
        .json::<VerificationResponse>()
        .await
        .map_err(|e| Error::GlobalNames(format!("Failed to decode response: {e}")))?;

    Ok(response)
}
