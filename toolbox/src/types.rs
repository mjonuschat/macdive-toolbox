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
