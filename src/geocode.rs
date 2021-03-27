use crate::types::DiveSite;
use google_maps::{ClientSettings, LatLng, PlaceType};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GeocodingError {
    #[error("Error talking to Google Maps API")]
    GoogleMaps,
    #[error("Missing or invalid latitude")]
    InvalidLatitude,
    #[error("Missing or invalid longitude")]
    InvalidLongitude,
}

pub fn geocode_site(site: DiveSite, key: &str) -> Result<DiveSite, GeocodingError> {
    let mut client = ClientSettings::new(key);
    let latlng = LatLng::try_from(
        Decimal::from_f32(site.latitude).ok_or(GeocodingError::InvalidLatitude)?,
        Decimal::from_f32(site.longitude).ok_or(GeocodingError::InvalidLongitude)?,
    )
    .map_err(|_e| GeocodingError::GoogleMaps)?;

    let location = client
        .reverse_geocoding(latlng.clone())
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
                geocoded_site.county = Some(component.long_name);
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
