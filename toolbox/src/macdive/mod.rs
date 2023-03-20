use std::path::Path;

use sqlx::SqlitePool;
use thiserror::Error;

use models::{Critter, DiveSite};

use crate::errors::DatabaseError;
use crate::macdive::models::{CritterCategory, CritterUpdate};
use crate::types::ConnectionPool;

pub(crate) mod models;
mod types;

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

pub async fn critters(connection: &ConnectionPool) -> Result<Vec<Critter>, MacDiveError> {
    let results = sqlx::query_as!(
        Critter,
        r#"
        SELECT 
            Z_PK AS id,
            Z_ENT AS ent,
            Z_OPT AS opt,
            ZRELATIONSHIPCRITTERTOCRITTERCATEGORY AS category,
            ZSIZE AS size,
            ZIMAGE AS image,
            ZNAME AS name,
            ZNOTES AS notes,
            ZSPECIES AS species,
            ZUUID AS "uuid: _"
        FROM ZCRITTER
        "#
    )
    .fetch_all(connection)
    .await?;

    Ok(results)
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

enum SqlParam {
    Text(String),
    Integer(i64),
}

#[allow(dead_code)]
pub async fn update_critter(
    changeset: &CritterUpdate,
    connection: &ConnectionPool,
) -> Result<(), MacDiveError> {
    let mut sql = String::from("UPDATE ZCRITTER SET Z_PK=?");
    let mut params: Vec<SqlParam> = Vec::new();

    if let Some(name) = &changeset.common_name {
        let name = format!("Review: {}", name);

        sql.push_str(", ZNAME=?");
        params.push(SqlParam::Text(name));
    }

    if let Some(name) = &changeset.scientific_name {
        let name = format!("Review: {}", name);

        sql.push_str(", ZSPECIES=?");
        params.push(SqlParam::Text(name));
    }

    if let Some(category) = &changeset.category {
        sql.push_str(", ZRELATIONSHIPCRITTERTOCRITTERCATEGORY=?");
        params.push(SqlParam::Integer(*category));
    }

    sql.push_str(" WHERE Z_PK=?");

    let mut query = sqlx::query(&sql);
    query = query.bind(changeset.id);
    for p in params {
        query = match p {
            SqlParam::Integer(p) => query.bind(p),
            SqlParam::Text(p) => query.bind(p),
        }
    }
    query = query.bind(changeset.id);

    query.execute(connection).await?;

    Ok(())
}

#[allow(dead_code)]
pub async fn update_critter_category(
    id: i64,
    name: &str,
    connection: &ConnectionPool,
) -> Result<(), MacDiveError> {
    let name = format!("UPDATED: {}", name);
    sqlx::query!(
        r#"UPDATE ZCRITTERCATEGORY SET ZNAME=? WHERE Z_PK=?"#,
        name,
        id
    )
    .execute(connection)
    .await?;

    Ok(())
}

pub async fn critter_categories(
    connection: &ConnectionPool,
) -> Result<Vec<CritterCategory>, MacDiveError> {
    let results = sqlx::query_as!(
        CritterCategory,
        r#"
        SELECT 
            Z_PK AS id,
            Z_ENT AS ent,
            Z_OPT AS opt,
            ZIMAGE AS image,
            ZNAME AS name,
            ZUUID AS "uuid: _"
        FROM ZCRITTERCATEGORY
        "#
    )
    .fetch_all(connection)
    .await?;

    Ok(results)
}
