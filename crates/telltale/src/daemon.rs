use std::error::Error;
use std::sync::mpsc;
use std::thread;

use telltale_core::{Engine, Platform, knowledge};

use crate::output;
use crate::sources;

pub fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let rules = match std::env::consts::OS {
        "linux" => knowledge::linux_rules(),
        "windows" => knowledge::windows_rules(),
        _ => Vec::new(),
    };

    if rules.is_empty() {
        return Err(format!("no rules available for {}", std::env::consts::OS).into());
    }

    let mut engine = Engine::new(rules);
    let mut source = sources::default_source()?;
    let source_name = source.name();

    let (tx, rx) = mpsc::channel();

    eprintln!(
        "telltale daemon started: source={source_name}, rules_loaded={}",
        engine.rule_count()
    );

    thread::spawn(move || {
        if let Err(err) = source.watch(tx) {
            eprintln!("source error ({source_name}): {err}");
        }
    });

    let mut seen_events: u64 = 0;

    for event in rx {
        seen_events = seen_events.saturating_add(1);
        let alerts = engine.process(&event);

        for alert in alerts {
            if !alert.suppressed {
                output::print_alert(&alert);
            }
        }

        if seen_events % 100 == 0 {
            eprintln!(
                "processed_events={seen_events}, platform={}",
                platform_name(event.platform)
            );
        }
    }

    Err("event source closed unexpectedly".into())
}

fn platform_name(platform: Platform) -> &'static str {
    match platform {
        Platform::Windows => "windows",
        Platform::Linux => "linux",
        Platform::MacOS => "macos",
    }
}
