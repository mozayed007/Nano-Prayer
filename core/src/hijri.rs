//! Hijri calendar module
//!
//! Provides conversion between Gregorian and Hijri (Islamic) calendars.
//! Based on the Umm al-Qura calendar algorithm.

use chrono::{Datelike, NaiveDate};
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
/// Based on the Kuwaiti algorithm (similar to Umm al-Qura)
pub fn gregorian_to_hijri(date: NaiveDate) -> HijriDate {
    let year = date.year();
    let month = date.month() as i32;
    let day = date.day() as i32;

    // Calculate Julian Day Number
    let a = (14 - month) / 12;
    let y = year + 4800 - a;
    let m = month + 12 * a - 3;
    let jdn = day + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045;

    // Convert JDN to Hijri using Kuwaiti algorithm
    let l = jdn - 1948440 + 10632;
    let n = (l as f64 / 10631.0).floor() as i32;
    let l2 = l - 10631 * n + 354;
    let j = ((l2 - 1) as f64 / 355.0).floor() as i32;
    let l3 = l2 - 355 * j;
    
    let hijri_day = if l3 == 0 { 30 } else { l3 as u8 };
    let hijri_month = if j == 11 { 12 } else { (j + 1) as u8 };
    let hijri_year = 30 * n + j - 4230;

    HijriDate {
        year: hijri_year,
        month: hijri_month,
        day: hijri_day,
    }
}

/// Convert Hijri date to Gregorian date
pub fn hijri_to_gregorian(hijri: HijriDate) -> NaiveDate {
    // Convert Hijri to Julian Day Number
    let n = (hijri.year as f64 / 30.0).floor() as i32;
    let l = hijri.year - 30 * n;
    let j = ((11 * hijri.month as i32 + 3) / 30) as i32;
    let l2 = 354 * l + (3 * l + 1) / 4 + 29 * hijri.month as i32 + j + hijri.day as i32 - 385;
    
    let jdn = l2 + 1948440 + 10631 * n;

    // Convert JDN to Gregorian
    let a = jdn + 32044;
    let b = ((4 * a + 3) as f64 / 146097.0).floor() as i32;
    let c = a - 146097 * b / 4;
    let d = ((4 * c + 3) as f64 / 1461.0).floor() as i32;
    let e = c - 1461 * d / 4;
    let m = ((5 * e + 2) as f64 / 153.0).floor() as i32;

    let day = (e - (153 * m + 2) / 5 + 1) as u32;
    let month = (m + 3 - 12 * (m / 10)) as u32;
    let year = 100 * b + d - 4800 + (m + 9) / 12;

    NaiveDate::from_ymd_opt(year, month, day)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
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
