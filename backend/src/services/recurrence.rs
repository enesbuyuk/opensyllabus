//! Repeating calendar events: expands a series into the occurrences inside a time window.
//!
//! Occurrences keep the wall-clock time of the first one in the series timezone, so a
//! 10:00 session stays at 10:00 across daylight-saving changes.

use chrono::{DateTime, Datelike, Duration, LocalResult, Months, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

/// Upper bound on candidate dates scanned for one series in one request.
const MAX_STEPS: usize = 20_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freq {
    Daily,
    Weekly,
    Monthly,
}

impl Freq {
    pub fn parse(v: &str) -> Option<Self> {
        match v {
            "daily" => Some(Self::Daily),
            "weekly" => Some(Self::Weekly),
            "monthly" => Some(Self::Monthly),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub freq: Freq,
    pub interval: u32,
    /// ISO weekdays (1 = Monday); weekly rules only. Empty means the weekday of the start.
    pub weekdays: Vec<u32>,
    /// Last occurrence date, inclusive.
    pub until: Option<NaiveDate>,
    /// Number of occurrences, skipped ones included.
    pub count: Option<u32>,
    /// Dates removed from the series.
    pub exceptions: Vec<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    /// Local date in the series timezone.
    pub date: NaiveDate,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
}

/// Local wall time to UTC. Ambiguous times take the earlier instant; times skipped by a
/// DST jump move forward by an hour.
fn to_utc(tz: Tz, local: NaiveDateTime) -> DateTime<Utc> {
    let r = match tz.from_local_datetime(&local) {
        LocalResult::None => tz.from_local_datetime(&(local + Duration::hours(1))),
        other => other,
    };
    r.earliest().map(|t| t.with_timezone(&Utc)).unwrap_or_else(|| Utc.from_utc_datetime(&local))
}

/// Candidate dates of the rule in order, starting at `start` (the first occurrence).
fn candidates(start: NaiveDate, rule: &Rule) -> Box<dyn Iterator<Item = NaiveDate>> {
    let step = rule.interval.max(1);
    match rule.freq {
        Freq::Daily => Box::new((0..).map_while(move |k: u32| start.checked_add_signed(Duration::days(i64::from(k) * i64::from(step))))),
        Freq::Monthly => {
            Box::new((0..).map_while(move |k: u32| start.checked_add_months(Months::new(k * step)).map(|d| (d, start.day()))).filter_map(
                // checked_add_months clamps the 31st to the 30th; skip months without that day instead.
                |(d, day)| (d.day() == day).then_some(d),
            ))
        }
        Freq::Weekly => {
            let mut days: Vec<u32> =
                if rule.weekdays.is_empty() { vec![start.weekday().number_from_monday()] } else { rule.weekdays.clone() };
            days.sort_unstable();
            days.dedup();
            let monday = start - Duration::days(i64::from(start.weekday().num_days_from_monday()));
            Box::new(
                (0..)
                    .map_while(move |w: u32| monday.checked_add_signed(Duration::weeks(i64::from(w) * i64::from(step))))
                    .flat_map(move |week| days.clone().into_iter().map(move |d| week + Duration::days(i64::from(d) - 1)))
                    .filter(move |d| *d >= start),
            )
        }
    }
}

/// Occurrences overlapping `[from, to)`. `starts_at` / `ends_at` are the first occurrence.
pub fn expand(
    starts_at: DateTime<Utc>,
    ends_at: Option<DateTime<Utc>>,
    tz: Tz,
    rule: &Rule,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Vec<Occurrence> {
    let local = starts_at.with_timezone(&tz).naive_local();
    let duration = ends_at.map(|e| e - starts_at);
    let mut out = Vec::new();
    for (n, date) in candidates(local.date(), rule).enumerate().take(MAX_STEPS) {
        if rule.until.is_some_and(|u| date > u) || rule.count.is_some_and(|c| n as u32 >= c) {
            break;
        }
        let start = to_utc(tz, date.and_time(local.time()));
        if start >= to {
            break;
        }
        let end = duration.map(|d| start + d);
        if rule.exceptions.contains(&date) || end.unwrap_or(start) < from {
            continue;
        }
        out.push(Occurrence { date, starts_at: start, ends_at: end });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn rule(freq: Freq) -> Rule {
        Rule { freq, interval: 1, weekdays: vec![], until: None, count: None, exceptions: vec![] }
    }

    fn dates(o: &[Occurrence]) -> Vec<String> {
        o.iter().map(|o| o.date.to_string()).collect()
    }

    #[test]
    fn weekly_on_chosen_days_with_count() {
        // Monday 2026-10-19 10:00 Istanbul (07:00 UTC), Mon + Wed, 5 occurrences.
        let r = Rule { weekdays: vec![3, 1], count: Some(5), ..rule(Freq::Weekly) };
        let o = expand(
            utc("2026-10-19T07:00:00Z"),
            Some(utc("2026-10-19T09:00:00Z")),
            chrono_tz::Europe::Istanbul,
            &r,
            utc("2026-10-01T00:00:00Z"),
            utc("2027-01-01T00:00:00Z"),
        );
        assert_eq!(dates(&o), ["2026-10-19", "2026-10-21", "2026-10-26", "2026-10-28", "2026-11-02"]);
        assert_eq!(o[1].starts_at, utc("2026-10-21T07:00:00Z"));
        assert_eq!(o[1].ends_at, Some(utc("2026-10-21T09:00:00Z")));
    }

    #[test]
    fn every_other_week_until_date_skipping_exceptions() {
        let r = Rule { interval: 2, until: Some(date("2026-11-30")), exceptions: vec![date("2026-11-02")], ..rule(Freq::Weekly) };
        let o = expand(
            utc("2026-10-19T07:00:00Z"),
            None,
            chrono_tz::Europe::Istanbul,
            &r,
            utc("2026-10-01T00:00:00Z"),
            utc("2027-01-01T00:00:00Z"),
        );
        assert_eq!(dates(&o), ["2026-10-19", "2026-11-16", "2026-11-30"]);
    }

    #[test]
    fn only_occurrences_in_window_and_series_start_ignores_earlier_weekdays() {
        // Starts on a Wednesday with Mon + Wed: the Monday before the start is not an occurrence.
        let r = Rule { weekdays: vec![1, 3], ..rule(Freq::Weekly) };
        let o = expand(
            utc("2026-10-21T07:00:00Z"),
            None,
            chrono_tz::Europe::Istanbul,
            &r,
            utc("2026-11-01T00:00:00Z"),
            utc("2026-11-10T00:00:00Z"),
        );
        assert_eq!(dates(&o), ["2026-11-02", "2026-11-04", "2026-11-09"]);
        let o = expand(
            utc("2026-10-21T07:00:00Z"),
            None,
            chrono_tz::Europe::Istanbul,
            &r,
            utc("2026-10-01T00:00:00Z"),
            utc("2026-10-27T00:00:00Z"),
        );
        assert_eq!(dates(&o), ["2026-10-21", "2026-10-26"]);
    }

    #[test]
    fn keeps_wall_clock_time_across_dst() {
        // Daily 10:00 in Berlin across the 2026-10-25 change from CEST (+2) to CET (+1).
        let o = expand(
            utc("2026-10-24T08:00:00Z"),
            None,
            chrono_tz::Europe::Berlin,
            &rule(Freq::Daily),
            utc("2026-10-24T00:00:00Z"),
            utc("2026-10-27T00:00:00Z"),
        );
        assert_eq!(
            o.iter().map(|o| o.starts_at).collect::<Vec<_>>(),
            [utc("2026-10-24T08:00:00Z"), utc("2026-10-25T09:00:00Z"), utc("2026-10-26T09:00:00Z")]
        );
    }

    #[test]
    fn monthly_skips_months_without_the_day() {
        let r = Rule { count: Some(4), ..rule(Freq::Monthly) };
        let o = expand(utc("2027-01-31T10:00:00Z"), None, chrono_tz::UTC, &r, utc("2027-01-01T00:00:00Z"), utc("2028-01-01T00:00:00Z"));
        // February, April and June have no 31st: skipped, and not counted toward the 4.
        assert_eq!(dates(&o), ["2027-01-31", "2027-03-31", "2027-05-31", "2027-07-31"]);
    }

    #[test]
    fn long_running_series_reaches_a_late_window() {
        let o = expand(
            utc("2020-01-01T09:00:00Z"),
            None,
            chrono_tz::UTC,
            &rule(Freq::Daily),
            utc("2026-10-26T00:00:00Z"),
            utc("2026-10-28T00:00:00Z"),
        );
        assert_eq!(dates(&o), ["2026-10-26", "2026-10-27"]);
    }
}
