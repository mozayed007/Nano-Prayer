//! Hijri calendar module
//!
//! Provides conversion between Gregorian and Hijri (Islamic) calendars.
//! Based on the Umm al-Qura calendar algorithm.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Hijri date representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HijriDate {
    /// Year (AH - After Hijrah)
    pub year: i32,
    /// Month (1-12)
    pub month: u8,
    /// Day (1-30)
    pub day: u8,
}

impl HijriDate {
    /// Create a new Hijri date
    pub fn new(year: i32, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }

    /// Convert from Gregorian date
    pub fn from_gregorian(date: NaiveDate) -> Self {
        gregorian_to_hijri(date)
    }

    /// Convert to Gregorian date
    pub fn to_gregorian(&self) -> NaiveDate {
        hijri_to_gregorian(*self)
    }

    /// Get month name in Arabic
    pub fn month_name_arabic(&self) -> &'static str {
        hijri_month_name_arabic(self.month)
    }

    /// Get month name in English
    pub fn month_name_english(&self) -> &'static str {
        hijri_month_name_english(self.month)
    }

    /// Format as string (e.g., "15 Ramadan 1445")
    pub fn format(&self) -> String {
        format!("{} {} {}", self.day, self.month_name_english(), self.year)
    }

    /// Format in Arabic (e.g., "١٥ رمضان ١٤٤٥")
    pub fn format_arabic(&self) -> String {
        format!(
            "{} {} {}",
            to_arabic_numerals(self.day as u32),
            self.month_name_arabic(),
            to_arabic_numerals(self.year as u32)
        )
    }

    /// Get today's Hijri date
    pub fn today() -> Self {
        Self::from_gregorian(chrono::Local::now().date_naive())
    }
}

/// Hijri month names in English
pub fn hijri_month_name_english(month: u8) -> &'static str {
    match month {
        1 => "Muharram",
        2 => "Safar",
        3 => "Rabi al-Awwal",
        4 => "Rabi al-Thani",
        5 => "Jumada al-Awwal",
        6 => "Jumada al-Thani",
        7 => "Rajab",
        8 => "Shaban",
        9 => "Ramadan",
        10 => "Shawwal",
        11 => "Dhu al-Qadah",
        12 => "Dhu al-Hijjah",
        _ => "Unknown",
    }
}

/// Hijri month names in Arabic
pub fn hijri_month_name_arabic(month: u8) -> &'static str {
    match month {
        1 => "محرم",
        2 => "صفر",
        3 => "ربيع الأول",
        4 => "ربيع الثاني",
        5 => "جمادى الأولى",
        6 => "جمادى الثانية",
        7 => "رجب",
        8 => "شعبان",
        9 => "رمضان",
        10 => "شوال",
        11 => "ذو القعدة",
        12 => "ذو الحجة",
        _ => "غير معروف",
    }
}

/// Convert Western numerals to Arabic numerals
pub fn to_arabic_numerals(n: u32) -> String {
    let arabic_digits = ['٠', '١', '٢', '٣', '٤', '٥', '٦', '٧', '٨', '٩'];
    n.to_string()
        .chars()
        .map(|c| {
            if c.is_ascii_digit() {
                arabic_digits[c as usize - '0' as usize]
            } else {
                c
            }
        })
        .collect()
}

/// Convert Gregorian date to Hijri date
/// Uses Tabular Islamic Calendar (highly deterministic fallback for Umm al-Qura)
pub fn gregorian_to_hijri(date: NaiveDate) -> HijriDate {
    // Epoch: 1 Muharram 1 AH = July 19, 622 CE (proleptic Gregorian)
    // Note: July 16, 622 is the Julian date; chrono uses proleptic Gregorian (+3 day offset)
    let epoch = NaiveDate::from_ymd_opt(622, 7, 19).unwrap();
    let days_since_epoch = date.signed_duration_since(epoch).num_days();

    if days_since_epoch < 0 {
        return HijriDate {
            year: 1,
            month: 1,
            day: 1,
        };
    }

    // 30-year cycle has 10631 days.
    let cycles = days_since_epoch / 10631;
    let days_in_current_cycle = days_since_epoch % 10631;

    let mut year_in_cycle = 1;
    let mut remaining_days = days_in_current_cycle;

    // Lengths of years in 30-year cycle (leap years: 2, 5, 7, 10, 13, 16, 18, 21, 24, 26, 29)
    for y in 1..=30 {
        let is_leap = is_leap_year_in_cycle(y);
        let year_len = if is_leap { 355 } else { 354 };
        if remaining_days < year_len {
            year_in_cycle = y;
            break;
        }
        remaining_days -= year_len;
    }

    let mut month = 1;
    let mut day = remaining_days + 1;

    for m in 1..=12 {
        let month_len = hijri_month_len(m, year_in_cycle);
        if day <= month_len {
            month = m;
            break;
        }
        day -= month_len;
    }

    let hijri_year = (cycles * 30) + year_in_cycle as i64;

    HijriDate {
        year: hijri_year as i32,
        month: month as u8,
        day: day as u8,
    }
}

/// Convert Hijri date to Gregorian date
pub fn hijri_to_gregorian(hijri: HijriDate) -> NaiveDate {
    let mut days = (hijri.year as i64 - 1) / 30 * 10631;
    let year_in_cycle = (hijri.year - 1) % 30 + 1;

    for y in 1..year_in_cycle {
        let is_leap = is_leap_year_in_cycle(y);
        days += if is_leap { 355 } else { 354 };
    }

    for m in 1..hijri.month {
        let month_len = hijri_month_len(i32::from(m), year_in_cycle);
        days += month_len;
    }

    days += hijri.day as i64 - 1;

    let epoch = NaiveDate::from_ymd_opt(622, 7, 19).unwrap();
    epoch + chrono::Duration::days(days)
}

fn is_leap_year_in_cycle(year_in_cycle: i32) -> bool {
    [2, 5, 7, 10, 13, 16, 18, 21, 24, 26, 29].contains(&year_in_cycle)
}

fn hijri_month_len(month: i32, year_in_cycle: i32) -> i64 {
    if month % 2 != 0 || (month == 12 && is_leap_year_in_cycle(year_in_cycle)) {
        30
    } else {
        29
    }
}

/// Apply offset to Hijri date (for moon sighting adjustments)
pub fn apply_offset(hijri: HijriDate, day_offset: i32) -> HijriDate {
    let gregorian = hijri.to_gregorian();
    let adjusted = gregorian + chrono::Duration::days(day_offset as i64);
    HijriDate::from_gregorian(adjusted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hijri_month_names() {
        assert_eq!(hijri_month_name_english(1), "Muharram");
        assert_eq!(hijri_month_name_english(9), "Ramadan");
        assert_eq!(hijri_month_name_arabic(9), "رمضان");
    }

    #[test]
    fn test_gregorian_to_hijri() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let hijri = gregorian_to_hijri(date);
        assert!(hijri.year >= 1445 && hijri.year <= 1446);
    }

    #[test]
    fn test_arabic_numerals() {
        assert_eq!(to_arabic_numerals(1), "١");
        assert_eq!(to_arabic_numerals(15), "١٥");
        assert_eq!(to_arabic_numerals(1445), "١٤٤٥");
    }

    #[test]
    fn test_hijri_today() {
        let today = HijriDate::today();
        assert!(today.month >= 1 && today.month <= 12);
        assert!(today.day >= 1 && today.day <= 30);
    }
}
