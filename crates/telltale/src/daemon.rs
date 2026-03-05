use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime};

use telltale_core::{Engine, Platform, Store};

use crate::app;
use crate::notify;
use crate::output;
use crate::sources;
use crate::sources::EventSource;

const CHECKPOINT_KEY: &str = "last_event_timestamp";

pub fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    run_with_source(sources::default_source()?, false)
}

pub fn run_simulated(interval_secs: u64, count: u64) -> Result<(), Box<dyn Error + Send + Sync>> {
    run_with_source(
        sources::simulated_source(Duration::from_secs(interval_secs), count)?,
        count != 0,
    )
}

fn run_with_source(
    mut source: Box<dyn EventSource>,
    allow_clean_source_close: bool,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let rules = app::rules_for_current_os();

    if rules.is_empty() {
        return Err(format!("no rules available for {}", std::env::consts::OS).into());
    }

    let rule_cooldowns: HashMap<String, u64> = rules
        .iter()
        .map(|rule| (rule.id.to_string(), rule.cooldown_secs))
        .collect();

    let mut engine = Engine::new(rules);

    let db_path = app::database_path()?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let store = Store::open(&db_path)?;

    let restored = seed_engine_from_store(&mut engine, &store, &rule_cooldowns)?;
    let source_name = source.name();
    let notifier = notify::default_notifier();

    let (tx, rx) = mpsc::channel();
    let (source_state_tx, source_state_rx) = mpsc::sync_channel::<Result<(), String>>(1);

    eprintln!(
        "telltale daemon started: source={source_name}, rules_loaded={}, restored_alerts={}, db={}",
        engine.rule_count(),
        restored,
        db_path.display()
    );

    thread::spawn(move || {
        let source_result = source.watch(tx).map_err(|err| err.to_string());
        let _ = source_state_tx.send(source_result);
    });

    let mut seen_events: u64 = 0;

    while let Ok(event) = rx.recv() {
        seen_events = seen_events.saturating_add(1);
        let checkpoint_value = timestamp_to_string(event.timestamp);

        let alerts = engine.process(&event);

        for alert in alerts {
            let updated = store.update_alert(&alert)?;
            if !updated {
                store.save_alert(&alert)?;
            }

            if !alert.suppressed {
                output::print_alert(&alert);

                if notify::is_notifiable_severity(alert.severity) {
                    if let Err(err) = notifier.notify(&alert) {
                        eprintln!("failed to emit notification: {err}");
                    }
                }
            }
        }

        if seen_events % 25 == 0 {
            store.set_state(CHECKPOINT_KEY, &checkpoint_value)?;
        }

        if seen_events % 100 == 0 {
            eprintln!(
                "processed_events={seen_events}, platform={}, checkpoint={checkpoint_value}",
                platform_name(event.platform)
            );
        }
    }

    match source_state_rx.recv() {
        Ok(Ok(())) if allow_clean_source_close => Ok(()),
        Ok(Ok(())) => Err("event source closed unexpectedly".into()),
        Ok(Err(source_err)) => Err(format!("source error ({source_name}): {source_err}").into()),
        Err(_) => Err(format!("source thread ended without status ({source_name})").into()),
    }
}

fn seed_engine_from_store(
    engine: &mut Engine,
    store: &Store,
    rule_cooldowns: &HashMap<String, u64>,
) -> Result<usize, Box<dyn Error + Send + Sync>> {
    let now = SystemTime::now();
    let alerts = store.get_all_alerts()?;
    let mut restored = 0usize;

    for alert in alerts {
        let cooldown_secs = match rule_cooldowns.get(&alert.rule_id) {
            Some(value) => *value,
            None => continue,
        };

        if within_cooldown(alert.last_seen, now, cooldown_secs) {
            engine.seed_alert_state(
                &alert.rule_id,
                &alert.fingerprint,
                alert.first_seen,
                alert.last_seen,
                alert.occurrence_count,
            );
            restored = restored.saturating_add(1);
        }
    }

    Ok(restored)
}

fn within_cooldown(last_seen: SystemTime, now: SystemTime, cooldown_secs: u64) -> bool {
    if cooldown_secs == 0 {
        return false;
    }

    match now.duration_since(last_seen) {
        Ok(elapsed) => elapsed < Duration::from_secs(cooldown_secs),
        Err(_) => true,
    }
}

fn timestamp_to_string(ts: SystemTime) -> String {
    match ts.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs().to_string(),
        Err(_) => "0".to_string(),
    }
}

fn platform_name(platform: Platform) -> &'static str {
    match platform {
        Platform::Windows => "windows",
        Platform::Linux => "linux",
        Platform::MacOS => "macos",
    }
}
