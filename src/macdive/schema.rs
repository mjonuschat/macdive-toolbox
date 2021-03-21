table! {
    use diesel::sql_types::*;
    use crate::macdive::types::NsDate;

    #[sql_name="ZDIVESITE"]
    divesites (id) {
        #[sql_name="Z_PK"]
        id -> Integer,
        #[sql_name="Z_ENT"]
        ent -> Nullable<Integer>,
        #[sql_name="Z_OPT"]
        opt -> Nullable<Integer>,
        #[sql_name="ZALTITUDE"]
        altitude -> Nullable<Float>,
        #[sql_name="ZGPSLAT"]
        latitude -> Nullable<Float>,
        #[sql_name="ZGPSLON"]
        longitude -> Nullable<Float>,
        #[sql_name="ZMODIFIED"]
        modified_at -> Nullable<NsDate>,
        #[sql_name="ZBODYOFWATER"]
        body_of_water -> Nullable<Text>,
        #[sql_name="ZCOUNTRY"]
        country -> Nullable<Text>,
        #[sql_name="ZDIFFICULTY"]
        difficulty -> Nullable<Text>,
        #[sql_name="ZDIVELOGUUID"]
        divelog_uuid -> Nullable<Text>,
        #[sql_name="ZFLAG"]
        flag -> Nullable<Text>,
        #[sql_name="ZIMAGE"]
        image -> Nullable<Text>,
        #[sql_name="ZLASTDIVELOGIMAGEHASH"]
        last_divelog_image_hash -> Nullable<Text>,
        #[sql_name="ZLOCATION"]
        location -> Nullable<Text>,
        #[sql_name="ZNAME"]
        name -> Nullable<Text>,
        #[sql_name="ZNOTES"]
        notes -> Nullable<Text>,
        #[sql_name="ZUUID"]
        uuid -> Nullable<Text>,
        #[sql_name="ZWATERTYPE"]
        water_type -> Nullable<Text>,
        #[sql_name="ZZOOM"]
        zoom -> Nullable<Text>,
    }
}
