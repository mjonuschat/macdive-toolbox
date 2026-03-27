use std::convert::TryInto;
use std::str::FromStr;

use sqlx::{Pool, Sqlite};
use uuid::Uuid;

pub use macdive_toolbox_core::domain::*;

use crate::errors::ConversionError;

pub type ConnectionPool = Pool<Sqlite>;

impl TryInto<DiveSite> for crate::macdive::models::DiveSite {
    type Error = ConversionError;

    fn try_into(self) -> Result<DiveSite, Self::Error> {
        let country = self
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
            uuid: self
                .uuid
                .ok_or(ConversionError::MissingUuid)
                .and_then(|v| {
                    Uuid::parse_str(&v.to_lowercase()).map_err(ConversionError::InvalidUuid)
                })?,
            country: self.country.ok_or(ConversionError::MissingCountry)?,
            iso_country_code: country.alpha2.to_string(),
            state: None,
            region: None,
            locality: None,
            name: self.name.ok_or(ConversionError::MissingName)?,
            latitude: self.latitude.ok_or(ConversionError::MissingLatitude)?,
            longitude: self.longitude.ok_or(ConversionError::MissingLongitude)?,
            altitude: 0.0,
            body_of_water: None,
            site_id: self.id,
        })
    }
}

/// Convert a SeaORM dive site entity into the domain `DiveSite` type.
///
/// This mirrors the `TryInto<DiveSite>` implementation for the sqlx-based
/// `macdive::models::DiveSite`, allowing the new `DatabaseManager` code path
/// to produce the same domain objects.
///
/// Implemented as a standalone function rather than `TryFrom` because both
/// `entity::dive_site::Model` and `DiveSite` are defined in external crates
/// (orphan rule). This will be cleaned up when the sqlx path is removed.
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
        body_of_water: None,
        site_id: model.id,
    })
}
