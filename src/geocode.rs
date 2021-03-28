use crate::errors::GeocodingError;
use crate::types::DiveSite;

use std::convert::TryInto;

use google_maps::{ClientSettings, LatLng, PlaceType};

pub fn geocode_site(site: DiveSite, key: &str) -> Result<DiveSite, GeocodingError> {
    let mut client = ClientSettings::new(key);
    let latlng: LatLng = site.clone().try_into()?;

    let location = client
        .reverse_geocoding(latlng)
        // .with_result_type(PlaceType::PlusCode)
        .with_result_types(&[PlaceType::PlusCode, PlaceType::Country])
        .execute()
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
            // County
            if component
                .types
                .contains(&PlaceType::AdministrativeAreaLevel2)
            {
                geocoded_site.county = component
                    .long_name
                    .trim()
                    .strip_suffix("County")
                    .map(|v| v.trim().to_string());
                continue;
            }
            // County
            if component.types.contains(&PlaceType::Locality) {
                geocoded_site.locality = Some(component.short_name);
                continue;
            }
        }
    }

    Ok(geocoded_site)
}
