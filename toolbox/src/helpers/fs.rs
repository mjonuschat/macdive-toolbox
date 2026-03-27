use crate::errors::PathError;
use std::path::Path;

// Re-export MTP-specific filesystem utilities from core.
pub use macdive_toolbox_core::services::mtp::read_existing_activities;

/// Create a directory and all parent directories if they do not exist.
///
/// Delegates to `macdive_toolbox_core::util::fs::create_dir` and maps errors
/// into the toolbox `PathError` type.
pub(crate) fn create_dir(path: &Path) -> Result<(), PathError> {
    macdive_toolbox_core::util::fs::create_dir(path)
        .map_err(|_| PathError::Inaccessible(path.to_string_lossy().to_string()))
}
