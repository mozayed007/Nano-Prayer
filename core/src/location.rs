//! Location management module
//!
//! Handles location data, city database, and multi-location management.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: Option<f64>,
}

impl Coordinates {
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            elevation: None,
        }
    }

    pub fn with_elevation(latitude: f64, longitude: f64, elevation: f64) -> Self {
        Self {
            latitude,
            longitude,
            elevation: Some(elevation),
        }
    }

    pub fn is_valid(&self) -> bool {
        (-90.0..=90.0).contains(&self.latitude) && (-180.0..=180.0).contains(&self.longitude)
    }
}

impl Default for Coordinates {
    fn default() -> Self {
        Self::new(21.4225, 39.8262)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct City {
    pub name: String,
    pub country: String,
    pub country_code: String,
    pub region: Option<String>,
    pub coordinates: Coordinates,
    pub timezone: String,
    pub population: Option<u64>,
}

impl City {
    pub fn new(
        name: String,
        country: String,
        country_code: String,
        latitude: f64,
        longitude: f64,
        timezone: String,
    ) -> Self {
        Self {
            name,
            country,
            country_code,
            region: None,
            coordinates: Coordinates::new(latitude, longitude),
            timezone,
            population: None,
        }
    }

    pub fn display_name(&self) -> String {
        match &self.region {
            Some(region) => format!("{}, {}, {}", self.name, region, self.country),
            None => format!("{}, {}", self.name, self.country),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedLocation {
    pub id: String,
    pub name: String,
    pub coordinates: Coordinates,
    pub timezone: String,
    pub is_favorite: bool,
    pub is_default: bool,
    pub calculation_method: Option<String>,
    pub notes: Option<String>,
}

impl SavedLocation {
    pub fn new(name: String, coordinates: Coordinates, timezone: String) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            id,
            name,
            coordinates,
            timezone,
            is_favorite: false,
            is_default: false,
            calculation_method: None,
            notes: None,
        }
    }

    pub fn from_city(city: &City) -> Self {
        Self::new(city.display_name(), city.coordinates, city.timezone.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationManager {
    pub locations: Vec<SavedLocation>,
    pub current_index: Option<usize>,
}

impl Default for LocationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LocationManager {
    pub fn new() -> Self {
        Self {
            locations: Vec::new(),
            current_index: None,
        }
    }

    pub fn add(&mut self, location: SavedLocation) {
        let is_first = self.locations.is_empty();
        let mut location = location;
        if is_first {
            location.is_default = true;
            self.current_index = Some(0);
        }
        self.locations.push(location);
    }

    pub fn remove(&mut self, id: &str) -> Result<()> {
        let index = self
            .locations
            .iter()
            .position(|l| l.id == id)
            .ok_or_else(|| Error::LocationNotFound(id.to_string()))?;

        let was_default = self.locations[index].is_default;
        self.locations.remove(index);

        if let Some(current) = self.current_index {
            if current >= index && current > 0 {
                self.current_index = Some(current - 1);
            } else if current >= self.locations.len() {
                self.current_index = if self.locations.is_empty() {
                    None
                } else {
                    Some(self.locations.len() - 1)
                };
            }
        }

        if was_default && !self.locations.is_empty() {
            self.locations[0].is_default = true;
        }

        Ok(())
    }

    pub fn current(&self) -> Option<&SavedLocation> {
        self.current_index.and_then(|i| self.locations.get(i))
    }

    pub fn set_current(&mut self, index: usize) -> Result<()> {
        if index >= self.locations.len() {
            return Err(Error::LocationNotFound(format!(
                "Index {} out of bounds",
                index
            )));
        }
        self.current_index = Some(index);
        Ok(())
    }

    pub fn set_current_by_id(&mut self, id: &str) -> Result<()> {
        let index = self
            .locations
            .iter()
            .position(|l| l.id == id)
            .ok_or_else(|| Error::LocationNotFound(id.to_string()))?;
        self.set_current(index)
    }

    pub fn default_location(&self) -> Option<&SavedLocation> {
        self.locations.iter().find(|l| l.is_default)
    }

    pub fn set_default(&mut self, id: &str) -> Result<()> {
        for location in &mut self.locations {
            location.is_default = false;
        }
        let location = self
            .locations
            .iter_mut()
            .find(|l| l.id == id)
            .ok_or_else(|| Error::LocationNotFound(id.to_string()))?;
        location.is_default = true;
        Ok(())
    }

    pub fn toggle_favorite(&mut self, id: &str) -> Result<()> {
        let location = self
            .locations
            .iter_mut()
            .find(|l| l.id == id)
            .ok_or_else(|| Error::LocationNotFound(id.to_string()))?;
        location.is_favorite = !location.is_favorite;
        Ok(())
    }

    pub fn favorites(&self) -> Vec<&SavedLocation> {
        self.locations.iter().filter(|l| l.is_favorite).collect()
    }

    pub fn import_json(&mut self, json: &str) -> Result<usize> {
        let imported: Vec<SavedLocation> = serde_json::from_str(json)?;
        let count = imported.len();
        for location in imported {
            self.add(location);
        }
        Ok(count)
    }

    pub fn export_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(&self.locations)?)
    }
}

pub struct CityDatabase {
    cities: Vec<City>,
    by_name: HashMap<String, Vec<usize>>,
    by_country: HashMap<String, Vec<usize>>,
}

impl Default for CityDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl CityDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            cities: Vec::new(),
            by_name: HashMap::new(),
            by_country: HashMap::new(),
        };
        db.load_default_cities();
        db
    }

    pub fn with_cities(cities: Vec<City>) -> Self {
        let mut db = Self {
            cities,
            by_name: HashMap::new(),
            by_country: HashMap::new(),
        };
        db.build_indexes();
        db
    }

    fn load_default_cities(&mut self) {
        let default_cities = vec![
            City::new(
                "Makkah".into(),
                "Saudi Arabia".into(),
                "SA".into(),
                21.4225,
                39.8262,
                "Asia/Riyadh".into(),
            ),
            City::new(
                "Madinah".into(),
                "Saudi Arabia".into(),
                "SA".into(),
                24.4672,
                39.6024,
                "Asia/Riyadh".into(),
            ),
            City::new(
                "Riyadh".into(),
                "Saudi Arabia".into(),
                "SA".into(),
                24.7136,
                46.6753,
                "Asia/Riyadh".into(),
            ),
            City::new(
                "Jeddah".into(),
                "Saudi Arabia".into(),
                "SA".into(),
                21.4858,
                39.1925,
                "Asia/Riyadh".into(),
            ),
            City::new(
                "Dubai".into(),
                "United Arab Emirates".into(),
                "AE".into(),
                25.2048,
                55.2708,
                "Asia/Dubai".into(),
            ),
            City::new(
                "Abu Dhabi".into(),
                "United Arab Emirates".into(),
                "AE".into(),
                24.4539,
                54.3773,
                "Asia/Dubai".into(),
            ),
            City::new(
                "Cairo".into(),
                "Egypt".into(),
                "EG".into(),
                30.0444,
                31.2357,
                "Africa/Cairo".into(),
            ),
            City::new(
                "Alexandria".into(),
                "Egypt".into(),
                "EG".into(),
                31.2001,
                29.9187,
                "Africa/Cairo".into(),
            ),
            City::new(
                "Istanbul".into(),
                "Turkey".into(),
                "TR".into(),
                41.0082,
                28.9784,
                "Europe/Istanbul".into(),
            ),
            City::new(
                "Ankara".into(),
                "Turkey".into(),
                "TR".into(),
                39.9334,
                32.8597,
                "Europe/Istanbul".into(),
            ),
            City::new(
                "Karachi".into(),
                "Pakistan".into(),
                "PK".into(),
                24.8607,
                67.0011,
                "Asia/Karachi".into(),
            ),
            City::new(
                "Lahore".into(),
                "Pakistan".into(),
                "PK".into(),
                31.5204,
                74.3587,
                "Asia/Karachi".into(),
            ),
            City::new(
                "Islamabad".into(),
                "Pakistan".into(),
                "PK".into(),
                33.6844,
                73.0479,
                "Asia/Karachi".into(),
            ),
            City::new(
                "Jakarta".into(),
                "Indonesia".into(),
                "ID".into(),
                -6.2088,
                106.8456,
                "Asia/Jakarta".into(),
            ),
            City::new(
                "Dhaka".into(),
                "Bangladesh".into(),
                "BD".into(),
                23.8103,
                90.4125,
                "Asia/Dhaka".into(),
            ),
            City::new(
                "Kuala Lumpur".into(),
                "Malaysia".into(),
                "MY".into(),
                3.1390,
                101.6869,
                "Asia/Kuala_Lumpur".into(),
            ),
            City::new(
                "London".into(),
                "United Kingdom".into(),
                "GB".into(),
                51.5074,
                -0.1278,
                "Europe/London".into(),
            ),
            City::new(
                "Manchester".into(),
                "United Kingdom".into(),
                "GB".into(),
                53.4808,
                -2.2426,
                "Europe/London".into(),
            ),
            City::new(
                "Birmingham".into(),
                "United Kingdom".into(),
                "GB".into(),
                52.4862,
                -1.8904,
                "Europe/London".into(),
            ),
            City::new(
                "Paris".into(),
                "France".into(),
                "FR".into(),
                48.8566,
                2.3522,
                "Europe/Paris".into(),
            ),
            City::new(
                "Berlin".into(),
                "Germany".into(),
                "DE".into(),
                52.5200,
                13.4050,
                "Europe/Berlin".into(),
            ),
            City::new(
                "New York".into(),
                "United States".into(),
                "US".into(),
                40.7128,
                -74.0060,
                "America/New_York".into(),
            ),
            City::new(
                "Los Angeles".into(),
                "United States".into(),
                "US".into(),
                34.0522,
                -118.2437,
                "America/Los_Angeles".into(),
            ),
            City::new(
                "Chicago".into(),
                "United States".into(),
                "US".into(),
                41.8781,
                -87.6298,
                "America/Chicago".into(),
            ),
            City::new(
                "Houston".into(),
                "United States".into(),
                "US".into(),
                29.7604,
                -95.3698,
                "America/Chicago".into(),
            ),
            City::new(
                "Toronto".into(),
                "Canada".into(),
                "CA".into(),
                43.6532,
                -79.3832,
                "America/Toronto".into(),
            ),
            City::new(
                "Vancouver".into(),
                "Canada".into(),
                "CA".into(),
                49.2827,
                -123.1207,
                "America/Vancouver".into(),
            ),
            City::new(
                "Sydney".into(),
                "Australia".into(),
                "AU".into(),
                -33.8688,
                151.2093,
                "Australia/Sydney".into(),
            ),
            City::new(
                "Melbourne".into(),
                "Australia".into(),
                "AU".into(),
                -37.8136,
                144.9631,
                "Australia/Melbourne".into(),
            ),
            City::new(
                "Tokyo".into(),
                "Japan".into(),
                "JP".into(),
                35.6762,
                139.6503,
                "Asia/Tokyo".into(),
            ),
            City::new(
                "Delhi".into(),
                "India".into(),
                "IN".into(),
                28.7041,
                77.1025,
                "Asia/Kolkata".into(),
            ),
            City::new(
                "Mumbai".into(),
                "India".into(),
                "IN".into(),
                19.0760,
                72.8777,
                "Asia/Kolkata".into(),
            ),
            City::new(
                "Morocco".into(),
                "Casablanca".into(),
                "MA".into(),
                33.5731,
                -7.5898,
                "Africa/Casablanca".into(),
            ),
            City::new(
                "Tunis".into(),
                "Tunisia".into(),
                "TN".into(),
                36.8065,
                10.1815,
                "Africa/Tunis".into(),
            ),
            City::new(
                "Amman".into(),
                "Jordan".into(),
                "JO".into(),
                31.9454,
                35.9284,
                "Asia/Amman".into(),
            ),
            City::new(
                "Beirut".into(),
                "Lebanon".into(),
                "LB".into(),
                33.8938,
                35.5018,
                "Asia/Beirut".into(),
            ),
            City::new(
                "Doha".into(),
                "Qatar".into(),
                "QA".into(),
                25.2854,
                51.5310,
                "Asia/Qatar".into(),
            ),
            City::new(
                "Kuwait City".into(),
                "Kuwait".into(),
                "KW".into(),
                29.3759,
                47.9774,
                "Asia/Kuwait".into(),
            ),
            City::new(
                "Muscat".into(),
                "Oman".into(),
                "OM".into(),
                23.5880,
                58.3829,
                "Asia/Muscat".into(),
            ),
            City::new(
                "Singapore".into(),
                "Singapore".into(),
                "SG".into(),
                1.3521,
                103.8198,
                "Asia/Singapore".into(),
            ),
        ];

        self.cities = default_cities;
        self.build_indexes();
    }

    fn build_indexes(&mut self) {
        self.by_name.clear();
        self.by_country.clear();

        for (idx, city) in self.cities.iter().enumerate() {
            let name_key = city.name.to_lowercase();
            self.by_name.entry(name_key).or_default().push(idx);

            let country_key = city.country.to_lowercase();
            self.by_country.entry(country_key).or_default().push(idx);
        }
    }

    pub fn search(&self, query: &str) -> Vec<&City> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<&City> = Vec::new();

        if let Some(indices) = self.by_name.get(&query_lower) {
            for &idx in indices {
                results.push(&self.cities[idx]);
            }
        }

        for (name, indices) in &self.by_name {
            if name.contains(&query_lower) && !name.eq(&query_lower) {
                for &idx in indices {
                    if !results.iter().any(|c| std::ptr::eq(*c, &self.cities[idx])) {
                        results.push(&self.cities[idx]);
                    }
                }
            }
        }

        for (country, indices) in &self.by_country {
            if country.contains(&query_lower) {
                for &idx in indices {
                    if !results.iter().any(|c| std::ptr::eq(*c, &self.cities[idx])) {
                        results.push(&self.cities[idx]);
                    }
                }
            }
        }

        results
    }

    pub fn search_by_country(&self, country: &str) -> Vec<&City> {
        let country_lower = country.to_lowercase();
        self.by_country
            .get(&country_lower)
            .map(|indices| indices.iter().map(|&idx| &self.cities[idx]).collect())
            .unwrap_or_default()
    }

    pub fn get_by_name(&self, name: &str) -> Option<&City> {
        let name_lower = name.to_lowercase();
        self.by_name
            .get(&name_lower)
            .and_then(|indices| indices.first())
            .map(|&idx| &self.cities[idx])
    }

    pub fn all(&self) -> &[City] {
        &self.cities
    }

    pub fn count(&self) -> usize {
        self.cities.len()
    }

    pub fn countries(&self) -> Vec<&str> {
        let mut countries: Vec<&str> = self.cities.iter().map(|c| c.country.as_str()).collect();
        countries.sort();
        countries.dedup();
        countries
    }

    pub fn add_city(&mut self, city: City) {
        self.cities.push(city);
        self.build_indexes();
    }

    pub fn load_from_json(&mut self, json: &str) -> Result<usize> {
        let cities: Vec<City> = serde_json::from_str(json)?;
        let count = cities.len();
        self.cities.extend(cities);
        self.build_indexes();
        Ok(count)
    }

    pub fn export_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(&self.cities)?)
    }

    pub fn nearest(&self, lat: f64, lng: f64, limit: usize) -> Vec<&City> {
        let mut with_distance: Vec<(f64, &City)> = self
            .cities
            .iter()
            .map(|city| {
                let distance = haversine_distance(
                    lat,
                    lng,
                    city.coordinates.latitude,
                    city.coordinates.longitude,
                );
                (distance, city)
            })
            .collect();

        with_distance.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        with_distance
            .into_iter()
            .take(limit)
            .map(|(_, city)| city)
            .collect()
    }
}

fn haversine_distance(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    use std::f64::consts::PI;

    const EARTH_RADIUS_KM: f64 = 6371.0;

    let lat1_rad = lat1 * PI / 180.0;
    let lat2_rad = lat2 * PI / 180.0;
    let delta_lat = (lat2 - lat1) * PI / 180.0;
    let delta_lng = (lng2 - lng1) * PI / 180.0;

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lng / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_KM * c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinates_validation() {
        let coords = Coordinates::new(45.0, -90.0);
        assert!(coords.is_valid());

        let invalid = Coordinates::new(100.0, 200.0);
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_city_display_name() {
        let mut city = City::new(
            "London".into(),
            "United Kingdom".into(),
            "GB".into(),
            51.5074,
            -0.1278,
            "Europe/London".into(),
        );
        assert_eq!(city.display_name(), "London, United Kingdom");

        city.region = Some("England".into());
        assert_eq!(city.display_name(), "London, England, United Kingdom");
    }

    #[test]
    fn test_location_manager() {
        let mut manager = LocationManager::new();
        let loc1 = SavedLocation::new(
            "Home".into(),
            Coordinates::new(51.5074, -0.1278),
            "Europe/London".into(),
        );

        manager.add(loc1);
        assert!(manager.current().is_some());
        assert!(manager.default_location().is_some());
    }

    #[test]
    fn test_city_database_search() {
        let db = CityDatabase::new();

        let results = db.search("London");
        assert!(!results.is_empty());

        let makkah = db.get_by_name("Makkah");
        assert!(makkah.is_some());
    }

    #[test]
    fn test_nearest_city() {
        let db = CityDatabase::new();

        let nearest = db.nearest(21.4, 39.8, 3);
        assert!(!nearest.is_empty());
        assert_eq!(nearest[0].name, "Makkah");
    }

    #[test]
    fn test_saved_location_from_city() {
        let city = City::new(
            "Makkah".into(),
            "Saudi Arabia".into(),
            "SA".into(),
            21.4225,
            39.8262,
            "Asia/Riyadh".into(),
        );

        let location = SavedLocation::from_city(&city);
        assert!(location.name.contains("Makkah"));
        assert_eq!(location.timezone, "Asia/Riyadh");
    }
}
