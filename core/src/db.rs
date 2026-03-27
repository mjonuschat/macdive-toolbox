use crate::error::Result;
use sea_orm::{Database, DbConn};
use std::path::Path;

/// Manages connections to both application databases.
///
/// - `macdive`: read-only connection to the MacDive.sqlite database
/// - `cache`: read-write connection to the app's toolbox.sqlite cache
pub struct DatabaseManager {
    macdive: DbConn,
    cache: DbConn,
}

impl DatabaseManager {
    /// Create a new `DatabaseManager` with connections to both databases.
    ///
    /// # Arguments
    ///
    /// * `macdive_path` - Path to the MacDive.sqlite file (opened read-only)
    /// * `cache_path` - Path to the toolbox.sqlite cache file (created if missing)
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::Io`] if the cache file cannot be created, or
    /// [`crate::error::Error::Database`] if either database connection fails.
    pub async fn new(macdive_path: &Path, cache_path: &Path) -> Result<Self> {
        // Ensure cache directory and file exist before connecting.
        if let Some(parent) = cache_path.parent() {
            crate::util::fs::create_dir(parent)?;
        }
        if !cache_path.exists() {
            std::fs::File::create(cache_path)?;
        }

        let macdive_url = format!("sqlite://{}?mode=ro", macdive_path.display());
        let cache_url = format!("sqlite://{}", cache_path.display());

        let macdive = Database::connect(&macdive_url).await?;
        let cache = Database::connect(&cache_url).await?;

        Ok(Self { macdive, cache })
    }

    /// Returns a read-only connection to the MacDive database.
    pub fn macdive(&self) -> &DbConn {
        &self.macdive
    }

    /// Returns the read-write connection to the application cache database.
    pub fn cache(&self) -> &DbConn {
        &self.cache
    }
}
