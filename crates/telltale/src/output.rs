use std::time::SystemTime;

use telltale_core::{Alert, Severity};

pub fn print_alert(alert: &Alert) {
    let (level, color) = match alert.severity {
        Severity::Critical => ("CRITICAL", "\x1b[31m"),
        Severity::Warning => ("WARNING", "\x1b[33m"),
        Severity::Info => ("INFO", "\x1b[36m"),
    };

    let reset = "\x1b[0m";
    let ts = format_timestamp(alert.last_seen);

    println!(
        "{color}[{level}]{reset} {ts} {} ({})",
        alert.title, alert.rule_id
    );
    println!("  {}", alert.description);
    println!("  Action: {}", alert.recommended_action);

    if alert.occurrence_count > 1 {
        println!("  Occurrences: {}", alert.occurrence_count);
    }

    if !alert.fingerprint.is_empty() {
        println!("  Entity: {}", alert.fingerprint);
    }

    println!();
}

fn format_timestamp(ts: SystemTime) -> String {
    match ts.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => format!("{}", d.as_secs()),
        Err(_) => "0".to_string(),
    }
}
