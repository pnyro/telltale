use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::Serialize;
use telltale_core::{Engine, Rule, Severity, Store, knowledge, sources};

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub db_exists: bool,
    pub db_path: String,
    pub rules_loaded: usize,
    pub last_checkpoint: Option<String>,
    pub total_alerts: u64,
}

#[derive(Debug, Serialize)]
pub struct AlertResponse {
    pub id: i64,
    pub rule_id: String,
    pub fingerprint: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub recommended_action: String,
    pub first_seen: i64,
    pub last_seen: i64,
    pub occurrence_count: u32,
}

#[derive(Debug, Default, Serialize)]
pub struct AlertCounts {
    pub critical: u64,
    pub warning: u64,
    pub info: u64,
}

#[derive(Debug, Serialize)]
pub struct RuleResponse {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub recommended_action: String,
    pub cooldown_secs: u64,
}

#[derive(Debug, Serialize)]
pub struct ScanResult {
    pub events_scanned: u64,
    pub alerts_found: u64,
    pub new_alerts: u64,
}

#[tauri::command]
pub fn get_status() -> Result<StatusResponse, String> {
    let db_path = database_path().map_err(|err| err.to_string())?;
    let db_exists = db_path.exists();
    let rules_loaded = rules_for_current_os().len();

    let (last_checkpoint, total_alerts) = if db_exists {
        let store = Store::open(&db_path).map_err(|err| err.to_string())?;
        (
            store
                .get_state("last_event_timestamp")
                .map_err(|err| err.to_string())?,
            store.count_alerts().map_err(|err| err.to_string())?,
        )
    } else {
        (None, 0)
    };

    Ok(StatusResponse {
        db_exists,
        db_path: db_path.display().to_string(),
        rules_loaded,
        last_checkpoint,
        total_alerts,
    })
}

#[tauri::command]
pub fn get_recent_alerts(
    limit: usize,
    severity: Option<String>,
) -> Result<Vec<AlertResponse>, String> {
    let db_path = database_path().map_err(|err| err.to_string())?;
    if !db_path.exists() {
        return Ok(Vec::new());
    }

    let severity_filter = match severity {
        Some(value) => Some(parse_severity(&value)?),
        None => None,
    };

    let store = Store::open(&db_path).map_err(|err| err.to_string())?;
    let alerts = store
        .get_recent(limit, severity_filter)
        .map_err(|err| err.to_string())?;

    Ok(alerts
        .into_iter()
        .map(|alert| AlertResponse {
            id: alert.id,
            rule_id: alert.rule_id,
            fingerprint: alert.fingerprint,
            severity: severity_label(alert.severity).to_string(),
            title: alert.title,
            description: alert.description,
            recommended_action: alert.recommended_action,
            first_seen: to_epoch(alert.first_seen),
            last_seen: to_epoch(alert.last_seen),
            occurrence_count: alert.occurrence_count,
        })
        .collect())
}

#[tauri::command]
pub fn get_alert_counts() -> Result<AlertCounts, String> {
    let db_path = database_path().map_err(|err| err.to_string())?;
    if !db_path.exists() {
        return Ok(AlertCounts::default());
    }

    let store = Store::open(&db_path).map_err(|err| err.to_string())?;
    let alerts = store.get_all_alerts().map_err(|err| err.to_string())?;

    let mut counts = AlertCounts::default();
    for alert in alerts {
        match alert.severity {
            Severity::Critical => counts.critical += 1,
            Severity::Warning => counts.warning += 1,
            Severity::Info => counts.info += 1,
        }
    }

    Ok(counts)
}

#[tauri::command]
pub fn get_rules() -> Result<Vec<RuleResponse>, String> {
    let rules = rules_for_current_os();

    Ok(rules
        .into_iter()
        .map(|rule| RuleResponse {
            id: rule.id.to_string(),
            severity: severity_label(rule.severity).to_string(),
            title: rule.title.to_string(),
            description: rule.description.to_string(),
            recommended_action: rule.recommended_action.to_string(),
            cooldown_secs: rule.cooldown_secs,
        })
        .collect())
}

#[tauri::command]
pub fn run_scan(hours: u64, severity: Option<String>) -> Result<ScanResult, String> {
    let rules = rules_for_current_os();
    if rules.is_empty() {
        return Err(format!("no rules available for {}", std::env::consts::OS));
    }

    let min_severity = match severity {
        Some(value) => Some(parse_severity(&value)?),
        None => None,
    };

    let db_path = database_path().map_err(|err| err.to_string())?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let store = Store::open(&db_path).map_err(|err| err.to_string())?;
    let mut engine = Engine::new(rules);

    // Chosen approach: use shared scan sources from telltale-core so CLI and GUI scan the same
    // platform event streams without shelling out to the CLI binary.
    let mut source = sources::default_historical_source().map_err(|err| err.to_string())?;
    let events = source.scan(hours).map_err(|err| err.to_string())?;

    let mut alerts_found = 0u64;
    let mut new_alerts = 0u64;

    for event in &events {
        for alert in engine.process(event) {
            let updated = store.update_alert(&alert).map_err(|err| err.to_string())?;
            if !updated {
                store.save_alert(&alert).map_err(|err| err.to_string())?;
            }

            let visible = should_show(alert.severity, min_severity);
            if alert.suppressed || !visible {
                continue;
            }

            alerts_found = alerts_found.saturating_add(1);
            if !updated {
                new_alerts = new_alerts.saturating_add(1);
            }
        }
    }

    Ok(ScanResult {
        events_scanned: events.len() as u64,
        alerts_found,
        new_alerts,
    })
}

fn rules_for_current_os() -> Vec<Rule> {
    match std::env::consts::OS {
        "linux" => knowledge::linux_rules(),
        "windows" => knowledge::windows_rules(),
        _ => Vec::new(),
    }
}

fn data_dir() -> Result<PathBuf, String> {
    let mut dir = dirs::data_dir().ok_or("failed to resolve data directory")?;
    dir.push("telltale");
    Ok(dir)
}

fn database_path() -> Result<PathBuf, String> {
    let mut path = data_dir()?;
    path.push("telltale.db");
    Ok(path)
}

fn parse_severity(value: &str) -> Result<Severity, String> {
    match value.to_ascii_lowercase().as_str() {
        "critical" => Ok(Severity::Critical),
        "warning" => Ok(Severity::Warning),
        "info" => Ok(Severity::Info),
        _ => Err(format!(
            "invalid severity '{value}', expected critical|warning|info"
        )),
    }
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "critical",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
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

fn to_epoch(ts: SystemTime) -> i64 {
    match ts.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() as i64,
        Err(_) => 0,
    }
}
