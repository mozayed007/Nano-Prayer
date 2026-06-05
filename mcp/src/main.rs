use chrono::{Datelike, NaiveDate};
use nano_pray_core::agent;
use nano_pray_core::config::{AppConfig, ReminderConfig};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            continue;
        };
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<Value>(&line) {
            Ok(request) => handle_request(request),
            Err(error) => Some(error_response(Value::Null, -32700, &error.to_string())),
        };

        if let Some(response) = response {
            println!(
                "{}",
                serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
            );
            let _ = io::stdout().flush();
        }
    }
}

fn handle_request(request: Value) -> Option<Value> {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");

    if method.starts_with("notifications/") {
        return None;
    }

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2025-06-18",
            "serverInfo": { "name": "nano-pray-mcp", "version": env!("CARGO_PKG_VERSION") },
            "capabilities": {
                "tools": { "listChanged": false },
                "resources": { "subscribe": false, "listChanged": false },
                "prompts": { "listChanged": false }
            }
        })),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => call_tool(request.get("params").cloned().unwrap_or_default()),
        "resources/list" => Ok(json!({ "resources": resources() })),
        "resources/read" => read_resource(request.get("params").cloned().unwrap_or_default()),
        "prompts/list" => Ok(json!({ "prompts": prompts() })),
        "prompts/get" => get_prompt(request.get("params").cloned().unwrap_or_default()),
        _ => Err(format!("Unsupported MCP method: {method}")),
    };

    Some(match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": value }),
        Err(error) => error_response(id, -32603, &error),
    })
}

fn call_tool(params: Value) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "tools/call requires params.name".to_string())?;
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let config = AppConfig::load().unwrap_or_default();

    let value = match name {
        "get_prayer_times" => {
            let date =
                opt_date(&args, "date")?.unwrap_or_else(|| chrono::Local::now().date_naive());
            let latitude = opt_f64(&args, "latitude")?;
            let longitude = opt_f64(&args, "longitude")?;
            to_value(agent::prayer_times(
                &config,
                date,
                latitude,
                longitude,
                opt_string(&args, "location_name"),
            ))?
        }
        "get_monthly_prayer_times" => {
            let today = chrono::Local::now().date_naive();
            let year = opt_i32(&args, "year")?.unwrap_or(today.year());
            let month = opt_u32(&args, "month")?.unwrap_or(today.month());
            to_value(agent::monthly_prayer_times(
                &config,
                year,
                month,
                opt_f64(&args, "latitude")?,
                opt_f64(&args, "longitude")?,
            ))?
        }
        "get_next_prayer" => to_value(agent::next_prayer(&config))?,
        "get_qibla_direction" => {
            let latitude = req_f64(&args, "latitude")?;
            let longitude = req_f64(&args, "longitude")?;
            to_value(agent::qibla_direction(latitude, longitude))?
        }
        "search_cities" => {
            let query = req_string(&args, "query")?;
            let limit = opt_u64(&args, "limit")?.unwrap_or(10) as usize;
            json!(agent::search_cities(&query, limit))
        }
        "get_config" => agent::config_summary(&config),
        "get_hijri_date" => json!(agent::hijri_date(opt_i32(&args, "offset_days")?)),
        "mute_reminders" => {
            let muted = opt_bool(&args, "muted")?.unwrap_or(true);
            to_value(agent::set_muted(muted))?
        }
        "update_reminder_settings" => {
            let prayer = req_string(&args, "prayer")?;
            let mut reminder = config.reminder_for(&prayer.to_lowercase());
            merge_reminder(&args, &mut reminder)?;
            to_value(agent::update_reminder(&prayer, reminder))?
        }
        "mark_prayer_completed" => {
            let prayer = req_string(&args, "prayer")?;
            to_value(agent::mark_prayer_completed(
                &prayer,
                opt_date(&args, "date")?,
            ))?
        }
        _ => return Err(format!("Unknown tool: {name}")),
    };

    Ok(json!({
        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string()) }],
        "structuredContent": value
    }))
}

fn read_resource(params: Value) -> Result<Value, String> {
    let uri = params
        .get("uri")
        .and_then(Value::as_str)
        .ok_or_else(|| "resources/read requires params.uri".to_string())?;
    let config = AppConfig::load().unwrap_or_default();
    let value = match uri {
        "nanoprayer://schedule/today" => to_value(agent::prayer_times(
            &config,
            chrono::Local::now().date_naive(),
            None,
            None,
            None,
        ))?,
        "nanoprayer://schedule/month" => to_value(agent::current_month(&config))?,
        "nanoprayer://config/current" => agent::config_summary(&config),
        "nanoprayer://locations" => json!(config.locations),
        _ => return Err(format!("Unknown resource: {uri}")),
    };
    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string())
        }]
    }))
}

fn get_prompt(params: Value) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "prompts/get requires params.name".to_string())?;
    match name {
        "daily_prayer_planning" => Ok(json!({
            "description": "Plan a day around the user's prayer schedule.",
            "messages": [{
                "role": "user",
                "content": {
                    "type": "text",
                    "text": "Use NanoPrayer tools to fetch today's prayer times, then help schedule focused work blocks around the next prayers. Ask before changing reminders."
                }
            }]
        })),
        "travel_location_check" => Ok(json!({
            "description": "Check prayer context for travel or a temporary city.",
            "messages": [{
                "role": "user",
                "content": {
                    "type": "text",
                    "text": "Search the destination city, compare prayer times and qibla direction, and suggest whether a temporary location change is needed. Do not modify saved config without confirmation."
                }
            }]
        })),
        _ => Err(format!("Unknown prompt: {name}")),
    }
}

fn tools() -> Vec<Value> {
    vec![
        tool(
            "get_prayer_times",
            "Get prayer times for a date and optional coordinates.",
            json!({
                "type": "object",
                "properties": {
                    "date": { "type": "string", "description": "YYYY-MM-DD. Defaults to today." },
                    "latitude": { "type": "number" },
                    "longitude": { "type": "number" },
                    "location_name": { "type": "string" }
                }
            }),
        ),
        tool(
            "get_monthly_prayer_times",
            "Get a month of prayer times.",
            json!({
                "type": "object",
                "properties": {
                    "year": { "type": "integer" },
                    "month": { "type": "integer", "minimum": 1, "maximum": 12 },
                    "latitude": { "type": "number" },
                    "longitude": { "type": "number" }
                }
            }),
        ),
        tool(
            "get_next_prayer",
            "Get current and next prayer for the saved location.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "get_qibla_direction",
            "Calculate qibla direction from coordinates.",
            json!({
                "type": "object",
                "required": ["latitude", "longitude"],
                "properties": { "latitude": { "type": "number" }, "longitude": { "type": "number" } }
            }),
        ),
        tool(
            "search_cities",
            "Search built-in city data.",
            json!({
                "type": "object",
                "required": ["query"],
                "properties": { "query": { "type": "string" }, "limit": { "type": "integer", "default": 10 } }
            }),
        ),
        tool(
            "get_config",
            "Get a redacted agent-safe config summary.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "get_hijri_date",
            "Get today's Hijri date with optional day offset.",
            json!({
                "type": "object",
                "properties": { "offset_days": { "type": "integer" } }
            }),
        ),
        tool(
            "mute_reminders",
            "Mute or unmute reminders.",
            json!({
                "type": "object",
                "properties": { "muted": { "type": "boolean", "default": true } }
            }),
        ),
        tool(
            "update_reminder_settings",
            "Update reminder settings for a prayer. Use only after user confirmation.",
            json!({
                "type": "object",
                "required": ["prayer"],
                "properties": {
                    "prayer": { "type": "string" },
                    "enabled": { "type": "boolean" },
                    "before_enabled": { "type": "boolean" },
                    "minutes_before": { "type": "integer" },
                    "after_enabled": { "type": "boolean" },
                    "minutes_after": { "type": "integer" },
                    "play_adhan": { "type": "boolean" },
                    "show_notification": { "type": "boolean" },
                    "volume": { "type": "number", "minimum": 0, "maximum": 1 }
                }
            }),
        ),
        tool(
            "mark_prayer_completed",
            "Mark a prayer completed for today or a date.",
            json!({
                "type": "object",
                "required": ["prayer"],
                "properties": { "prayer": { "type": "string" }, "date": { "type": "string" } }
            }),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({ "name": name, "description": description, "inputSchema": input_schema })
}

fn resources() -> Vec<Value> {
    vec![
        resource("nanoprayer://schedule/today", "Today's Prayer Schedule"),
        resource(
            "nanoprayer://schedule/month",
            "Current Month Prayer Schedule",
        ),
        resource(
            "nanoprayer://config/current",
            "Current NanoPrayer Config Summary",
        ),
        resource("nanoprayer://locations", "Saved Locations"),
    ]
}

fn resource(uri: &str, name: &str) -> Value {
    json!({ "uri": uri, "name": name, "mimeType": "application/json" })
}

fn prompts() -> Vec<Value> {
    vec![
        json!({ "name": "daily_prayer_planning", "description": "Plan the day around prayer times." }),
        json!({ "name": "travel_location_check", "description": "Check prayer context for a destination." }),
    ]
}

fn merge_reminder(args: &Value, reminder: &mut ReminderConfig) -> Result<(), String> {
    if let Some(value) = opt_bool(args, "enabled")? {
        reminder.enabled = value;
    }
    if let Some(value) = opt_bool(args, "before_enabled")? {
        reminder.before_enabled = value;
    }
    if let Some(value) = opt_i32(args, "minutes_before")? {
        reminder.minutes_before = value;
    }
    if let Some(value) = opt_bool(args, "after_enabled")? {
        reminder.after_enabled = value;
    }
    if let Some(value) = opt_i32(args, "minutes_after")? {
        reminder.minutes_after = value;
    }
    if let Some(value) = opt_bool(args, "play_adhan")? {
        reminder.play_adhan = value;
    }
    if let Some(value) = opt_bool(args, "show_notification")? {
        reminder.show_notification = value;
    }
    if let Some(value) = opt_f64(args, "volume")? {
        reminder.volume = (value as f32).clamp(0.0, 1.0);
    }
    Ok(())
}

fn to_value<T: serde::Serialize>(result: nano_pray_core::Result<T>) -> Result<Value, String> {
    result
        .map_err(|error| error.to_string())
        .and_then(|value| serde_json::to_value(value).map_err(|error| error.to_string()))
}

fn req_string(args: &Value, key: &str) -> Result<String, String> {
    opt_string(args, key).ok_or_else(|| format!("{key} is required"))
}

fn req_f64(args: &Value, key: &str) -> Result<f64, String> {
    opt_f64(args, key)?.ok_or_else(|| format!("{key} is required"))
}

fn opt_string(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn opt_bool(args: &Value, key: &str) -> Result<Option<bool>, String> {
    args.get(key)
        .map(|value| {
            value
                .as_bool()
                .ok_or_else(|| format!("{key} must be a boolean"))
        })
        .transpose()
}

fn opt_i32(args: &Value, key: &str) -> Result<Option<i32>, String> {
    args.get(key)
        .map(|value| {
            value
                .as_i64()
                .and_then(|n| i32::try_from(n).ok())
                .ok_or_else(|| format!("{key} must be an integer"))
        })
        .transpose()
}

fn opt_u32(args: &Value, key: &str) -> Result<Option<u32>, String> {
    args.get(key)
        .map(|value| {
            value
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| format!("{key} must be an unsigned integer"))
        })
        .transpose()
}

fn opt_u64(args: &Value, key: &str) -> Result<Option<u64>, String> {
    args.get(key)
        .map(|value| {
            value
                .as_u64()
                .ok_or_else(|| format!("{key} must be an unsigned integer"))
        })
        .transpose()
}

fn opt_f64(args: &Value, key: &str) -> Result<Option<f64>, String> {
    args.get(key)
        .map(|value| {
            value
                .as_f64()
                .ok_or_else(|| format!("{key} must be a number"))
        })
        .transpose()
}

fn opt_date(args: &Value, key: &str) -> Result<Option<NaiveDate>, String> {
    args.get(key)
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| format!("{key} must be a string"))
                .and_then(|date| {
                    NaiveDate::parse_from_str(date, "%Y-%m-%d")
                        .map_err(|_| format!("{key} must be YYYY-MM-DD"))
                })
        })
        .transpose()
}

fn error_response(id: Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}
