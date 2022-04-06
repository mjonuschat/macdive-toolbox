use crate::macdive::types::NsDate;

#[derive(Debug, Clone, PartialEq)]
pub struct Critter {
    pub id: i64,
    pub ent: Option<i64>,
    pub opt: Option<i64>,
    pub category: Option<i64>,
    pub size: Option<f32>,
    pub image: Option<String>,
    pub name: Option<String>,
    pub notes: Option<String>,
    pub species: Option<String>,
    pub uuid: Option<String>,
}

#[derive(Debug)]
pub struct CritterCategory {
    pub id: i64,
    pub ent: Option<i64>,
    pub opt: Option<i64>,
    pub image: Option<String>,
    pub name: Option<String>,
    pub uuid: Option<String>,
}

#[derive(Debug, Default)]
pub struct CritterUpdate {
    pub id: i64,
    pub category: Option<i64>,
    pub common_name: Option<String>,
    pub scientific_name: Option<String>,
}

impl CritterUpdate {
    pub fn has_changes(&self) -> bool {
        self.category.is_some() || self.common_name.is_some() || self.scientific_name.is_some()
    }
}

#[derive(Debug)]
pub struct DiveSite {
    pub id: i64,
    pub ent: Option<i64>,
    pub opt: Option<i64>,
    pub altitude: Option<f32>,
    pub latitude: Option<f32>,
    pub longitude: Option<f32>,
    pub modified_at: Option<NsDate>,
    pub body_of_water: Option<String>,
    pub country: Option<String>,
    pub difficulty: Option<String>,
    pub divelog_uuid: Option<String>,
    pub flag: Option<String>,
    pub image: Option<String>,
    pub last_divelog_image_hash: Option<String>,
    pub location: Option<String>,
    pub name: Option<String>,
    pub notes: Option<String>,
    pub uuid: Option<String>,
    pub water_type: Option<String>,
    pub zoom: Option<String>,
}
