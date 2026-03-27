//! MacDive critter entity (read-only).

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "ZCRITTER")]
pub struct Model {
    #[sea_orm(primary_key, column_name = "Z_PK")]
    pub id: i64,
    #[sea_orm(column_name = "Z_ENT")]
    pub ent: Option<i64>,
    #[sea_orm(column_name = "Z_OPT")]
    pub opt: Option<i64>,
    #[sea_orm(column_name = "ZRELATIONSHIPCRITTERTOCRITTERCATEGORY")]
    pub category: Option<i64>,
    #[sea_orm(column_name = "ZSIZE")]
    pub size: Option<f64>,
    #[sea_orm(column_name = "ZIMAGE")]
    pub image: Option<String>,
    #[sea_orm(column_name = "ZNAME")]
    pub name: Option<String>,
    #[sea_orm(column_name = "ZNOTES")]
    pub notes: Option<String>,
    #[sea_orm(column_name = "ZSPECIES")]
    pub species: Option<String>,
    #[sea_orm(column_name = "ZUUID")]
    pub uuid: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
