use std::error::Error;
use std::fs;
use std::path::Path;

use telltale_core::{Engine, Event, Severity};

use crate::app;
use crate::output;

pub fn run(
    input: &Path,
    min_severity: Option<Severity>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let json = fs::read_to_string(input)?;
    let events: Vec<Event> = serde_json::from_str(&json)?;

    let rules = app::rules_for_current_os();
    if rules.is_empty() {
        return Err(format!("no rules available for {}", std::env::consts::OS).into());
    }

    let mut engine = Engine::new(rules);

    let mut total_alerts = 0u64;
    let mut new_alerts = 0u64;
    let mut suppressed_alerts = 0u64;

    for event in &events {
        for alert in engine.process(event) {
            total_alerts = total_alerts.saturating_add(1);

            if alert.occurrence_count == 1 {
                new_alerts = new_alerts.saturating_add(1);
            }

            if alert.suppressed {
                suppressed_alerts = suppressed_alerts.saturating_add(1);
                continue;
            }

            if should_show(alert.severity, min_severity) {
                output::print_alert(&alert);
            }
        }
    }

    println!(
        "Replayed {} events — {} alerts ({} new, {} suppressed)",
        events.len(),
        total_alerts,
        new_alerts,
        suppressed_alerts
    );

    Ok(())
}

fn should_show(severity: Severity, minimum: Option<Severity>) -> bool {
    match minimum {
        Some(min) => severity_rank(severity) >= severity_rank(min),
        None => true,
    }
}

fn severity_rank(severity: Severity) -> u8 {
    match severity {
        Severity::Info => 0,
        Severity::Warning => 1,
        Severity::Critical => 2,
    }
}
