use chrono::NaiveDate;
use nano_pray_core::agent;
use nano_pray_core::config::{AppConfig, ReminderConfig};
use serde::Serialize;
use serde_json::json;
use std::env;
use std::process;

type CliResult<T> = Result<T, String>;

#[derive(Debug)]
struct Args {
    values: Vec<String>,
}

impl Args {
    fn new() -> Self {
        Self {
            values: env::args().skip(1).collect(),
        }
    }

    fn command(&self) -> Option<&str> {
        self.values.first().map(String::as_str)
    }

    fn position(&self, index: usize) -> Option<&str> {
        self.values.get(index).map(String::as_str)
    }

    fn flag(&self, name: &str) -> Option<&str> {
        self.values
            .windows(2)
            .find(|pair| pair[0] == name)
            .map(|pair| pair[1].as_str())
    }

    fn bool_flag(&self, name: &str) -> bool {
        self.values.iter().any(|value| value == name)
    }
}

fn main() {
    let args = Args::new();
    let result = run(&args);
    match result {
        Ok(value) => print_json(value),
        Err(error) => {
            print_json(json!({
                "ok": false,
                "error": error,
                "usage": usage(),
            }));
            process::exit(1);
        }
    }
}

fn run(args: &Args) -> CliResult<serde_json::Value> {
    match args.command() {
        Some("times") => {
            let config = AppConfig::load().unwrap_or_default();
            let date = parse_date(args.flag("--date"))?;
            let (lat, lng) = parse_coordinates(args)?;
            to_value(agent::prayer_times(
                &config,
                date,
                lat,
                lng,
                args.flag("--name").map(ToString::to_string),
            ))
        }
        Some("monthly") => {
            let config = AppConfig::load().unwrap_or_default();
            let today = chrono::Local::now().date_naive();
            let year = parse_i32(args.flag("--year"), today.year())?;
            let month = parse_u32(args.flag("--month"), today.month())?;
            let (lat, lng) = parse_coordinates(args)?;
            to_value(agent::monthly_prayer_times(&config, year, month, lat, lng))
        }
        Some("next") => {
            let config = AppConfig::load().unwrap_or_default();
            to_value(agent::next_prayer(&config))
        }
        Some("qibla") => {
            let latitude = required_f64(args.flag("--lat"), "--lat")?;
            let longitude = required_f64(args.flag("--lng"), "--lng")?;
            to_value(agent::qibla_direction(latitude, longitude))
        }
        Some("cities") if args.position(1) == Some("search") => {
            let query = args
                .position(2)
                .or_else(|| args.flag("--query"))
                .ok_or_else(|| "cities search requires a query".to_string())?;
            let limit = parse_usize(args.flag("--limit"), 10)?;
            to_value(Ok(agent::search_cities(query, limit)))
        }
        Some("config") if args.position(1) == Some("get") => {
            let config = AppConfig::load().unwrap_or_default();
            if args.bool_flag("--full") {
                to_value(Ok(config))
            } else {
                Ok(agent::config_summary(&config))
            }
        }
        Some("hijri") => {
            let offset = args
                .flag("--offset-days")
                .map(|value| {
                    value
                        .parse::<i32>()
                        .map_err(|_| "Invalid --offset-days".to_string())
                })
                .transpose()?;
            to_value(Ok(agent::hijri_date(offset)))
        }
        Some("reminders") if args.position(1) == Some("mute") => to_value(agent::set_muted(true)),
        Some("reminders") if args.position(1) == Some("unmute") => {
            to_value(agent::set_muted(false))
        }
        Some("reminders") if args.position(1) == Some("set") => {
            let prayer = args
                .flag("--prayer")
                .ok_or_else(|| "--prayer is required".to_string())?;
            let config = AppConfig::load().unwrap_or_default();
            let mut reminder = config.reminder_for(&prayer.to_lowercase());
            apply_reminder_flags(args, &mut reminder)?;
            to_value(agent::update_reminder(prayer, reminder))
        }
        Some("log") if args.position(1) == Some("complete") => {
            let prayer = args
                .position(2)
                .or_else(|| args.flag("--prayer"))
                .ok_or_else(|| "log complete requires a prayer".to_string())?;
            let date = args
                .flag("--date")
                .map(|value| {
                    NaiveDate::parse_from_str(value, "%Y-%m-%d")
                        .map_err(|_| "Invalid --date".to_string())
                })
                .transpose()?;
            to_value(agent::mark_prayer_completed(prayer, date))
        }
        Some("help") | None => Ok(json!({ "ok": true, "usage": usage() })),
        Some(other) => Err(format!("Unknown command: {other}")),
    }
}

fn apply_reminder_flags(args: &Args, reminder: &mut ReminderConfig) -> CliResult<()> {
    if let Some(enabled) = args.flag("--enabled") {
        reminder.enabled = parse_bool(enabled)?;
    }
    if let Some(value) = args.flag("--before-enabled") {
        reminder.before_enabled = parse_bool(value)?;
    }
    if let Some(value) = args.flag("--minutes-before") {
        reminder.minutes_before = value
            .parse()
            .map_err(|_| "Invalid --minutes-before".to_string())?;
    }
    if let Some(value) = args.flag("--after-enabled") {
        reminder.after_enabled = parse_bool(value)?;
    }
    if let Some(value) = args.flag("--minutes-after") {
        reminder.minutes_after = value
            .parse()
            .map_err(|_| "Invalid --minutes-after".to_string())?;
    }
    if let Some(value) = args.flag("--play-adhan") {
        reminder.play_adhan = parse_bool(value)?;
    }
    if let Some(value) = args.flag("--show-notification") {
        reminder.show_notification = parse_bool(value)?;
    }
    if let Some(value) = args.flag("--volume") {
        reminder.volume = value
            .parse::<f32>()
            .map_err(|_| "Invalid --volume".to_string())?
            .clamp(0.0, 1.0);
    }
    Ok(())
}

fn parse_coordinates(args: &Args) -> CliResult<(Option<f64>, Option<f64>)> {
    let lat = args
        .flag("--lat")
        .map(|value| {
            value
                .parse::<f64>()
                .map_err(|_| "Invalid --lat".to_string())
        })
        .transpose()?;
    let lng = args
        .flag("--lng")
        .map(|value| {
            value
                .parse::<f64>()
                .map_err(|_| "Invalid --lng".to_string())
        })
        .transpose()?;
    Ok((lat, lng))
}

fn parse_date(value: Option<&str>) -> CliResult<NaiveDate> {
    value
        .map(|date| {
            NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| "Invalid --date".to_string())
        })
        .unwrap_or_else(|| Ok(chrono::Local::now().date_naive()))
}

fn required_f64(value: Option<&str>, name: &str) -> CliResult<f64> {
    value
        .ok_or_else(|| format!("{name} is required"))?
        .parse::<f64>()
        .map_err(|_| format!("Invalid {name}"))
}

fn parse_i32(value: Option<&str>, fallback: i32) -> CliResult<i32> {
    value
        .map(|v| v.parse::<i32>().map_err(|_| "Invalid integer".to_string()))
        .unwrap_or(Ok(fallback))
}

fn parse_u32(value: Option<&str>, fallback: u32) -> CliResult<u32> {
    value
        .map(|v| v.parse::<u32>().map_err(|_| "Invalid integer".to_string()))
        .unwrap_or(Ok(fallback))
}

fn parse_usize(value: Option<&str>, fallback: usize) -> CliResult<usize> {
    value
        .map(|v| {
            v.parse::<usize>()
                .map_err(|_| "Invalid integer".to_string())
        })
        .unwrap_or(Ok(fallback))
}

fn parse_bool(value: &str) -> CliResult<bool> {
    match value.trim().to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(format!("Invalid boolean: {value}")),
    }
}

fn to_value<T: Serialize>(result: nano_pray_core::Result<T>) -> CliResult<serde_json::Value> {
    result
        .map_err(|error| error.to_string())
        .and_then(|value| serde_json::to_value(value).map_err(|error| error.to_string()))
}

fn print_json(value: serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string())
    );
}

fn usage() -> &'static str {
    "nano-pray times [--date YYYY-MM-DD] [--lat N --lng N] | monthly --year YYYY --month M | next | qibla --lat N --lng N | cities search QUERY | config get [--full] | hijri [--offset-days N] | reminders mute|unmute|set --prayer NAME ... | log complete PRAYER"
}

use chrono::Datelike;
