use std::cmp::Ordering;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;

use google_maps::LatLng;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use rust_decimal_macros::dec;
use serde_derive::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use crate::errors::{ConversionError, GeocodingError};
use crate::inaturalist::TaxonGroupName;

pub trait DecimalToDms {
    fn to_dms(&self) -> Result<String, GeocodingError>;
}

impl DecimalToDms for LatLng {
    fn to_dms(&self) -> Result<String, GeocodingError> {
        let lat_absolute = self.lat.abs();
        let lat_degrees = lat_absolute.trunc();
        let lat_minutes = lat_absolute.fract() * Decimal::new(60, 0);
        let lat_seconds = lat_minutes.fract() * Decimal::new(60, 0);

        let lat_direction = match self.lat.cmp(&dec!(0.0)) {
            Ordering::Less => " S".to_string(),
            Ordering::Greater => " N".to_string(),
            Ordering::Equal => "".to_string(),
        };

        let lng_absolute = self.lng.abs();
        let lng_degrees = lng_absolute.trunc();
        let lng_minutes = lng_absolute.fract() * Decimal::new(60, 0);
        let lng_seconds = lng_minutes.fract() * Decimal::new(60, 0);

        let lng_direction = match self.lng.cmp(&dec!(0.0)) {
            Ordering::Less => " W".to_string(),
            Ordering::Greater => " E".to_string(),
            Ordering::Equal => "".to_string(),
        };

        Ok(format!(
            r#"{}°{}'{}"{} {}°{}'{}"{}"#,
            lat_degrees,
            lat_minutes.trunc(),
            lat_seconds.normalize(),
            lat_direction,
            lng_degrees,
            lng_minutes.trunc(),
            lng_seconds.normalize(),
            lng_direction
        ))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApplicationConfig {
    pub locations: HashMap<String, LocationOverride>,
    pub critters: CritterConfig,
}

impl ApplicationConfig {
    pub fn locations(&self) -> Vec<LocationOverride> {
        self.locations.values().cloned().collect()
    }
}

type CritterNameSubstitutions = HashMap<String, String>;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CritterConfig {
    pub name_substitutions: CritterNameSubstitutions,
    pub categories: CritterCategoryConfig,
}

impl From<ApplicationConfig> for CritterConfig {
    fn from(config: ApplicationConfig) -> Self {
        config.critters
    }
}

impl From<ApplicationConfig> for CritterCategoryConfig {
    fn from(config: ApplicationConfig) -> Self {
        config.critters.categories
    }
}

impl From<ApplicationConfig> for CritterNameSubstitutions {
    fn from(config: ApplicationConfig) -> Self {
        config.critters.name_substitutions
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CritterCategoryConfig {
    pub group_names: HashMap<String, String>,
    pub preferred_higher_ranks: HashMap<String, Vec<TaxonGroupName>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocationOverride {
    pub area: Vec<(f64, f64)>,
    /// The full name should be expressed as a verbal name and not as a code
    pub country: Option<String>,
    /// ISO country code of the location where the image was created
    pub iso_country_code: Option<String>,
    /// The name of the subregion of a country, either a State or Province where the image was created
    pub state: Option<String>,
    /// The name of the sub-subregion of a country, could be a county or region name where the image was created
    pub region: Option<String>,
    /// The name of the city or area
    pub locality: Option<String>,
}

impl LocationOverride {
    pub fn polygon(&self) -> geo::Polygon<f64> {
        geo::Polygon::new(geo::LineString::from(self.area.clone()), vec![])
    }
}

#[derive(Debug, Clone)]
pub struct DiveSite {
    /// Unique Identifier
    pub uuid: Uuid,
    /// The full name of the country of the location where the image was created
    ///
    // TODO: Try to normalize/guess from country names/aliases
    /// The full name should be expressed as a verbal name and not as a code
    pub country: String,
    /// ISO country code of the location where the image was created
    ///
    /// Either the two- or three-letter code, as defined by ISO-3166
    pub iso_country_code: String,
    /// The name of the subregion of a country, either a State or Province where the image was created
    ///
    /// Since the abbreviation for a State or Province may be unknown consider using the
    /// full spelling of the name.
    pub state: Option<String>,
    /// The name of the sub-subregion of a country, could be a county or region name where the image was created
    pub region: Option<String>,
    /// The name of the city of the location where the image was created
    ///
    /// If there is no city, use the Sublocation field alone to specify where the
    /// image was created.
    pub locality: Option<String>,
    /// The name of the sublocation of the location where the image was created.
    ///
    /// This sublocation name should be filled with the common name of the dive site.
    pub name: String,
    /// Latitude of a WGS84 based position of this Location
    pub latitude: f64,
    /// Longitude of a WGS84 based position of this Location
    pub longitude: f64,
    /// Altitude in meters of a WGS84 based position of this Location
    pub altitude: f32,
    /// The name of the bod of water where the image was created.
    pub body_of_water: Option<String>,
    /// MacDive Primary ID
    pub site_id: i64,
}

impl TryFrom<DiveSite> for LatLng {
    type Error = GeocodingError;

    fn try_from(site: DiveSite) -> Result<Self, Self::Error> {
        let lat = Decimal::from_f64(site.latitude).ok_or(GeocodingError::InvalidLatitude)?;
        let lng = Decimal::from_f64(site.longitude).ok_or(GeocodingError::InvalidLongitude)?;
        Ok(LatLng { lat, lng })
    }
}

impl TryInto<DiveSite> for crate::macdive::models::DiveSite {
    type Error = ConversionError;

    fn try_into(self) -> Result<DiveSite, Self::Error> {
        let country = self
            .country
            .clone()
            .map(|c| {
                if c == *"Netherlands Antilles" {
                    String::from("Bonaire")
                } else {
                    c
                }
            })
            .ok_or(ConversionError::MissingCountry)
            .and_then(|v| celes::Country::from_str(&v).map_err(ConversionError::UnknownCountry))?;

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

pub type ConnectionPool = Pool<Sqlite>;
