use crate::error::Result;
use ::entity::prelude::*;
use sea_orm::{ColumnTrait, DbConn, EntityTrait, QueryFilter};

/// Fetch all dive sites that have GPS coordinates.
///
/// Returns only rows where both latitude and longitude are non-null,
/// matching the original sqlx query behaviour.
///
/// # Arguments
///
/// * `db` - A read-only connection to the MacDive SQLite database.
///
/// # Errors
///
/// Returns [`crate::error::Error::Database`] if the query fails.
pub async fn sites(db: &DbConn) -> Result<Vec<::entity::dive_site::Model>> {
    Ok(DiveSite::find()
        .filter(::entity::dive_site::Column::Latitude.is_not_null())
        .filter(::entity::dive_site::Column::Longitude.is_not_null())
        .all(db)
        .await?)
}

/// Fetch all critters from the MacDive database.
///
/// # Arguments
///
/// * `db` - A read-only connection to the MacDive SQLite database.
///
/// # Errors
///
/// Returns [`crate::error::Error::Database`] if the query fails.
pub async fn critters(db: &DbConn) -> Result<Vec<::entity::critter::Model>> {
    Ok(Critter::find().all(db).await?)
}

/// Fetch all critter categories from the MacDive database.
///
/// # Arguments
///
/// * `db` - A read-only connection to the MacDive SQLite database.
///
/// # Errors
///
/// Returns [`crate::error::Error::Database`] if the query fails.
pub async fn critter_categories(db: &DbConn) -> Result<Vec<::entity::critter_category::Model>> {
    Ok(CritterCategory::find().all(db).await?)
}
