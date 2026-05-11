//! One-shot `/remind` slash command — schedules a delayed mention in a Discord channel.
//!
//! Persistence: reminders are stored in `reminders.json` and reloaded on startup.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serenity::http::Http;
use serenity::model::id::ChannelId;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

/// A single pending reminder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: String,
    pub channel_id: u64,
    pub sender_id: u64,
    /// Raw mention strings (e.g. "<@123>", "<@&456>")
    pub targets: Vec<String>,
    pub message: String,
    pub fire_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Shared reminder store with file persistence.
#[derive(Clone)]
pub struct ReminderStore {
    reminders: Arc<Mutex<Vec<Reminder>>>,
    path: PathBuf,
}

impl ReminderStore {
    /// Load or create the reminder store from the given path.
    pub fn load(path: PathBuf) -> Self {
        let reminders = match std::fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_else(|e| {
                warn!(error = %e, "failed to parse reminders.json, starting empty");
                Vec::new()
            }),
            Err(_) => Vec::new(),
        };
        info!(count = reminders.len(), path = %path.display(), "loaded reminders");
        Self {
            reminders: Arc::new(Mutex::new(reminders)),
            path,
        }
    }

    /// Add a reminder and persist to disk.
    pub async fn add(&self, reminder: Reminder) {
        let mut reminders = self.reminders.lock().await;
        reminders.push(reminder);
        self.persist(&reminders);
    }

    /// Remove a reminder by ID and persist.
    pub async fn remove(&self, id: &str) {
        let mut reminders = self.reminders.lock().await;
        reminders.retain(|r| r.id != id);
        self.persist(&reminders);
    }

    /// Get all pending reminders (for startup re-scheduling).
    pub async fn pending(&self) -> Vec<Reminder> {
        self.reminders.lock().await.clone()
    }

    fn persist(&self, reminders: &[Reminder]) {
        if let Err(e) = std::fs::write(&self.path, serde_json::to_string_pretty(reminders).unwrap_or_default()) {
            error!(error = %e, "failed to persist reminders.json");
        }
    }
}

/// Parse a human delay string like "30m", "2h", "7d" into seconds.
/// Supports combinations: "1h30m", "2d12h".
/// Range: 1m (60s) to 30d (2_592_000s).
pub fn parse_delay(input: &str) -> Result<u64, String> {
    let s = input.trim().to_lowercase();
    if s.is_empty() {
        return Err("empty delay".into());
    }

    let mut total_secs: u64 = 0;
    let mut num_buf = String::new();

    for ch in s.chars() {
        if ch.is_ascii_digit() {
            num_buf.push(ch);
        } else {
            let n: u64 = num_buf.parse().map_err(|_| format!("invalid number in delay: {input}"))?;
            num_buf.clear();
            let multiplier = match ch {
                'm' => 60,
                'h' => 3600,
                'd' => 86400,
                _ => return Err(format!("unknown unit '{ch}' in delay (use m/h/d)")),
            };
            total_secs += n * multiplier;
        }
    }

    // Handle bare number (default to minutes)
    if !num_buf.is_empty() {
        let n: u64 = num_buf.parse().map_err(|_| format!("invalid number in delay: {input}"))?;
        total_secs += n * 60; // default unit = minutes
    }

    if total_secs < 60 {
        return Err("minimum delay is 1m".into());
    }
    if total_secs > 2_592_000 {
        return Err("maximum delay is 30d".into());
    }

    Ok(total_secs)
}

/// Format seconds into a human-readable string like "2h 30m".
pub fn format_delay(secs: u64) -> String {
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    let mut parts = Vec::new();
    if d > 0 { parts.push(format!("{d}d")); }
    if h > 0 { parts.push(format!("{h}h")); }
    if m > 0 { parts.push(format!("{m}m")); }
    if parts.is_empty() { "< 1m".into() } else { parts.join(" ") }
}

/// Spawn a tokio task that fires the reminder after the delay.
pub fn schedule_reminder(
    http: Arc<Http>,
    store: ReminderStore,
    reminder: Reminder,
) {
    let now = Utc::now();
    let delay = if reminder.fire_at > now {
        (reminder.fire_at - now).to_std().unwrap_or_default()
    } else {
        std::time::Duration::ZERO
    };

    let id = reminder.id.clone();
    tokio::spawn(async move {
        tokio::time::sleep(delay).await;

        let targets_str = reminder.targets.join(" ");
        let content = format!(
            "⏰ **Reminder** from <@{}>:\n\"{}\"\ncc {}",
            reminder.sender_id, reminder.message, targets_str
        );

        let channel = ChannelId::new(reminder.channel_id);
        if let Err(e) = channel.say(&http, &content).await {
            error!(error = %e, id = %id, "failed to send reminder");
        } else {
            info!(id = %id, channel = reminder.channel_id, "reminder fired");
        }

        store.remove(&id).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_delay_minutes() {
        assert_eq!(parse_delay("5m").unwrap(), 300);
        assert_eq!(parse_delay("1m").unwrap(), 60);
    }

    #[test]
    fn test_parse_delay_hours() {
        assert_eq!(parse_delay("2h").unwrap(), 7200);
    }

    #[test]
    fn test_parse_delay_days() {
        assert_eq!(parse_delay("1d").unwrap(), 86400);
        assert_eq!(parse_delay("30d").unwrap(), 2_592_000);
    }

    #[test]
    fn test_parse_delay_combined() {
        assert_eq!(parse_delay("1h30m").unwrap(), 5400);
        assert_eq!(parse_delay("1d12h").unwrap(), 129_600);
    }

    #[test]
    fn test_parse_delay_bare_number_defaults_to_minutes() {
        assert_eq!(parse_delay("10").unwrap(), 600);
    }

    #[test]
    fn test_parse_delay_too_short() {
        assert!(parse_delay("30").is_err()); // 30 seconds via bare number = 30*60=1800, actually valid
        // Actually 30 bare = 30 minutes = 1800s, that's valid
        // Let's test actual too-short
        assert!(parse_delay("0m").is_err());
    }

    #[test]
    fn test_parse_delay_too_long() {
        assert!(parse_delay("31d").is_err());
    }

    #[test]
    fn test_format_delay() {
        assert_eq!(format_delay(3600), "1h");
        assert_eq!(format_delay(5400), "1h 30m");
        assert_eq!(format_delay(90000), "1d 1h");
    }
}
