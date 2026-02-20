//! Error types for NanoPrayReminder core library

use thiserror::Error;

/// Main error type for the core library
#[derive(Error, Debug)]
pub enum Error {
    #[error("Prayer calculation error: {0}")]
    PrayerCalculation(String),

    #[error("Invalid coordinates: latitude={lat}, longitude={long}")]
    InvalidCoordinates { lat: f64, long: f64 },

    #[error("Location not found: {0}")]
    LocationNotFound(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Date/time error: {0}")]
    DateTime(String),

    #[error("Reminder scheduling error: {0}")]
    ReminderScheduling(String),

    #[cfg(feature = "audio")]
    #[error("Audio playback error: {0}")]
    Audio(String),

    #[cfg(feature = "network")]
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Type alias for Result with our Error type
pub type Result<T> = std::result::Result<T, Error>;

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Error::Config(err.to_string())
    }
}

impl From<toml::ser::Error> for Error {
    fn from(err: toml::ser::Error) -> Self {
        Error::Config(err.to_string())
    }
}

impl From<confy::ConfyError> for Error {
    fn from(err: confy::ConfyError) -> Self {
        Error::Config(err.to_string())
    }
}
