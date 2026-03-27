/// Unified error type for the core library.
///
/// All public functions in core return `core::Result<T>`.
/// The toolbox CLI wraps these with anyhow for context.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid latitude")]
    InvalidLatitude,
    #[error("invalid longitude")]
    InvalidLongitude,
    #[error("invalid GPS coordinates")]
    InvalidGps,
    #[error("geocoding API failed")]
    GeocodingFailed,
    #[error("configuration error: {0}")]
    Config(String),
    #[error("species name parse error: {0}")]
    ParseError(String),
    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),
    #[error("iNaturalist API error: {0}")]
    INaturalist(String),
    #[error("species verification failed: {0}")]
    GlobalNames(String),
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("template rendering failed: {0}")]
    Template(String),
    #[error("invalid UUID in Lightroom template")]
    InvalidUuid(#[from] uuid::Error),
    #[error("error parsing existing Lightroom template")]
    LightroomParsing,
    #[error("MTP device error: {0}")]
    Mtp(String),
    #[error("MTP storage error: folder not found: {0}")]
    MtpFolderNotFound(String),
}

/// Convenience alias used throughout the core crate.
pub type Result<T> = std::result::Result<T, Error>;
