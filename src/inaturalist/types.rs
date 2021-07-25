use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;

mod api;
mod models;

pub(in crate::inaturalist) use api::*;
pub use models::*;

/// SQLx: Return type used for cache_taxa table
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedTaxon {
    pub id: i32,
    pub uuid: Option<Uuid>,
    pub name: String,
    pub taxon: Json<Taxon>,
}
