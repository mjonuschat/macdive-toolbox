/// Unified error type for the core library.
///
/// All public functions in core return `core::Result<T>`.
/// The toolbox CLI wraps these with anyhow for context.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convenience alias used throughout the core crate.
pub type Result<T> = std::result::Result<T, Error>;
