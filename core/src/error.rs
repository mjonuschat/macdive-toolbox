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
}

/// Convenience alias used throughout the core crate.
pub type Result<T> = std::result::Result<T, Error>;
