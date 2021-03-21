use std::convert::{TryFrom, TryInto};
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("The MacDive dive site is missing the unique identifier")]
    MissingUuid,
    #[error("The MacDive dive site is using an invalid UUID: `{0}`")]
    InvalidUuid(#[from] uuid::Error),
    #[error("The MacDive dive site is missing country information")]
    MissingCountry,
    #[error("The MacDive dive site is using an unknown country name")]
    UnknownCountry(String),
    #[error("The MacDive dive site is missing latitude information")]
    MissingLatitude,
    #[error("The MacDive dive site is missing longitude information")]
    MissingLongitude,
    #[error("The MacDive dive site is missing a name")]
    MissingName,
}

#[derive(Debug, Clone)]
pub enum WaterType {
    Fresh,
    Salt,
    EN13319,
    Custom(f32),
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
    /// The name of the city of the location where the image was created
    ///
    /// If there is no city, use the Sublocation field alone to specify where the
    /// image was created.
    pub city: Option<String>,
    /// The name of the sublocation of the location where the image was created.
    ///
    /// This sublocation name should be filled with the common name of the dive site.
    pub location: String,
    /// Latitude of a WGS84 based position of this Location
    pub latitude: f32,
    /// Longitude of a WGS84 based position of this Location
    pub longitude: f32,
    /// Altitude in meters of a WGS84 based position of this Location
    pub altitude: f32,
    /// The name of the bod of water where the image was created.
    pub body_of_water: Option<String>,
    /// The freeform name of the body of water, e.g. Pacific Ocean
    pub water_type: WaterType,
    /// MacDive Primary ID
    pub site_id: i32,
}

impl TryInto<DiveSite> for crate::macdive::models::DiveSite {
    type Error = ConversionError;

    fn try_into(self) -> Result<DiveSite, Self::Error> {
        let country = self
            .country
            .ok_or(ConversionError::MissingCountry)
            .and_then(|v| celes::Country::from_str(&v).map_err(ConversionError::UnknownCountry))?;

        Ok(DiveSite {
            uuid: self
                .uuid
                .ok_or(ConversionError::MissingUuid)
                .and_then(|v| Uuid::parse_str(&v).map_err(ConversionError::InvalidUuid))?,
            country: country.long_name,
            iso_country_code: country.alpha2,
            state: None,
            city: None,
            location: self.name.ok_or(ConversionError::MissingName)?,
            latitude: self.latitude.ok_or(ConversionError::MissingLatitude)?,
            longitude: self.latitude.ok_or(ConversionError::MissingLongitude)?,
            altitude: 0.0,
            body_of_water: None,
            water_type: WaterType::Salt,
            site_id: self.id,
        })
    }
}
