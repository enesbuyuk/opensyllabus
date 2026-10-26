//! Course class schedule: when course day N of week W takes place.

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc};
use chrono_tz::Tz;

use super::recurrence::to_utc;

/// Date of the class on ISO `weekday` (1 = Monday) in course week `week_number`. Weeks are the
/// 7 days starting at `start_date + (week_number - 1) * 7`, as in `progress::week_start_date`,
/// so a course starting on a Wednesday has its Monday class on day 6 of each week.
pub fn class_date(start_date: NaiveDate, week_number: i64, weekday: u32) -> NaiveDate {
    let week_start = start_date + Duration::days((week_number - 1) * 7);
    let offset = (i64::from(weekday) - i64::from(week_start.weekday().number_from_monday())).rem_euclid(7);
    week_start + Duration::days(offset)
}

pub fn parse_time(v: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(v, "%H:%M").ok()
}

/// Start and end of a class in UTC.
pub fn class_times(date: NaiveDate, start: NaiveTime, end: Option<NaiveTime>, tz: Tz) -> (DateTime<Utc>, Option<DateTime<Utc>>) {
    (to_utc(tz, date.and_time(start)), end.map(|e| to_utc(tz, date.and_time(e))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn class_dates_fall_inside_their_week() {
        // Course starts Monday 2026-10-19.
        assert_eq!(class_date(d("2026-10-19"), 1, 1), d("2026-10-19"));
        assert_eq!(class_date(d("2026-10-19"), 1, 3), d("2026-10-21"));
        assert_eq!(class_date(d("2026-10-19"), 2, 5), d("2026-10-30"));
        // Course starts Wednesday 2026-10-21: week 1 runs Wed..Tue, so its Monday is the 26th.
        assert_eq!(class_date(d("2026-10-21"), 1, 3), d("2026-10-21"));
        assert_eq!(class_date(d("2026-10-21"), 1, 1), d("2026-10-26"));
        assert_eq!(class_date(d("2026-10-21"), 2, 2), d("2026-11-03"));
    }

    #[test]
    fn class_times_use_the_schedule_timezone() {
        let (s, e) = class_times(d("2026-10-19"), parse_time("10:00").unwrap(), parse_time("12:30"), chrono_tz::Europe::Istanbul);
        assert_eq!(s.to_rfc3339(), "2026-10-19T07:00:00+00:00");
        assert_eq!(e.unwrap().to_rfc3339(), "2026-10-19T09:30:00+00:00");
        assert!(parse_time("24:00").is_none() && parse_time("9:5x").is_none());
    }
}
