use std::error::Error;
use std::fs;

use telltale_core::{Engine, Severity, Store};

use crate::app;
use crate::output;
use crate::sources;

pub fn run(hours: u64, min_severity: Option<Severity>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let rules = app::rules_for_current_os();
    if rules.is_empty() {
        return Err(format!("no rules available for {}", std::env::consts::OS).into());
    }

    let db_path = app::database_path()?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let store = Store::open(&db_path)?;

    let mut source = sources::default_historical_source()?;
    let source_name = source.name();
    eprintln!(
        "telltale scan started: source={source_name}, hours={hours}, db={}",
        db_path.display()
    );
    let events = source.scan(hours)?;
    let scanned = events.len() as u64;

    let mut engine = Engine::new(rules);

    let mut total_matched = 0u64;
    let mut shown = 0u64;
    let mut new_count = 0u64;
    let mut suppressed_count = 0u64;

    for event in events {
        for alert in engine.process(&event) {
            total_matched = total_matched.saturating_add(1);

            // Always persist regardless of severity filter
            let updated = store.update_alert(&alert)?;
            if !updated {
                store.save_alert(&alert)?;
            }

            let visible = should_show(alert.severity, min_severity);

            if alert.suppressed {
                if visible {
                    suppressed_count = suppressed_count.saturating_add(1);
                }
                continue;
            }

            if visible {
                if !updated {
                    new_count = new_count.saturating_add(1);
                }
                shown = shown.saturating_add(1);
                output::print_alert(&alert);
            }
        }
    }

    let severity_note = match min_severity {
        Some(sev) => format!(" (filtered to {:?}+)", sev),
        None => String::new(),
    };
    println!(
        "Scanned {scanned} events from last {hours}h{severity_note} — {shown} alerts ({new_count} new, {suppressed_count} suppressed)"
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
