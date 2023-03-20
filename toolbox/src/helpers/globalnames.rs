use ::entity::{prelude::VerifiedName, verified_name};
use anyhow::bail;
use governor::clock::QuantaClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Jitter, Quota, RateLimiter};
use nonzero_ext::nonzero;
use once_cell::sync::Lazy;
use sea_orm::prelude::*;
use sea_orm::{sea_query::OnConflict, Set};
use serde_derive::{Deserialize, Serialize};
use std::time::Duration;
use surf::http::mime;
use tracing::instrument;
use uuid::Uuid;

use crate::helpers::database;

const SOURCE_WORMS: usize = 9;
const SOURCE_GBIF: usize = 11;
const VERIFIER_URL: &str = "https://verifier.globalnames.org/api/v1/verifications";
static VERIFIER_API_LIMIT: Lazy<RateLimiter<NotKeyed, InMemoryState, QuantaClock>> =
    Lazy::new(|| RateLimiter::direct(Quota::per_minute(nonzero!(60u32))));

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
enum MatchType {
    NoMatch,
    PartialFuzzy,
    PartialExact,
    Fuzzy,
    #[default]
    Exact,
    Virus,
    FacetedSearch,
}

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

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerificationResultData {
    data_source_id: i32,
    record_id: String,
    sort_score: f32,
    matched_name: String,
    matched_canonical_simple: String,
    current_name: String,
    current_canonical_simple: String,
    match_type: MatchType,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerifiedNameData {
    id: Uuid,
    name: String,
    match_type: MatchType,
    results: Vec<VerificationResultData>,
    data_sources_num: i32,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationResponse {
    names: Vec<VerifiedNameData>,
}

#[instrument]
pub async fn verify_name(name: &str) -> anyhow::Result<VerificationResponse> {
    VERIFIER_API_LIMIT
        .until_ready_with_jitter(Jitter::new(
            Duration::from_millis(50),
            Duration::from_millis(250),
        ))
        .await;

    let response = surf::post(VERIFIER_URL)
        .content_type(mime::JSON)
        .body_json(&VerificationRequest {
            name_strings: vec![name.to_string()],
            data_sources: vec![SOURCE_WORMS, SOURCE_GBIF],
            with_all_matches: true,
            with_capitalization: true,
            ..Default::default()
        })
        .map_err(|e| anyhow::anyhow!("Request error: {e}"))?
        .recv_json()
        .await
        .map_err(|e| anyhow::anyhow!("Request error: {e}"))?;

    Ok(response)
}

async fn cache_verified_name(name: &str, data: &VerificationResultData) -> anyhow::Result<()> {
    let db = database::connect().await?;
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

#[instrument(name = "normalize-name")]
pub async fn normalize(name: &str) -> anyhow::Result<String> {
    let db = database::connect().await?;
    // Check the cache
    let cached_record: Option<verified_name::Model> = VerifiedName::find()
        .filter(verified_name::Column::MatchedName.eq(name))
        .filter(
            verified_name::Column::VerifiedAt.gte(chrono::Utc::now() - chrono::Duration::days(90)),
        )
        .one(db)
        .await?;

    if let Some(data) = cached_record {
        return Ok(data.current_name);
    }

    let response = verify_name(name).await?;
    match response.names.into_iter().next() {
        None => Ok(name.to_string()),
        Some(record) => match record.results.into_iter().next() {
            None => bail!("Matched name without result in response"),
            Some(data) => {
                cache_verified_name(name, &data).await?;
                Ok(data.current_canonical_simple)
            }
        },
    }
}
