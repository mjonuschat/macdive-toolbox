//! Google Maps reverse geocoding service and location override matching.
//!
//! This module provides two public functions:
//!
//! - [`apply_overrides`] – applies user-defined polygon-based location overrides
//!   to a [`DiveSite`], replacing country/state/region/locality fields when the
//!   site's GPS coordinates fall within a configured polygon.
//! - [`geocode_site`] – calls the Google Maps Geocoding API to reverse-geocode a
//!   [`DiveSite`] and fill in country, state, region, and locality fields.

use std::convert::TryInto;

use geo::{Coord, contains::Contains};
use google_maps::{ClientSettings, LatLng, PlaceType};

use crate::domain::{DiveSite, LocationOverride};
use crate::error::{Error, Result};

/// Find the first location override whose polygon contains the given GPS point.
///
/// Coordinates use the geographic convention: `x` maps to longitude and `y`
/// maps to latitude, matching the `geo` crate's `Coord` layout.
///
/// # Arguments
///
/// * `latitude` – WGS84 latitude in decimal degrees.
/// * `longitude` – WGS84 longitude in decimal degrees.
/// * `overrides` – Slice of user-configured location overrides to search.
///
/// # Returns
///
/// A reference to the first matching [`LocationOverride`], or `None` if no
/// polygon contains the given point.
fn find_override(
    latitude: f64,
    longitude: f64,
    overrides: &[LocationOverride],
) -> Option<&LocationOverride> {
    overrides.iter().find(|location| {
        location.polygon().contains(&Coord {
            x: longitude,
            y: latitude,
        })
    })
}

/// Apply user-defined location overrides to a dive site.
///
/// If the site's GPS coordinates fall within one of the configured polygons,
/// the matching override's country, ISO country code, state, region, and
/// locality fields are copied onto the site (only fields that are `Some` in
/// the override are applied).
///
/// # Arguments
///
/// * `site` – The [`DiveSite`] to update. Consumed and returned by value.
/// * `overrides` – Slice of polygon-based location overrides to check against.
///
/// # Returns
///
/// The (potentially modified) site. Returns `Err` only if a future validation
/// step is added; currently always succeeds.
///
/// # Examples
///
/// ```
/// use macdive_toolbox_core::domain::{DiveSite, LocationOverride};
/// use macdive_toolbox_core::services::geocoding::apply_overrides;
/// use uuid::Uuid;
///
/// let site = DiveSite {
///     uuid: Uuid::new_v4(),
///     country: String::from("Unknown"),
///     iso_country_code: String::from("XX"),
///     state: None,
///     region: None,
///     locality: None,
///     name: String::from("Test Site"),
///     latitude: 12.0,
///     longitude: 34.0,
///     altitude: 0.0,
///     body_of_water: None,
///     site_id: 1,
/// };
///
/// let result = apply_overrides(site, &[]).unwrap();
/// assert_eq!(result.country, "Unknown");
/// ```
pub fn apply_overrides(mut site: DiveSite, overrides: &[LocationOverride]) -> Result<DiveSite> {
    if let Some(loc) = find_override(site.latitude, site.longitude, overrides) {
        if let Some(country) = &loc.country {
            site.country = country.to_owned();
        }
        if let Some(code) = &loc.iso_country_code {
            site.iso_country_code = code.to_owned();
        }
        if let Some(state) = &loc.state {
            site.state = Some(state.to_owned());
        }
        if let Some(region) = &loc.region {
            site.region = Some(region.to_owned());
        }
        if let Some(locality) = &loc.locality {
            site.locality = Some(locality.to_owned());
        }
    }

    Ok(site)
}

/// Reverse-geocode a dive site using the Google Maps Geocoding API.
///
/// Queries the Google Maps API with the site's latitude and longitude and
/// fills in country, ISO country code, state, region, and locality fields
/// from the returned address components.
///
/// # Arguments
///
/// * `site` – The [`DiveSite`] to geocode. Consumed and returned by value.
/// * `key` – Google Maps API key.
///
/// # Errors
///
/// Returns [`Error::GeocodingFailed`] if the API key is invalid, the HTTP
/// request fails, or the response cannot be parsed.
/// Returns [`Error::InvalidLatitude`] or [`Error::InvalidLongitude`] if the
/// site's coordinates cannot be converted to a `LatLng`.
pub async fn geocode_site(site: DiveSite, key: &str) -> Result<DiveSite> {
    let client = ClientSettings::try_new(key).map_err(|_e| Error::GeocodingFailed)?;
    let latlng: LatLng = site.clone().try_into()?;

    let location = client
        .reverse_geocoding(latlng)
        .with_result_types([PlaceType::PlusCode, PlaceType::Country])
        .execute()
        .await
        .map_err(|_e| Error::GeocodingFailed)?;

    let mut geocoded_site = DiveSite { ..site };
    for result in location.results {
        for component in result.address_components {
            // Country name and ISO code
            if component.types.contains(&PlaceType::Country) {
                geocoded_site.iso_country_code = component.short_name;
                geocoded_site.country = component.long_name;
                continue;
            }
            // State or province (administrative level 1)
            if component
                .types
                .contains(&PlaceType::AdministrativeAreaLevel1)
            {
                geocoded_site.state = Some(component.long_name);
                continue;
            }
            // County or sub-region (administrative level 2); strip trailing "County"
            if component
                .types
                .contains(&PlaceType::AdministrativeAreaLevel2)
            {
                geocoded_site.region = component
                    .long_name
                    .trim()
                    .strip_suffix("County")
                    .map(|v| v.trim().to_string());
                continue;
            }
            // City / locality
            if component.types.contains(&PlaceType::Locality) {
                geocoded_site.locality = Some(component.short_name);
                continue;
            }
        }
    }

    Ok(geocoded_site)
}
