use crate::error::Result;
use std::path::Path;

/// Create a directory and all parent directories if they do not exist.
///
/// If the path already exists and is a directory, this is a no-op.
/// If the path exists but is not a directory, an `Io` error is returned.
///
/// # Errors
///
/// Returns `Error::Io` if directory creation fails.
pub fn create_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}
