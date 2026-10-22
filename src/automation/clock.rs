//! Local wall-clock arithmetic for scheduled rules, generic over the time
//! zone so tests can use a zone with daylight-saving changes.
//!
//! A scheduled time is a local wall time (`19:00` on a date). Each such
//! wall time is one occurrence, whatever the offset: in the autumn the
//! repeated hour still holds each time only once, and a time that doesn't
//! exist in the spring (inside the skipped hour) happens at the first
//! moment after the gap.
use super::rules::{Days, Time, weekday_index};
use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};

/// How far back or ahead a weekly rule can be: just over a week.
const SEARCH_DAYS: usize = 8;

pub fn local<Tz: TimeZone>(tz: &Tz, at: DateTime<Utc>) -> NaiveDateTime {
    at.with_timezone(tz).naive_local()
}

/// The real moment of a local wall time: the earlier one when the hour
/// repeats, the end of the gap when it doesn't exist.
pub fn instant<Tz: TimeZone>(tz: &Tz, local: NaiveDateTime) -> DateTime<Utc> {
    if let Some(at) = tz.from_local_datetime(&local).earliest() {
        return at.with_timezone(&Utc);
    }
    let mut probe = local;
    for _ in 0..(26 * 60) {
        probe += Duration::minutes(1);
        if let Some(at) = tz.from_local_datetime(&probe).earliest() {
            return at.with_timezone(&Utc);
        }
    }
    Utc.from_utc_datetime(&local)
}

/// The latest occurrence in `(after, upto]`.
pub fn latest(
    time: Time,
    days: Days,
    after: NaiveDateTime,
    upto: NaiveDateTime,
) -> Option<NaiveDateTime> {
    if upto <= after {
        return None;
    }
    let mut date = upto.date();
    for _ in 0..SEARCH_DAYS {
        let at = date.and_time(time.naive());
        if at <= upto {
            if at <= after {
                return None;
            }
            if days.has(weekday_index(date)) {
                return Some(at);
            }
        }
        date = date.pred_opt()?;
    }
    None
}

/// The first occurrence strictly after `after`.
pub fn next(time: Time, days: Days, after: NaiveDateTime) -> Option<NaiveDateTime> {
    let mut date = after.date();
    for _ in 0..=SEARCH_DAYS {
        let at = date.and_time(time.naive());
        if at > after && days.has(weekday_index(date)) {
            return Some(at);
        }
        date = date.succ_opt()?;
    }
    None
}

pub fn unix(at: DateTime<Utc>) -> i64 {
    at.timestamp()
}

pub fn from_unix(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(seconds, 0).unwrap_or_default()
}

/// `NaiveDateTime` as text in saved state.
pub mod wall {
    use chrono::NaiveDateTime;
    const FORMAT: &str = "%Y-%m-%dT%H:%M:%S";
    pub fn format(at: NaiveDateTime) -> String {
        at.format(FORMAT).to_string()
    }
    pub fn parse(text: &str) -> Option<NaiveDateTime> {
        NaiveDateTime::parse_from_str(text, FORMAT).ok()
    }
}
