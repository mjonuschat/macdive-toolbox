use crate::macdive::types::NsDate;

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
