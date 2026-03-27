use std::str::FromStr;

use macdive_toolbox_core::domain::DiveSite;
use uuid::Uuid;

use crate::errors::ConversionError;

/// Convert a SeaORM dive site entity into the domain `DiveSite` type.
///
/// Maps the raw database model into the validated domain struct, applying
/// country name corrections (e.g. "Netherlands Antilles" -> "Bonaire")
/// and deriving the ISO country code via the `celes` crate.
///
/// Implemented as a standalone function rather than `TryFrom` because both
/// `entity::dive_site::Model` and `DiveSite` are defined in external crates
/// (orphan rule).
pub fn dive_site_from_entity(model: entity::dive_site::Model) -> Result<DiveSite, ConversionError> {
    let country = model
        .country
        .clone()
        .map(|c| match c.as_str() {
            "Netherlands Antilles" => String::from("Bonaire"),
            "Solomon Islands" => String::from("SolomonIslands"),
            _ => c,
        })
        .ok_or(ConversionError::MissingCountry)
        .and_then(|v| {
            celes::Country::from_str(&v)
                .map_err(|_result| ConversionError::UnknownCountry(v.to_string()))
        })?;

    Ok(DiveSite {
        uuid: model
            .uuid
            .ok_or(ConversionError::MissingUuid)
            .and_then(|v| {
                Uuid::parse_str(&v.to_lowercase()).map_err(ConversionError::InvalidUuid)
            })?,
        country: model.country.ok_or(ConversionError::MissingCountry)?,
        iso_country_code: country.alpha2.to_string(),
        state: None,
        region: None,
        locality: None,
        name: model.name.ok_or(ConversionError::MissingName)?,
        latitude: model.latitude.ok_or(ConversionError::MissingLatitude)?,
        longitude: model.longitude.ok_or(ConversionError::MissingLongitude)?,
        altitude: 0.0,
        body_of_water: model.body_of_water,
        site_id: model.id,
    })
}
