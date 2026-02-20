//! Qibla direction calculation module

use serde::{Deserialize, Serialize};

/// Makkah coordinates (Kaaba)
const MAKKAH_LAT: f64 = 21.4225;
const MAKKAH_LNG: f64 = 39.8262;

/// Qibla direction information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QiblaDirection {
    pub from_latitude: f64,
    pub from_longitude: f64,
    pub degrees_from_north: f64,
    pub distance_km: f64,
    pub cardinal_direction: String,
}

impl QiblaDirection {
    /// Calculate Qibla direction from given coordinates
    pub fn calculate(latitude: f64, longitude: f64) -> crate::error::Result<Self> {
        let degrees = calculate_qibla(latitude, longitude);
        let distance = calculate_distance(latitude, longitude);
        let cardinal = degrees_to_cardinal(degrees);

        Ok(QiblaDirection {
            from_latitude: latitude,
            from_longitude: longitude,
            degrees_from_north: degrees,
            distance_km: distance,
            cardinal_direction: cardinal,
        })
    }
}

/// Calculate Qibla direction in degrees from true North
pub fn calculate_qibla(latitude: f64, longitude: f64) -> f64 {
    use std::f64::consts::PI;

    let lat_rad = latitude * PI / 180.0;
    let lng_rad = longitude * PI / 180.0;
    let makkah_lat_rad = MAKKAH_LAT * PI / 180.0;
    let makkah_lng_rad = MAKKAH_LNG * PI / 180.0;

    let delta_lng = makkah_lng_rad - lng_rad;

    let x = delta_lng.sin();
    let y = lat_rad.cos() * makkah_lat_rad.tan() - lat_rad.sin() * delta_lng.cos();

    let qibla_rad = y.atan2(x);
    let qibla_deg = qibla_rad * 180.0 / PI;

    if qibla_deg < 0.0 { qibla_deg + 360.0 } else { qibla_deg }
}

/// Calculate distance to Makkah using Haversine formula
pub fn calculate_distance(latitude: f64, longitude: f64) -> f64 {
    use std::f64::consts::PI;

    const EARTH_RADIUS_KM: f64 = 6371.0;

    let lat1_rad = latitude * PI / 180.0;
    let lat2_rad = MAKKAH_LAT * PI / 180.0;
    let delta_lat = (MAKKAH_LAT - latitude) * PI / 180.0;
    let delta_lng = (MAKKAH_LNG - longitude) * PI / 180.0;

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lng / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_KM * c
}

/// Convert degrees to cardinal direction
fn degrees_to_cardinal(degrees: f64) -> String {
    let normalized = if degrees < 0.0 { degrees + 360.0 } else if degrees >= 360.0 { degrees - 360.0 } else { degrees };

    let directions = [
        "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE",
        "S", "SSW", "SW", "WSW", "W", "WNW", "NW", "NNW",
    ];

    let index = ((normalized + 11.25) / 22.5).floor() as usize % 16;
    directions[index].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qibla_from_new_york() {
        let qibla = QiblaDirection::calculate(40.7128, -74.0060).unwrap();
        assert!(qibla.degrees_from_north > 50.0 && qibla.degrees_from_north < 70.0);
        assert!(qibla.distance_km > 10000.0);
    }
}
