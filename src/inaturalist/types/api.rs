use super::models::*;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub(in crate::inaturalist) static TAXON_FIELDS: Lazy<Value> = Lazy::new(|| {
    json!(
        {
            "fields": {
                "id": true,
                "uuid": true,
                "ancestor_ids": true,
                "ancestry": true,
                "complete_rank": true,
                "conservation_status": {
                    "authority": true,
                    "description": true,
                    "iucn": true,
                    "iucn_status": true,
                    "iucn_status_code": true,
                    "status": true,
                    "status_name": true,
                    "url": true,
                },
                "default_photo": {
                        "id": true,
                        "attribution": true,
                        "large_url": true,
                        "license_code": true,
                        "medium_url": true,
                        "native_page_url": true,
                        "native_photo_id": true,
                        "original_dimensions": true,
                        "original_url": true,
                        "small_url": true,
                        "square_url": true,
                        "type": true,
                        "url": true,
                },
                "endemic": true,
                "iconic_taxon_name": true,
                "is_active": true,
                "name": true,
                "native": true,
                "observations_count": true,
                "parent_id": true,
                "preferred_common_name": true,
                "rank": true,
                "rank_level": true,
                "threatened": true,
                "wikipedia_summary  ": true,
                "wikipedia_url": true
            }
        }
    )
});

#[derive(Debug, Deserialize, Serialize)]
pub(in crate::inaturalist) struct TaxaAutocompleteQuery {
    pub(crate) q: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(in crate::inaturalist) struct ResultsTaxa {
    pub total_results: i32,
    pub page: i32,
    pub per_page: i32,
    pub results: Vec<Taxon>,
}
