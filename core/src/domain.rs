use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use google_maps::LatLng;
use rust_decimal::{Decimal, prelude::FromPrimitive};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Error;

pub const APPLICATION_NAME: &str = "MacDive Toolbox";

/// Convert an Apple NSDate timestamp (seconds since 2001-01-01) to a chrono `DateTime`.
///
/// Apple Core Data stores dates as floating-point seconds since the
/// reference date 2001-01-01T00:00:00Z (the Cocoa/CF time epoch).
///
/// # Arguments
///
/// * `timestamp` - Seconds since 2001-01-01T00:00:00Z as stored in Core Data.
///
/// # Examples
///
/// ```
/// use macdive_toolbox_core::domain::nsdate_to_datetime;
/// let dt = nsdate_to_datetime(0.0);
/// assert_eq!(dt.to_rfc3339(), "2001-01-01T00:00:00+00:00");
/// ```
pub fn nsdate_to_datetime(timestamp: f64) -> DateTime<Utc> {
    // The Apple Core Data epoch starts on 2001-01-01, not the Unix epoch (1970-01-01).
    let epoch = NaiveDate::from_ymd_opt(2001, 1, 1)
        .expect("2001-01-01 is a valid date")
        .and_hms_opt(0, 0, 0)
        .expect("00:00:00 is a valid time")
        .and_utc();
    epoch + TimeDelta::milliseconds((timestamp * 1000.0) as i64)
}

pub trait DecimalToDms {
    fn to_dms(&self) -> Result<String, Error>;
}

impl DecimalToDms for LatLng {
    fn to_dms(&self) -> Result<String, Error> {
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

pub type CritterNameSubstitutions = HashMap<String, String>;

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

#[derive(Clone, Debug, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaxonGroupName {
    Unspecified,
    Custom(String),
    Phylum(String),
    Subphylum(String),
    Class(String),
    Subclass(String),
    Infraclass(String),
    Superorder(String),
    Order(String),
    Suborder(String),
    Infraorder(String),
    Parvorder(String),
    Superfamily(String),
    Family(String),
    Subfamily(String),
    Genus(String),
}

impl PartialEq for TaxonGroupName {
    fn eq(&self, other: &Self) -> bool {
        use TaxonGroupName::*;

        match (self, other) {
            (Unspecified, Unspecified) => true,
            (Custom(lhs), Custom(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Phylum(lhs), Phylum(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Subphylum(lhs), Subphylum(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Class(lhs), Class(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Subclass(lhs), Subclass(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Infraclass(lhs), Infraclass(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Superorder(lhs), Superorder(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Order(lhs), Order(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Suborder(lhs), Suborder(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Infraorder(lhs), Infraorder(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Parvorder(lhs), Parvorder(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Superfamily(lhs), Superfamily(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Family(lhs), Family(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Subfamily(lhs), Subfamily(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (Genus(lhs), Genus(rhs)) => lhs.to_lowercase() == rhs.to_lowercase(),
            (_, _) => false,
        }
    }
}

impl Hash for TaxonGroupName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use TaxonGroupName::*;

        match self {
            Unspecified => "".hash(state),
            Custom(v) => v.hash(state),
            Phylum(v) => v.hash(state),
            Subphylum(v) => v.hash(state),
            Class(v) => v.hash(state),
            Subclass(v) => v.hash(state),
            Infraclass(v) => v.hash(state),
            Superorder(v) => v.hash(state),
            Order(v) => v.hash(state),
            Suborder(v) => v.hash(state),
            Infraorder(v) => v.hash(state),
            Parvorder(v) => v.hash(state),
            Superfamily(v) => v.hash(state),
            Family(v) => v.hash(state),
            Subfamily(v) => v.hash(state),
            Genus(v) => v.hash(state),
        }
    }
}

impl TaxonGroupName {
    fn normalize(name: &str) -> String {
        change_case::title_case(
            name.to_lowercase()
                .trim_start_matches("true")
                .trim_start_matches("false")
                .trim_start_matches("typical")
                .trim_end_matches("and allies")
                .trim()
                .trim_end_matches(','),
        )
    }

    pub fn prefer_higher_common_name(
        &self,
        class: &str,
        overrides: &CritterCategoryConfig,
    ) -> bool {
        overrides
            .preferred_higher_ranks
            .get(class)
            .map(|list| list.contains(self))
            .unwrap_or(false)
    }
}

impl Display for TaxonGroupName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TaxonGroupName::Unspecified => write!(f, "Unknown"),
            TaxonGroupName::Custom(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Phylum(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Subphylum(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Class(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Subclass(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Infraclass(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Superorder(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Order(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Suborder(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Infraorder(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Parvorder(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Superfamily(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Family(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Subfamily(name) => write!(f, "{}", Self::normalize(name)),
            TaxonGroupName::Genus(name) => write!(f, "{}", Self::normalize(name)),
        }
    }
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
    /// The name of the body of water where the image was created.
    pub body_of_water: Option<String>,
    /// MacDive Primary ID
    pub site_id: i64,
}

impl TryFrom<DiveSite> for LatLng {
    type Error = Error;

    fn try_from(site: DiveSite) -> Result<Self, Self::Error> {
        let lat = Decimal::from_f64(site.latitude).ok_or(Error::InvalidLatitude)?;
        let lng = Decimal::from_f64(site.longitude).ok_or(Error::InvalidLongitude)?;
        Ok(LatLng { lat, lng })
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

    #[test]
    fn test_nsdate_epoch() {
        // Timestamp 0.0 must map exactly to the Apple Core Data epoch: 2001-01-01T00:00:00Z.
        let dt = nsdate_to_datetime(0.0);
        assert_eq!(dt.to_rfc3339(), "2001-01-01T00:00:00+00:00");
    }

    #[test]
    fn test_nsdate_known_value() {
        // 2024-03-15T12:00:00Z expressed as seconds since 2001-01-01T00:00:00Z.
        // Days from 2001-01-01 to 2024-03-15: 8474 days * 86400 s + 43200 s (12h)
        let seconds = (8474.0_f64 * 86_400.0) + 43_200.0;
        let dt = nsdate_to_datetime(seconds);
        assert_eq!(dt.to_rfc3339(), "2024-03-15T12:00:00+00:00");
    }
}
