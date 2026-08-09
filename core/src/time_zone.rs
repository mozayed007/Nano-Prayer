//! Location timezone helpers with DST / summer-time awareness via `chrono-tz`.

use chrono::{DateTime, Local, NaiveDate, TimeZone, Timelike, Utc};
use chrono_tz::Tz;

/// Parse an IANA zone name (e.g. `Africa/Cairo`, `Europe/London`). Returns `None` if invalid.
pub fn parse_timezone(name: &str) -> Option<Tz> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<Tz>().ok()
}

/// Civil calendar date "today" in the given timezone (respects DST transitions).
pub fn civil_date_now(tz: Option<Tz>) -> NaiveDate {
    civil_date_at(Utc::now(), tz)
}

/// Civil calendar date for an instant in the given timezone.
pub fn civil_date_at(utc: DateTime<Utc>, tz: Option<Tz>) -> NaiveDate {
    match tz {
        Some(tz) => utc.with_timezone(&tz).date_naive(),
        None => utc.with_timezone(&Local).date_naive(),
    }
}

/// Format UTC instant as `HH:MM` in the location timezone (DST-aware).
pub fn format_hm(utc: DateTime<Utc>, tz: Option<Tz>) -> String {
    match tz {
        Some(tz) => utc.with_timezone(&tz).format("%H:%M").to_string(),
        None => utc.with_timezone(&Local).format("%H:%M").to_string(),
    }
}

/// Wall-clock hour (0–23) in the location timezone at `utc`.
pub fn hour_in_zone(utc: DateTime<Utc>, tz: Option<Tz>) -> u32 {
    match tz {
        Some(tz) => utc.with_timezone(&tz).hour(),
        None => utc.with_timezone(&Local).hour(),
    }
}

/// Convert UTC to a `DateTime<Local>` that preserves the **wall clock** of the city zone
/// when possible (for APIs that still expose `DateTime<Local>`). Prefer `time_utc` + `format_hm`
/// for countdowns and display.
pub fn wall_local_from_utc(utc: DateTime<Utc>, tz: Option<Tz>) -> DateTime<Local> {
    match tz {
        Some(tz) => {
            let in_tz = utc.with_timezone(&tz);
            let naive = in_tz.naive_local();
            // Interpret that wall time as system Local (display fields only).
            // Countdowns must use `time_utc`, not this value.
            Local
                .from_local_datetime(&naive)
                .single()
                .unwrap_or_else(|| utc.with_timezone(&Local))
        }
        None => utc.with_timezone(&Local),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn parse_common_zones() {
        assert!(parse_timezone("Africa/Cairo").is_some());
        assert!(parse_timezone("Europe/London").is_some());
        assert!(parse_timezone("America/New_York").is_some());
        assert!(parse_timezone("not/a/zone").is_none());
        assert!(parse_timezone("").is_none());
    }

    #[test]
    fn london_winter_vs_summer_offset_differs() {
        let tz = parse_timezone("Europe/London").expect("London");
        // Mid-January: GMT (UTC+0)
        let winter = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        // Mid-July: BST (UTC+1)
        let summer = Utc.with_ymd_and_hms(2024, 7, 15, 12, 0, 0).unwrap();
        let w = winter.with_timezone(&tz);
        let s = summer.with_timezone(&tz);
        assert_eq!(w.format("%z").to_string(), "+0000");
        assert_eq!(s.format("%z").to_string(), "+0100");
        // Same UTC noon → different wall clocks under DST
        assert_eq!(format_hm(winter, Some(tz)), "12:00");
        assert_eq!(format_hm(summer, Some(tz)), "13:00");
    }

    #[test]
    fn civil_date_can_differ_across_date_line() {
        let tz = parse_timezone("Pacific/Auckland").expect("Auckland");
        // 2024-01-01 10:00 UTC → 2024-01-01 23:00 NZDT (UTC+13)
        let utc = Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap();
        let local_date = civil_date_at(utc, Some(tz));
        assert_eq!(local_date, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        // Later UTC pushes into next local day
        let utc2 = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let local_date2 = civil_date_at(utc2, Some(tz));
        assert_eq!(local_date2, NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
    }
}
