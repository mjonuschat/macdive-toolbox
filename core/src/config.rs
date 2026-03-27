use std::path::Path;

use crate::domain::ApplicationConfig;
use crate::error::{Error, Result};

/// Load application configuration from a YAML file.
///
/// # Parameters
///
/// - `path`: Path to a YAML file containing serialized [`ApplicationConfig`] data.
///
/// # Returns
///
/// The deserialized [`ApplicationConfig`] on success.
///
/// # Errors
///
/// Returns [`Error::Config`] if the file cannot be read or the YAML is invalid.
pub fn load_config(path: &Path) -> Result<ApplicationConfig> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::Config(format!("could not read {}: {e}", path.display())))?;
    serde_saphyr::from_str(&content)
        .map_err(|e| Error::Config(format!("invalid config {}: {e}", path.display())))
}
