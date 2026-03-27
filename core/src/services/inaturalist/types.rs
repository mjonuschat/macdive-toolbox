//! iNaturalist API types and model structs.
//!
//! These correspond to the iNaturalist API v2 schema.
//! See <http://api.inaturalist.org/v2/docs/>.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// JSON fields specification sent with every iNaturalist API v2 request
/// to control which fields are returned in the response.
pub(crate) static TAXON_FIELDS: LazyLock<Value> = LazyLock::new(|| {
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
                "wikipedia_summary": true,
                "wikipedia_url": true
            }
        }
    )
});

/// Query parameters for the taxa autocomplete endpoint.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct TaxaAutocompleteQuery {
    pub q: String,
}

/// Paginated response wrapper for taxa results.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ResultsTaxa {
    pub total_results: i32,
    pub page: i32,
    pub per_page: i32,
    pub results: Vec<Taxon>,
}

/// A taxon record from the iNaturalist API v2.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Taxon {
    /// Unique auto-increment integer identifier.
    pub id: i32,
    pub uuid: Option<String>,
    pub ancestors: Option<Vec<Self>>,
    pub ancestor_ids: Option<Vec<i32>>,
    pub ancestry: Option<String>,
    pub atlas_id: Option<i32>,
    pub children: Option<Vec<Self>>,
    pub complete_rank: Option<String>,
    pub complete_species_count: Option<i32>,
    pub conservation_status: Option<ConservationStatus>,
    pub conservation_statuses: Option<Vec<ConservationStatus>>,
    pub created_at: Option<String>,
    pub current_synonymous_taxon_ids: Option<Vec<i32>>,
    pub default_photo: Option<Photo>,
    #[serde(default)]
    pub endemic: bool,
    #[serde(default)]
    pub extinct: bool,
    pub flag_counts: Option<FlagCounts>,
    pub iconic_taxon_id: Option<i32>,
    pub iconic_taxon_name: Option<String>,
    #[serde(default)]
    pub introduced: bool,
    #[serde(default)]
    pub is_active: bool,
    pub listed_taxa: Option<Vec<ListedTaxon>>,
    pub listed_taxa_count: Option<i32>,
    pub matched_term: Option<String>,
    pub min_species_ancestry: Option<String>,
    pub min_species_taxon_id: Option<i32>,
    pub name: Option<String>,
    #[serde(default)]
    pub native: bool,
    pub observations_count: Option<i32>,
    pub parent_id: Option<i32>,
    /// Whether or not photos for this taxon can be edited.
    #[serde(default)]
    pub photos_locked: bool,
    pub preferred_common_name: Option<String>,
    pub rank: Option<String>,
    pub rank_level: Option<f32>,
    pub statuses: Option<Vec<String>>,
    pub taxon_changes_count: Option<i32>,
    #[serde(default)]
    pub taxon_photos: Vec<TaxonPhoto>,
    pub taxon_schemes_count: Option<i32>,
    #[serde(default)]
    pub threatened: bool,
    pub universal_search_rank: Option<i32>,
    pub wikipedia_summary: Option<String>,
    pub wikipedia_url: Option<String>,
}

/// IUCN conservation status for a taxon.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConservationStatus {
    pub authority: Option<String>,
    pub description: Option<String>,
    pub geoprivacy: Option<String>,
    pub iucn: Option<i32>,
    pub iucn_status: Option<String>,
    pub iucn_status_code: Option<String>,
    pub place: Option<Place>,
    pub place_id: Option<i32>,
    pub source_id: Option<i32>,
    pub user_id: Option<i32>,
    pub status: Option<String>,
    pub status_name: Option<String>,
    pub url: Option<String>,
}

/// A photo associated with a taxon.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Photo {
    pub id: i32,
    pub attribution: Option<String>,
    pub flags: Option<Vec<Flag>>,
    pub large_url: Option<String>,
    pub license_code: Option<String>,
    pub medium_url: Option<String>,
    pub native_page_url: Option<String>,
    pub native_photo_id: Option<String>,
    pub original_dimensions: Option<OriginalDimensions>,
    pub original_url: Option<String>,
    pub small_url: Option<String>,
    pub square_url: Option<String>,
    pub r#type: Option<String>,
    pub url: Option<String>,
}

/// Count of resolved/unresolved flags on a taxon.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FlagCounts {
    pub resolved: Option<i32>,
    pub unresolved: Option<i32>,
}

/// A taxon listed in a specific place or checklist.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListedTaxon {
    pub id: i32,
    pub establishment_means: Option<String>,
    pub list: Option<List>,
    pub taxon: Option<Taxon>,
    pub taxon_id: Option<i32>,
    pub place: Option<Place>,
}

/// A flag (moderation annotation) on a record.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Flag {
    pub id: i32,
    pub comment: Option<String>,
    pub created_at: Option<String>,
    pub flag: Option<String>,
    #[serde(default)]
    pub resolved: bool,
    pub resolver_id: Option<i32>,
    pub updated_at: Option<String>,
    pub user: Option<User>,
    pub user_id: Option<i32>,
}

/// A photo linked to a taxon with its parent taxon reference.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TaxonPhoto {
    pub taxon: Option<Taxon>,
    pub taxon_id: Option<i32>,
    pub photo: Option<Photo>,
}

/// A geographic place from the iNaturalist taxonomy.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Place {
    pub id: i32,
    pub admin_level: Option<i32>,
    pub ancestor_place_ids: Option<Vec<i32>>,
    pub bbox_area: Option<i32>,
    pub display_name: Option<String>,
    pub geometry_geojson: Option<GeoJson>,
    pub name: String,
    pub place_type: Option<i32>,
    pub uuid: Option<String>,
}

/// Simplified GeoJSON geometry (point coordinates).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GeoJson {
    pub coordinates: Option<(f64, f64)>,
    pub r#type: Option<String>,
}

/// An iNaturalist user profile.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct User {
    pub id: i32,
    pub uuid: Option<String>,
    pub activity_count: Option<i32>,
    pub created_at: Option<String>,
    pub icon: Option<String>,
    pub icon_url: Option<String>,
    pub identifications_count: Option<i32>,
    pub journal_posts_count: Option<i32>,
    pub login: Option<String>,
    pub login_autocomplete: Option<String>,
    pub login_exact: Option<String>,
    pub name: Option<String>,
    pub name_autocomplete: Option<String>,
    pub observations_count: Option<i32>,
    pub orcid: Option<String>,
    pub roles: Option<Vec<String>>,
    pub site_id: Option<i32>,
    #[serde(default)]
    pub spam: bool,
    pub species_count: Option<i32>,
    #[serde(default)]
    pub suspended: bool,
    pub universal_search_rank: Option<i32>,
}

/// Pixel dimensions of an original photo.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OriginalDimensions {
    pub width: Option<i32>,
    pub height: Option<i32>,
}

/// A checklist or list that a taxon belongs to.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct List {
    pub id: Option<i32>,
    pub title: Option<String>,
}
