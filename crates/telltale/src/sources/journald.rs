use std::collections::HashMap;
use std::error::Error;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, SystemTime};

use telltale_core::{Event, Platform};

use super::{EventSource, HistoricalEventSource};

pub struct JournaldSource;

impl JournaldSource {
    pub fn new() -> Self {
        Self
    }
}

impl EventSource for JournaldSource {
    fn name(&self) -> &'static str {
        "journald"
    }

    fn watch(&mut self, sender: mpsc::Sender<Event>) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut child = Command::new("journalctl")
            .args(["-f", "-n", "0", "-o", "short-iso", "--no-pager"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child
            .stdout
            .take()
            .ok_or("failed to capture journalctl stdout")?;
        let reader = BufReader::new(stdout);

        for line_result in reader.lines() {
            let line = line_result?;
            if line.trim().is_empty() {
                continue;
            }

            let event = parse_line(&line);
            if sender.send(event).is_err() {
                break;
            }
        }

        let status = child.wait()?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("journalctl exited with status {status}").into())
        }
    }
}

impl HistoricalEventSource for JournaldSource {
    fn name(&self) -> &'static str {
        "journald"
    }

    fn scan(&mut self, hours: u64) -> Result<Vec<Event>, Box<dyn Error + Send + Sync>> {
        let since = format!("{hours} hours ago");
        let output = Command::new("journalctl")
            .args(["--since", &since, "-o", "short-iso", "--no-pager"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "journalctl exited with status {}: {}",
                output.status, stderr
            )
            .into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut events = Vec::new();
        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            events.push(parse_line(line));
        }

        Ok(events)
    }
}

fn parse_line(line: &str) -> Event {
    let timestamp = parse_iso_timestamp(line).unwrap_or_else(SystemTime::now);
    let (source, message) = extract_source_and_message(line);
    let mut metadata = HashMap::new();
    metadata.insert("raw_line".to_string(), line.to_string());
    metadata.insert("entity".to_string(), source.clone());

    Event {
        timestamp,
        platform: Platform::Linux,
        source,
        event_id: None,
        message,
        metadata,
    }
}

/// Parse the ISO 8601 timestamp at the start of a journald short-iso line.
/// Format: "2026-03-05T10:30:00+0000" or "2026-03-05T10:30:00+00:00"
fn parse_iso_timestamp(line: &str) -> Option<SystemTime> {
    // First token is the timestamp
    let ts_str = line.split_whitespace().next()?;
    if ts_str.len() < 19 || ts_str.as_bytes().get(10) != Some(&b'T') {
        return None;
    }

    let year: u64 = ts_str[0..4].parse().ok()?;
    let month: u64 = ts_str[5..7].parse().ok()?;
    let day: u64 = ts_str[8..10].parse().ok()?;
    let hour: u64 = ts_str[11..13].parse().ok()?;
    let min: u64 = ts_str[14..16].parse().ok()?;
    let sec: u64 = ts_str[17..19].parse().ok()?;

    // Simple days-from-epoch (same approach as windows.rs)
    let days_in_months = [0u64, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let is_leap = |y: u64| (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;

    let mut days: u64 = 0;
    for y in 1970..year {
        days += if is_leap(y) { 366 } else { 365 };
    }
    for m in 1..month {
        days += days_in_months[m as usize];
        if m == 2 && is_leap(year) {
            days += 1;
        }
    }
    days += day - 1;

    let total_secs = days * 86400 + hour * 3600 + min * 60 + sec;

    // Parse timezone offset if present (e.g., +0200, +02:00, -0500)
    let after_time = &ts_str[19..];
    let offset_secs: i64 = if after_time.is_empty() || after_time == "Z" {
        0
    } else if let Some(rest) = after_time
        .strip_prefix('+')
        .or_else(|| after_time.strip_prefix('-'))
    {
        let sign: i64 = if after_time.starts_with('-') { -1 } else { 1 };
        let digits: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() >= 4 {
            let oh: i64 = digits[0..2].parse().ok()?;
            let om: i64 = digits[2..4].parse().ok()?;
            sign * (oh * 3600 + om * 60)
        } else {
            0
        }
    } else {
        0
    };

    let utc_secs = (total_secs as i64) - offset_secs;
    if utc_secs < 0 {
        return None;
    }

    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(utc_secs as u64))
}

fn extract_source_and_message(line: &str) -> (String, String) {
    let rest = line
        .split_whitespace()
        .skip(2)
        .collect::<Vec<_>>()
        .join(" ");
    let rest = if rest.is_empty() { line } else { rest.as_str() }.trim();

    if let Some((prefix, msg)) = rest.split_once(':') {
        let source = prefix.trim().to_string();
        let message = msg.trim().to_string();

        if !source.is_empty() && !message.is_empty() {
            return (source, message);
        }
    }

    ("journald".to_string(), rest.to_string())
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::{extract_source_and_message, parse_iso_timestamp};

    #[test]
    fn parses_source_and_message_from_short_iso_line() {
        let line = "2026-03-05T10:30:00+00:00 host sudo[123]: authentication failure";
        let (source, message) = extract_source_and_message(line);

        assert_eq!(source, "sudo[123]");
        assert_eq!(message, "authentication failure");
    }

    #[test]
    fn parses_utc_timestamp() {
        let line = "2026-03-05T10:30:00+0000 host kernel: test";
        let ts = parse_iso_timestamp(line).unwrap();
        let epoch_secs = ts.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        // 2026-03-05 = 20517 days, 10:30:00 = 37800s
        assert_eq!(epoch_secs, 20517 * 86400 + 37800);
    }

    #[test]
    fn parses_timestamp_with_positive_offset() {
        let line = "2026-03-05T12:30:00+0200 host kernel: test";
        let ts = parse_iso_timestamp(line).unwrap();
        let epoch_secs = ts.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        // 12:30 local at +0200 = 10:30 UTC
        assert_eq!(epoch_secs, 20517 * 86400 + 37800);
    }

    #[test]
    fn parses_timestamp_with_colon_offset() {
        let line = "2026-03-05T10:30:00+00:00 host kernel: test";
        let ts = parse_iso_timestamp(line).unwrap();
        let epoch_secs = ts.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        assert_eq!(epoch_secs, 20517 * 86400 + 37800);
    }
}
