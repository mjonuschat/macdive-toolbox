use chrono::{Duration, NaiveDate, NaiveDateTime};
use diesel::backend::Backend;
use diesel::{
    deserialize::{FromSql, Result},
    sql_types::Text,
    sqlite::Sqlite,
};
use once_cell::sync::Lazy;

/// A representation of a specific point in time that bridges to Date
///
/// [`NsDate`] objects encapsulate a single point in time, independent of any particular
/// calendrical system or time zone. Date objects represent an invariant time
/// interval relative to an absolute reference date (2001-01-01T00:00:00Z).
///
/// [`NsDate`]: https://developer.apple.com/documentation/foundation/nsdate
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[sqlite_type = "Text"]
pub struct NsDate;

static NSDATE_EPOCH: Lazy<NaiveDateTime> =
    Lazy::new(|| NaiveDate::from_ymd(2001, 1, 1).and_hms(0, 0, 0));

impl FromSql<NsDate, Sqlite> for NaiveDateTime {
    fn from_sql(bytes: Option<&<Sqlite as Backend>::RawValue>) -> Result<Self> {
        let value = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;

        if let Ok(ts) = value.parse::<f64>() {
            if let Ok(duration) = Duration::from_std(std::time::Duration::from_secs_f64(ts)) {
                return Ok(*NSDATE_EPOCH + duration);
            }
        }

        Err(format!("Invalid datetime {}", value).into())
    }
}
