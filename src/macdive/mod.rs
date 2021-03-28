pub(crate) mod models;
mod schema;
mod types;

use diesel::prelude::*;
use std::path::Path;
use thiserror::Error;

use models::DiveSite;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Invalid path to MacDive database")]
    InvalidPath,
    #[error("Error establishing connection to MacDive Database: `{0}`")]
    Connection(#[from] ConnectionError),
    #[error("Error querying MacDive database: `{0}`")]
    Query(#[from] diesel::result::Error),
}

#[derive(Error, Debug)]
pub enum MacDiveError {
    #[error("Error interacting with MacDive database: {0}")]
    DatabaseError(#[from] DatabaseError),
}

pub(crate) fn establish_connection(path: &Path) -> Result<SqliteConnection, DatabaseError> {
    let database_url = path.to_str().ok_or(DatabaseError::InvalidPath)?;
    SqliteConnection::establish(database_url).map_err(DatabaseError::Connection)
}

pub fn sites(connection: &SqliteConnection) -> Result<Vec<DiveSite>, MacDiveError> {
    // let conn = establish_connection(database).map_err(MacDiveError::DatabaseError)?;
    use schema::divesites::dsl::*;

    let results = divesites
        .filter(latitude.is_not_null())
        .filter(longitude.is_not_null())
        .load::<DiveSite>(connection)
        .map_err(DatabaseError::Query)?;

    Ok(results)
}
