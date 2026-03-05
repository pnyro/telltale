use std::collections::HashMap;
use std::error::Error;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::SystemTime;

use telltale_core::{Event, Platform};

use super::EventSource;

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

fn parse_line(line: &str) -> Event {
    let (source, message) = extract_source_and_message(line);
    let mut metadata = HashMap::new();
    metadata.insert("raw_line".to_string(), line.to_string());
    metadata.insert("entity".to_string(), source.clone());

    Event {
        timestamp: SystemTime::now(),
        platform: Platform::Linux,
        source,
        event_id: None,
        message,
        metadata,
    }
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
    use super::extract_source_and_message;

    #[test]
    fn parses_source_and_message_from_short_iso_line() {
        let line = "2026-03-05T10:30:00+00:00 host sudo[123]: authentication failure";
        let (source, message) = extract_source_and_message(line);

        assert_eq!(source, "sudo[123]");
        assert_eq!(message, "authentication failure");
    }
}
