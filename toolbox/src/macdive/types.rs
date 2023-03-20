use chrono::{Duration, NaiveDate, NaiveDateTime};
use once_cell::sync::Lazy;

/// A representation of a specific point in time that bridges to Date
///
/// [`NsDate`] objects encapsulate a single point in time, independent of any particular
/// calendrical system or time zone. Date objects represent an invariant time
/// interval relative to an absolute reference date (2001-01-01T00:00:00Z).
///
/// [`NsDate`]: https://developer.apple.com/documentation/foundation/nsdate
#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(transparent)]
pub struct NsDate(f64);

static NSDATE_EPOCH: Lazy<NaiveDateTime> = Lazy::new(|| {
    NaiveDate::from_ymd_opt(2001, 1, 1)
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .expect("Unix epoch should be a valid date/time")
});

impl From<NsDate> for NaiveDateTime {
    fn from(value: NsDate) -> Self {
        if let Ok(duration) = Duration::from_std(std::time::Duration::from_secs_f64(value.0)) {
            return *NSDATE_EPOCH + duration;
        }

        *NSDATE_EPOCH
    }
}
