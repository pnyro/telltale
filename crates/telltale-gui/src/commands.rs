use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Serialize;
use telltale_core::{Engine, Rule, Severity, Store, StoredAlert, knowledge, sources};

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

#[derive(Debug, Default, Serialize, Clone, PartialEq, Eq)]
pub struct AlertCounts {
    pub critical: u64,
    pub warning: u64,
    pub info: u64,
}

#[derive(Debug, Serialize)]
pub struct DashboardOverviewResponse {
    pub db_exists: bool,
    pub db_path: String,
    pub rules_loaded: usize,
    pub total_alerts: u64,
    pub last_checkpoint: Option<String>,
    pub last_alert_at: Option<i64>,
    pub health_state: String,
    pub counts: AlertCounts,
    pub recent_alerts: Vec<AlertResponse>,
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
pub fn get_dashboard_overview() -> Result<DashboardOverviewResponse, String> {
    let db_path = database_path().map_err(|err| err.to_string())?;
    let rules_loaded = rules_for_current_os().len();

    build_dashboard_overview_for_path(&db_path, rules_loaded)
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

    Ok(alerts.into_iter().map(alert_response).collect())
}

#[tauri::command]
pub fn get_alert_counts() -> Result<AlertCounts, String> {
    let db_path = database_path().map_err(|err| err.to_string())?;
    if !db_path.exists() {
        return Ok(AlertCounts::default());
    }

    let store = Store::open(&db_path).map_err(|err| err.to_string())?;
    let alerts = store.get_all_alerts().map_err(|err| err.to_string())?;

    Ok(count_alerts_by_severity(&alerts))
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

fn build_dashboard_overview_for_path(
    db_path: &Path,
    rules_loaded: usize,
) -> Result<DashboardOverviewResponse, String> {
    let db_exists = db_path.exists();
    let db_path_display = db_path.display().to_string();

    if !db_exists {
        return Ok(DashboardOverviewResponse {
            db_exists,
            db_path: db_path_display,
            rules_loaded,
            total_alerts: 0,
            last_checkpoint: None,
            last_alert_at: None,
            health_state: "empty".to_string(),
            counts: AlertCounts::default(),
            recent_alerts: Vec::new(),
        });
    }

    let store = Store::open(db_path).map_err(|err| err.to_string())?;
    let alerts = store.get_all_alerts().map_err(|err| err.to_string())?;
    let counts = count_alerts_by_severity(&alerts);
    let total_alerts = alerts.len() as u64;
    let last_checkpoint = store
        .get_state("last_event_timestamp")
        .map_err(|err| err.to_string())?;
    let last_alert_at = alerts.first().map(|alert| to_epoch(alert.last_seen));
    let recent_alerts = alerts.iter().take(6).cloned().map(alert_response).collect();

    Ok(DashboardOverviewResponse {
        db_exists,
        db_path: db_path_display,
        rules_loaded,
        total_alerts,
        last_checkpoint,
        last_alert_at,
        health_state: dashboard_health_state(total_alerts, &counts).to_string(),
        counts,
        recent_alerts,
    })
}

fn count_alerts_by_severity(alerts: &[StoredAlert]) -> AlertCounts {
    let mut counts = AlertCounts::default();

    for alert in alerts {
        match alert.severity {
            Severity::Critical => counts.critical += 1,
            Severity::Warning => counts.warning += 1,
            Severity::Info => counts.info += 1,
        }
    }

    counts
}

fn dashboard_health_state(total_alerts: u64, counts: &AlertCounts) -> &'static str {
    if total_alerts == 0 {
        return "empty";
    }

    if counts.critical > 0 {
        return "critical";
    }

    if counts.warning > 0 {
        return "warning";
    }

    "quiet"
}

fn alert_response(alert: StoredAlert) -> AlertResponse {
    AlertResponse {
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
    }
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};

    use telltale_core::{Alert, Severity, Store};

    use super::build_dashboard_overview_for_path;

    fn temp_db_path() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("telltale-gui-overview-{nanos}.db"))
    }

    fn make_alert(rule_id: &str, fingerprint: &str, severity: Severity, secs: u64) -> Alert {
        let seen_at = SystemTime::UNIX_EPOCH + Duration::from_secs(secs);

        Alert {
            rule_id: rule_id.to_string(),
            fingerprint: fingerprint.to_string(),
            severity,
            title: format!("{rule_id} title"),
            description: format!("{rule_id} description"),
            recommended_action: "Investigate the source and remediate.".to_string(),
            first_seen: seen_at,
            last_seen: seen_at,
            occurrence_count: 1,
            suppressed: false,
        }
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn missing_db_returns_empty_overview() {
        let path = temp_db_path();

        let overview = build_dashboard_overview_for_path(&path, 5).expect("overview");

        assert!(!overview.db_exists);
        assert_eq!(overview.health_state, "empty");
        assert_eq!(overview.total_alerts, 0);
        assert!(overview.recent_alerts.is_empty());

        cleanup(&path);
    }

    #[test]
    fn empty_db_returns_empty_health_state() {
        let path = temp_db_path();
        let _store = Store::open(&path).expect("open store");

        let overview = build_dashboard_overview_for_path(&path, 5).expect("overview");

        assert!(overview.db_exists);
        assert_eq!(overview.health_state, "empty");
        assert_eq!(overview.total_alerts, 0);

        cleanup(&path);
    }

    #[test]
    fn info_only_returns_quiet() {
        let path = temp_db_path();
        let store = Store::open(&path).expect("open store");
        store
            .save_alert(&make_alert("info.rule", "disk-0", Severity::Info, 12))
            .expect("save alert");

        let overview = build_dashboard_overview_for_path(&path, 5).expect("overview");

        assert_eq!(overview.health_state, "quiet");
        assert_eq!(overview.counts.info, 1);
        assert_eq!(overview.counts.warning, 0);
        assert_eq!(overview.counts.critical, 0);

        cleanup(&path);
    }

    #[test]
    fn warning_present_returns_warning() {
        let path = temp_db_path();
        let store = Store::open(&path).expect("open store");
        store
            .save_alert(&make_alert("info.rule", "disk-0", Severity::Info, 12))
            .expect("save alert");
        store
            .save_alert(&make_alert("warning.rule", "svc-a", Severity::Warning, 18))
            .expect("save alert");

        let overview = build_dashboard_overview_for_path(&path, 5).expect("overview");

        assert_eq!(overview.health_state, "warning");
        assert_eq!(overview.counts.info, 1);
        assert_eq!(overview.counts.warning, 1);
        assert_eq!(overview.counts.critical, 0);

        cleanup(&path);
    }

    #[test]
    fn critical_present_returns_critical() {
        let path = temp_db_path();
        let store = Store::open(&path).expect("open store");
        store
            .save_alert(&make_alert("warning.rule", "svc-a", Severity::Warning, 18))
            .expect("save alert");
        store
            .save_alert(&make_alert("critical.rule", "disk-2", Severity::Critical, 25))
            .expect("save alert");

        let overview = build_dashboard_overview_for_path(&path, 5).expect("overview");

        assert_eq!(overview.health_state, "critical");
        assert_eq!(overview.counts.critical, 1);

        cleanup(&path);
    }

    #[test]
    fn latest_alert_timestamp_and_recent_order_are_preserved() {
        let path = temp_db_path();
        let store = Store::open(&path).expect("open store");
        store
            .save_alert(&make_alert("older.rule", "disk-0", Severity::Info, 10))
            .expect("save alert");
        store
            .save_alert(&make_alert("newer.rule", "disk-1", Severity::Warning, 95))
            .expect("save alert");

        let overview = build_dashboard_overview_for_path(&path, 5).expect("overview");

        assert_eq!(overview.last_alert_at, Some(95));
        assert_eq!(overview.total_alerts, 2);
        assert_eq!(overview.recent_alerts.len(), 2);
        assert_eq!(overview.recent_alerts[0].rule_id, "newer.rule");
        assert_eq!(overview.recent_alerts[1].rule_id, "older.rule");

        cleanup(&path);
    }
}
