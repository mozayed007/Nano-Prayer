//! Prayer time calculation module using the salah crate

use chrono::{DateTime, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

// Use prelude to get all necessary types including HighLatitudeRule if available
#[allow(unused_imports)]
use salah::prelude::*;
pub use salah::{
    Configuration, Coordinates, Madhab, Method, Prayer as SalahPrayer, PrayerSchedule,
    PrayerTimes as SalahPrayerTimes,
};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Prayer {
    Fajr,
    Sunrise,
    Dhuhr,
    Asr,
    Maghrib,
    Isha,
}

impl Prayer {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().replace(['-', '_'], "").as_str() {
            "fajr" => Some(Self::Fajr),
            "sunrise" => Some(Self::Sunrise),
            "dhuhr" | "zuhr" => Some(Self::Dhuhr),
            "asr" => Some(Self::Asr),
            "maghrib" => Some(Self::Maghrib),
            "isha" | "ishaa" => Some(Self::Isha),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Prayer::Fajr => "Fajr",
            Prayer::Sunrise => "Sunrise",
            Prayer::Dhuhr => "Dhuhr",
            Prayer::Asr => "Asr",
            Prayer::Maghrib => "Maghrib",
            Prayer::Isha => "Isha",
        }
    }

    pub fn to_salah(&self) -> SalahPrayer {
        match self {
            Prayer::Fajr => SalahPrayer::Fajr,
            Prayer::Sunrise => SalahPrayer::Sunrise,
            Prayer::Dhuhr => SalahPrayer::Dhuhr,
            Prayer::Asr => SalahPrayer::Asr,
            Prayer::Maghrib => SalahPrayer::Maghrib,
            Prayer::Isha => SalahPrayer::Isha,
        }
    }
}

impl fmt::Display for Prayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CalculationMethod {
    #[default]
    MuslimWorldLeague,
    Egyptian,
    Karachi,
    UmmAlQura,
    Dubai,
    MoonsightingCommittee,
    NorthAmerica,
    Kuwait,
    Qatar,
    Singapore,
    Tehran,
    Turkey,
}

impl CalculationMethod {
    pub fn to_salah(&self) -> Method {
        match self {
            CalculationMethod::MuslimWorldLeague => Method::MuslimWorldLeague,
            CalculationMethod::Egyptian => Method::Egyptian,
            CalculationMethod::Karachi => Method::Karachi,
            CalculationMethod::UmmAlQura => Method::UmmAlQura,
            CalculationMethod::Dubai => Method::Dubai,
            CalculationMethod::MoonsightingCommittee => Method::MoonsightingCommittee,
            CalculationMethod::NorthAmerica => Method::NorthAmerica,
            CalculationMethod::Kuwait => Method::Kuwait,
            CalculationMethod::Qatar => Method::Qatar,
            CalculationMethod::Singapore => Method::Singapore,
            CalculationMethod::Tehran => Method::Tehran,
            CalculationMethod::Turkey => Method::Turkey,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AsrMadhab {
    #[default]
    Shafi,
    Hanafi,
}

impl AsrMadhab {
    pub fn to_salah(&self) -> Madhab {
        match self {
            AsrMadhab::Shafi => Madhab::Shafi,
            AsrMadhab::Hanafi => Madhab::Hanafi,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HighLatitudeRule {
    #[default]
    MiddleOfTheNight,
    SeventhOfTheNight,
    TwilightAngle,
}

impl HighLatitudeRule {
    pub fn to_salah(&self) {
        // NOTE: `salah` v0.7.6 does not publicly export `HighLatitudeRule` / `PolarCircleResolution`.
        // All high-latitude rules fall back to `Unresolved` until a future `salah` release exposes this type.
        // Track: https://github.com/insha/salah/issues/...
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct PrayerAdjustments {
    pub fajr: i32,
    pub sunrise: i32,
    pub dhuhr: i32,
    pub asr: i32,
    pub maghrib: i32,
    pub isha: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrayerInfo {
    pub prayer: Prayer,
    pub time: DateTime<Local>,
    pub time_utc: DateTime<Utc>,
    pub has_passed: bool,
    pub minutes_until: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrayerTimes {
    pub date: NaiveDate,
    pub location_name: Option<String>,
    pub prayers: Vec<PrayerInfo>,
    pub current_prayer: Option<Prayer>,
    pub next_prayer: Option<Prayer>,
    pub minutes_to_next: Option<i32>,
}

impl PrayerTimes {
    pub fn get_time(&self, prayer: Prayer) -> Option<&PrayerInfo> {
        self.prayers.iter().find(|p| p.prayer == prayer)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrayerCalculator {
    pub method: CalculationMethod,
    pub madhab: AsrMadhab,
    pub high_latitude_rule: HighLatitudeRule,
    pub adjustments: PrayerAdjustments,
}

impl PrayerCalculator {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_method(mut self, m: CalculationMethod) -> Self {
        self.method = m;
        self
    }
    pub fn with_madhab(mut self, m: AsrMadhab) -> Self {
        self.madhab = m;
        self
    }
    pub fn with_high_latitude_rule(mut self, r: HighLatitudeRule) -> Self {
        self.high_latitude_rule = r;
        self
    }
    pub fn with_adjustments(mut self, a: PrayerAdjustments) -> Self {
        self.adjustments = a;
        self
    }

    pub fn calculate(
        &self,
        date: NaiveDate,
        lat: f64,
        lng: f64,
        name: Option<String>,
    ) -> Result<PrayerTimes> {
        let coords = Coordinates::new(lat, lng);
        let config = Configuration::with(self.method.to_salah(), self.madhab.to_salah());
        // .with_high_latitude_rule(self.high_latitude_rule.to_salah()); // FIXME: HighLatitudeRule unavailable
        let st = PrayerSchedule::new()
            .on(date)
            .for_location(coords)
            .with_configuration(config)
            .calculate()
            .map_err(|e| Error::PrayerCalculation(e.to_string()))?;
        let now = Local::now();
        let prayers = vec![
            self.build_info(Prayer::Fajr, st.time(SalahPrayer::Fajr), now),
            self.build_info(Prayer::Sunrise, st.time(SalahPrayer::Sunrise), now),
            self.build_info(Prayer::Dhuhr, st.time(SalahPrayer::Dhuhr), now),
            self.build_info(Prayer::Asr, st.time(SalahPrayer::Asr), now),
            self.build_info(Prayer::Maghrib, st.time(SalahPrayer::Maghrib), now),
            self.build_info(Prayer::Isha, st.time(SalahPrayer::Isha), now),
        ];
        let next_idx = prayers.iter().position(|p| !p.has_passed);
        let mut next_prayer = next_idx.map(|i| prayers[i].prayer);
        let mut mins = next_idx.and_then(|i| prayers[i].minutes_until);

        if next_prayer.is_none() {
            // It is past Isha, next prayer is Fajr tomorrow
            let tomorrow = date + chrono::Duration::days(1);
            if let Ok(st_tomorrow) = PrayerSchedule::new()
                .on(tomorrow)
                .for_location(coords)
                .with_configuration(config)
                .calculate()
            {
                let time_utc = st_tomorrow.time(SalahPrayer::Fajr);
                let time_local = time_utc.with_timezone(&Local);
                let delta = time_local.signed_duration_since(now).num_minutes() as i32;
                // Historical lookups (date << today) leave next Fajr in the past; do not
                // report huge negative countdowns that confuse CLI/agent clients.
                mins = if delta >= 0 { Some(delta) } else { None };
            }
            next_prayer = Some(Prayer::Fajr);
        }

        let current = next_prayer.map(|p| match p {
            Prayer::Fajr => Prayer::Isha,
            Prayer::Sunrise => Prayer::Fajr,
            Prayer::Dhuhr => Prayer::Sunrise,
            Prayer::Asr => Prayer::Dhuhr,
            Prayer::Maghrib => Prayer::Asr,
            Prayer::Isha => Prayer::Maghrib,
        });

        Ok(PrayerTimes {
            date,
            location_name: name,
            prayers,
            current_prayer: current,
            next_prayer,
            minutes_to_next: mins,
        })
    }

    fn build_info(&self, prayer: Prayer, utc: DateTime<Utc>, now: DateTime<Local>) -> PrayerInfo {
        let offset = self.adjustment_for(prayer);
        let adjusted_utc = utc + chrono::Duration::minutes(offset as i64);
        let time = adjusted_utc.with_timezone(&Local);
        let passed = time <= now;
        PrayerInfo {
            prayer,
            time,
            time_utc: adjusted_utc,
            has_passed: passed,
            minutes_until: if passed {
                None
            } else {
                Some((time.signed_duration_since(now)).num_minutes() as i32)
            },
        }
    }

    fn adjustment_for(&self, prayer: Prayer) -> i32 {
        match prayer {
            Prayer::Fajr => self.adjustments.fajr,
            Prayer::Sunrise => self.adjustments.sunrise,
            Prayer::Dhuhr => self.adjustments.dhuhr,
            Prayer::Asr => self.adjustments.asr,
            Prayer::Maghrib => self.adjustments.maghrib,
            Prayer::Isha => self.adjustments.isha,
        }
    }

    pub fn calculate_today(&self, lat: f64, lng: f64, name: Option<String>) -> Result<PrayerTimes> {
        self.calculate(Local::now().date_naive(), lat, lng, name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn t() {
        assert!(PrayerCalculator::new()
            .calculate_today(21.4, 39.8, None)
            .is_ok());
    }

    #[test]
    fn prayer_times_are_ordered_for_makkah() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).expect("valid date");
        let times = PrayerCalculator::new()
            .calculate(date, 21.4225, 39.8262, Some("Makkah".into()))
            .expect("prayer times");
        assert_eq!(times.prayers.len(), 6);
        for window in times.prayers.windows(2) {
            assert!(
                window[0].time < window[1].time,
                "{} should be before {}",
                window[0].prayer,
                window[1].prayer
            );
        }
        let fajr = times.get_time(Prayer::Fajr).unwrap().time;
        let dhuhr = times.get_time(Prayer::Dhuhr).unwrap().time;
        let maghrib = times.get_time(Prayer::Maghrib).unwrap().time;
        assert!(fajr.time().hour() < 12);
        assert!((11..=14).contains(&dhuhr.time().hour()));
        assert!(maghrib.time().hour() >= 17);
    }

    #[test]
    fn historical_day_does_not_report_negative_minutes_to_next() {
        let date = NaiveDate::from_ymd_opt(2020, 1, 1).expect("valid date");
        let times = PrayerCalculator::new()
            .calculate(date, 21.4225, 39.8262, None)
            .expect("times");
        if let Some(mins) = times.minutes_to_next {
            assert!(mins >= 0, "minutes_to_next must not be negative, got {mins}");
        }
    }

    #[test]
    fn adjustments_shift_times_by_minutes() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).expect("valid date");
        let base = PrayerCalculator::new()
            .calculate(date, 21.4225, 39.8262, None)
            .expect("base");
        let adjusted = PrayerCalculator::new()
            .with_adjustments(PrayerAdjustments {
                fajr: 5,
                sunrise: 0,
                dhuhr: 0,
                asr: 0,
                maghrib: 0,
                isha: 0,
            })
            .calculate(date, 21.4225, 39.8262, None)
            .expect("adjusted");
        let base_fajr = base.get_time(Prayer::Fajr).unwrap().time;
        let adj_fajr = adjusted.get_time(Prayer::Fajr).unwrap().time;
        assert_eq!(
            (adj_fajr - base_fajr).num_minutes(),
            5,
            "fajr adjustment should apply +5 minutes"
        );
    }

    #[test]
    fn prayer_parse_aliases() {
        assert_eq!(Prayer::parse("zuhr"), Some(Prayer::Dhuhr));
        assert_eq!(Prayer::parse("ISHA"), Some(Prayer::Isha));
        assert_eq!(Prayer::parse("nope"), None);
    }
}
