pub(crate) mod models;
// mod schema;
mod types;

use std::path::Path;
use thiserror::Error;

use models::DiveSite;
use sqlx::{Pool, Sqlite, SqlitePool};

type ConnectionPool = Pool<Sqlite>;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Invalid path to MacDive database")]
    InvalidPath,
    #[error("Error querying MacDive database: `{0}`")]
    Query(#[from] sqlx::Error),
}

#[derive(Error, Debug)]
pub enum MacDiveError {
    #[error("Error interacting with MacDive database: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

pub(crate) async fn establish_connection(path: &Path) -> Result<ConnectionPool, DatabaseError> {
    let database_url = path.to_str().ok_or(DatabaseError::InvalidPath)?;
    let pool = SqlitePool::connect(database_url).await;

    Ok(pool?)
}

pub async fn sites(connection: &ConnectionPool) -> Result<Vec<DiveSite>, MacDiveError> {
    let results = sqlx::query_as!(
        DiveSite,
        r#"
        SELECT 
            Z_PK AS id,
            Z_ENT AS ent,
            Z_OPT AS opt,
            ZALTITUDE AS altitude,
            ZGPSLAT AS latitude,
            ZGPSLON AS longitude,
            CAST(ZMODIFIED AS FLOAT) AS "modified_at: _",
            ZBODYOFWATER AS body_of_water,
            ZCOUNTRY AS country,
            ZDIFFICULTY AS difficulty,
            ZDIVELOGUUID AS divelog_uuid,
            ZFLAG AS flag,
            ZIMAGE AS image,
            ZLASTDIVELOGIMAGEHASH AS last_divelog_image_hash,
            ZLOCATION AS location,
            ZNAME AS name,
            ZNOTES AS notes,
            ZUUID AS uuid,
            ZWATERTYPE AS water_type,
            ZZOOM AS zoom
        FROM ZDIVESITE 
        WHERE 
            latitude IS NOT NULL 
            AND longitude IS NOT NULL
        "#
    )
    .fetch_all(connection)
    .await?;

    Ok(results)
}
