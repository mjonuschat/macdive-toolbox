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

#[cfg(test)]
mod tests {
    use google_maps::LatLng;
    use rust_decimal::Decimal;

    use super::*;

    #[test]
    fn test_dms_null_island() {
        // Null Island, Intersection of Prime Meridian and Equator
        let latlng = LatLng {
            lat: Decimal::new(0, 0),
            lng: Decimal::new(0, 0),
        };

        assert_eq!("0°0'0\" 0°0'0\"", latlng.to_dms().unwrap());
    }

    #[test]
    fn test_dms_nw() {
        // Golden Gate Park, San Francisco, CA, USA
        let latlng = LatLng {
            lat: Decimal::new(37769722, 6),
            lng: Decimal::new(-122476944, 6),
        };

        assert_eq!(
            "37°46'10.9992\" N 122°28'36.9984\" W",
            latlng.to_dms().unwrap()
        );
    }

    #[test]
    fn test_dms_ne() {
        // The Moscow Kremlin, Moscow, Russia
        let latlng = LatLng {
            lat: Decimal::new(55752460, 6),
            lng: Decimal::new(37617779, 6),
        };

        assert_eq!("55°45'8.856\" N 37°37'4.0044\" E", latlng.to_dms().unwrap());
    }

    #[test]
    fn test_dms_sw() {
        // Maracanã Stadium, Rio de Janeiro, Brazil
        let latlng = LatLng {
            lat: Decimal::new(-22912376, 6),
            lng: Decimal::new(-43230320, 6),
        };

        assert_eq!(
            "22°54'44.5536\" S 43°13'49.152\" W",
            latlng.to_dms().unwrap()
        );
    }

    #[test]
    fn test_dms_se() {
        // Sydney Opera House, Sydney, Australia
        let latlng = LatLng {
            lat: Decimal::new(-33856159, 6),
            lng: Decimal::new(151215256, 6),
        };

        assert_eq!(
            "33°51'22.1724\" S 151°12'54.9216\" E",
            latlng.to_dms().unwrap()
        );
    }
}
