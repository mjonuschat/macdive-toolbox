use crate::errors::GeocodingError;
use crate::types::{DiveSite, LocationOverride};

use std::convert::TryInto;

use geo::{contains::Contains, Coord};
use google_maps::{ClientSettings, LatLng, PlaceType};

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

pub fn apply_overrides(
    mut site: DiveSite,
    overrides: &[LocationOverride],
) -> Result<DiveSite, GeocodingError> {
    if let Some(loc) = find_override(site.latitude, site.longitude, overrides) {
        if let Some(country) = &loc.country {
            site.country = country.to_owned()
        }
        if let Some(code) = &loc.iso_country_code {
            site.iso_country_code = code.to_owned()
        }
        if let Some(state) = &loc.state {
            site.state = Some(state.to_owned())
        }
        if let Some(region) = &loc.region {
            site.region = Some(region.to_owned())
        }
        if let Some(locality) = &loc.locality {
            site.locality = Some(locality.to_owned())
        }
    }

    Ok(site)
}
pub async fn geocode_site(site: DiveSite, key: &str) -> Result<DiveSite, GeocodingError> {
    let client = ClientSettings::new(key);
    let latlng: LatLng = site.clone().try_into()?;

    let location = client
        .reverse_geocoding(latlng)
        // .with_result_type(PlaceType::PlusCode)
        .with_result_types(&[PlaceType::PlusCode, PlaceType::Country])
        .execute()
        .await
        .map_err(|_e| GeocodingError::GoogleMaps)?;

    let mut geocoded_site = DiveSite { ..site };
    for result in location.results {
        for component in result.address_components {
            // Country
            if component.types.contains(&PlaceType::Country) {
                geocoded_site.iso_country_code = component.short_name;
                geocoded_site.country = component.long_name;
                continue;
            }
            // State
            if component
                .types
                .contains(&PlaceType::AdministrativeAreaLevel1)
            {
                geocoded_site.state = Some(component.long_name);
                continue;
            }
            // Region
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
            // City
            if component.types.contains(&PlaceType::Locality) {
                geocoded_site.locality = Some(component.short_name);
                continue;
            }
        }
    }

    Ok(geocoded_site)
}
