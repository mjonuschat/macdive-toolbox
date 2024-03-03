use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("The MacDive dive site is missing the unique identifier")]
    MissingUuid,
    #[error("The MacDive dive site is using an invalid UUID: `{0}`")]
    InvalidUuid(#[from] uuid::Error),
    #[error("The MacDive dive site is missing country information")]
    MissingCountry,
    #[error("The MacDive dive site is using an unknown country name: `{0}`")]
    UnknownCountry(String),
    #[error("The MacDive dive site is missing latitude information")]
    MissingLatitude,
    #[error("The MacDive dive site is missing longitude information")]
    MissingLongitude,
    #[error("The MacDive dive site is missing a name")]
    MissingName,
    #[error("Error reverse geocoding the dive site")]
    GeocodingError(#[from] GeocodingError),
}

#[derive(Error, Debug)]
pub enum GeocodingError {
    #[error("Error talking to Google Maps API")]
    GoogleMaps,
    #[error("Missing or invalid latitude")]
    InvalidLatitude,
    #[error("Missing or invalid longitude")]
    InvalidLongitude,
    #[error("Invalid GPS coordinates for dive site")]
    InvalidGps,
}

#[derive(Error, Debug)]
pub enum LightroomTemplateError {
    #[error("Invalid UUID in Lightroom Template")]
    InvalidUuid(#[from] uuid::Error),
    #[error("Invalid reading/writing Lightroom Template")]
    IoError(#[from] std::io::Error),
    #[error("Error rendering the Lightroom Template")]
    Rendering(#[from] askama::Error),
    #[error("Error parsing existing Lightroom Template")]
    Parsing,
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Invalid path to MacDive database")]
    InvalidPath,
    #[error("Error querying MacDive database: `{0}`")]
    Query(#[from] sqlx::Error),
}

#[derive(Error, Debug)]
pub enum PathError {
    #[error("Path `{0}` could not be resolved")]
    Canonicalize(#[from] std::io::Error),
    #[error("Path to user's data directory could not be detected")]
    DataDir,
    #[error("File or directory `{0}` is not accessible")]
    Inaccessible(String),
}

#[derive(Debug, Error)]
pub enum MtpStorageError {
    #[error("Folder {0} could not be found")]
    FolderNotFound(String),
}

#[derive(Error, Debug)]
pub enum MtpDeviceError {
    #[error("No MTP device found on USB bus")]
    NoDeviceAttached,
    #[error("No device matching selection criteria found")]
    DeviceNotFound,
    #[error("FFI error: {0}")]
    LibMtpError(#[from] libmtp_rs::error::Error),
}
