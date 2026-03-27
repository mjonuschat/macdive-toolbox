//! MacDive dive site entity (read-only).

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "ZDIVESITE")]
pub struct Model {
    #[sea_orm(primary_key, column_name = "Z_PK")]
    pub id: i64,
    #[sea_orm(column_name = "Z_ENT")]
    pub ent: Option<i64>,
    #[sea_orm(column_name = "Z_OPT")]
    pub opt: Option<i64>,
    #[sea_orm(column_name = "ZALTITUDE")]
    pub altitude: Option<f64>,
    #[sea_orm(column_name = "ZGPSLAT")]
    pub latitude: Option<f64>,
    #[sea_orm(column_name = "ZGPSLON")]
    pub longitude: Option<f64>,
    /// Seconds since 2001-01-01 (Apple NSDate epoch). Convert to `chrono::DateTime` in the domain layer.
    #[sea_orm(column_name = "ZMODIFIED")]
    pub modified_at: Option<f64>,
    #[sea_orm(column_name = "ZBODYOFWATER")]
    pub body_of_water: Option<String>,
    #[sea_orm(column_name = "ZCOUNTRY")]
    pub country: Option<String>,
    #[sea_orm(column_name = "ZDIFFICULTY")]
    pub difficulty: Option<String>,
    #[sea_orm(column_name = "ZDIVELOGUUID")]
    pub divelog_uuid: Option<String>,
    #[sea_orm(column_name = "ZFLAG")]
    pub flag: Option<String>,
    #[sea_orm(column_name = "ZIMAGE")]
    pub image: Option<String>,
    #[sea_orm(column_name = "ZLASTDIVELOGIMAGEHASH")]
    pub last_divelog_image_hash: Option<String>,
    #[sea_orm(column_name = "ZLOCATION")]
    pub location: Option<String>,
    #[sea_orm(column_name = "ZNAME")]
    pub name: Option<String>,
    #[sea_orm(column_name = "ZNOTES")]
    pub notes: Option<String>,
    #[sea_orm(column_name = "ZUUID")]
    pub uuid: Option<String>,
    #[sea_orm(column_name = "ZWATERTYPE")]
    pub water_type: Option<String>,
    #[sea_orm(column_name = "ZZOOM")]
    pub zoom: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
