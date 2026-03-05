use std::path::Path;
use std::time::{Duration, SystemTime};

use rusqlite::{Connection, OptionalExtension, params};

use crate::engine::Alert;
use crate::event::Severity;

#[derive(Debug, Clone)]
pub struct StoredAlert {
    pub id: i64,
    pub rule_id: String,
    pub fingerprint: String,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub recommended_action: String,
    pub first_seen: SystemTime,
    pub last_seen: SystemTime,
    pub occurrence_count: u32,
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.init()?;
        Ok(store)
    }

    fn init(&self) -> rusqlite::Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS alerts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                rule_id TEXT NOT NULL,
                fingerprint TEXT NOT NULL,
                severity TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                recommended_action TEXT NOT NULL,
                first_seen INTEGER NOT NULL,
                last_seen INTEGER NOT NULL,
                occurrence_count INTEGER NOT NULL DEFAULT 1,
                UNIQUE(rule_id, fingerprint)
            );

            CREATE INDEX IF NOT EXISTS idx_alerts_last_seen ON alerts(last_seen DESC);

            CREATE TABLE IF NOT EXISTS daemon_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )?;

        Ok(())
    }

    pub fn save_alert(&self, alert: &Alert) -> rusqlite::Result<()> {
        self.conn.execute(
            "
            INSERT INTO alerts (
                rule_id, fingerprint, severity, title, description,
                recommended_action, first_seen, last_seen, occurrence_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(rule_id, fingerprint) DO UPDATE SET
                severity = excluded.severity,
                title = excluded.title,
                description = excluded.description,
                recommended_action = excluded.recommended_action,
                last_seen = excluded.last_seen,
                occurrence_count = alerts.occurrence_count + 1
            ",
            params![
                alert.rule_id,
                alert.fingerprint,
                severity_as_str(alert.severity),
                alert.title,
                alert.description,
                alert.recommended_action,
                to_epoch(alert.first_seen),
                to_epoch(alert.last_seen),
                alert.occurrence_count,
            ],
        )?;

        Ok(())
    }

    pub fn update_alert(&self, alert: &Alert) -> rusqlite::Result<bool> {
        let changed = self.conn.execute(
            "
            UPDATE alerts
            SET
                severity = ?1,
                title = ?2,
                description = ?3,
                recommended_action = ?4,
                last_seen = ?5,
                occurrence_count = occurrence_count + 1
            WHERE rule_id = ?6 AND fingerprint = ?7
            ",
            params![
                severity_as_str(alert.severity),
                alert.title,
                alert.description,
                alert.recommended_action,
                to_epoch(alert.last_seen),
                alert.rule_id,
                alert.fingerprint,
            ],
        )?;

        Ok(changed > 0)
    }

    pub fn get_recent(
        &self,
        limit: usize,
        severity_filter: Option<Severity>,
    ) -> rusqlite::Result<Vec<StoredAlert>> {
        let mut alerts = Vec::new();

        match severity_filter {
            Some(severity) => {
                let mut stmt = self.conn.prepare(
                    "
                    SELECT id, rule_id, fingerprint, severity, title, description,
                           recommended_action, first_seen, last_seen, occurrence_count
                    FROM alerts
                    WHERE severity = ?1
                    ORDER BY last_seen DESC
                    LIMIT ?2
                    ",
                )?;

                let rows = stmt
                    .query_map(params![severity_as_str(severity), limit as i64], |row| {
                        map_stored_alert(row)
                    })?;

                for row in rows {
                    alerts.push(row?);
                }
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "
                    SELECT id, rule_id, fingerprint, severity, title, description,
                           recommended_action, first_seen, last_seen, occurrence_count
                    FROM alerts
                    ORDER BY last_seen DESC
                    LIMIT ?1
                    ",
                )?;

                let rows = stmt.query_map(params![limit as i64], |row| map_stored_alert(row))?;

                for row in rows {
                    alerts.push(row?);
                }
            }
        }

        Ok(alerts)
    }

    pub fn get_all_alerts(&self) -> rusqlite::Result<Vec<StoredAlert>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id, rule_id, fingerprint, severity, title, description,
                   recommended_action, first_seen, last_seen, occurrence_count
            FROM alerts
            ORDER BY last_seen DESC
            ",
        )?;

        let rows = stmt.query_map([], |row| map_stored_alert(row))?;
        let mut alerts = Vec::new();
        for row in rows {
            alerts.push(row?);
        }

        Ok(alerts)
    }

    pub fn count_alerts(&self) -> rusqlite::Result<u64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM alerts", [], |row| row.get(0))
    }

    pub fn get_state(&self, key: &str) -> rusqlite::Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT value FROM daemon_state WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn set_state(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "
            INSERT INTO daemon_state (key, value)
            VALUES (?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            ",
            params![key, value],
        )?;

        Ok(())
    }
}

fn map_stored_alert(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredAlert> {
    let severity_raw: String = row.get(3)?;
    let severity = severity_from_str(&severity_raw);

    Ok(StoredAlert {
        id: row.get(0)?,
        rule_id: row.get(1)?,
        fingerprint: row.get(2)?,
        severity,
        title: row.get(4)?,
        description: row.get(5)?,
        recommended_action: row.get(6)?,
        first_seen: from_epoch(row.get(7)?),
        last_seen: from_epoch(row.get(8)?),
        occurrence_count: row.get(9)?,
    })
}

fn severity_as_str(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "critical",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn severity_from_str(value: &str) -> Severity {
    match value {
        "critical" => Severity::Critical,
        "warning" => Severity::Warning,
        "info" => Severity::Info,
        _ => Severity::Info,
    }
}

fn to_epoch(ts: SystemTime) -> i64 {
    match ts.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() as i64,
        Err(_) => 0,
    }
}

fn from_epoch(secs: i64) -> SystemTime {
    if secs <= 0 {
        return SystemTime::UNIX_EPOCH;
    }

    SystemTime::UNIX_EPOCH + Duration::from_secs(secs as u64)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::SystemTime;

    use crate::engine::Alert;

    use super::Store;
    use crate::event::Severity;

    fn temp_db_path() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("telltale-store-{nanos}.db"))
    }

    fn make_alert() -> Alert {
        Alert {
            rule_id: "test.rule".to_string(),
            fingerprint: "entity-a".to_string(),
            severity: Severity::Warning,
            title: "title".to_string(),
            description: "description".to_string(),
            recommended_action: "action".to_string(),
            first_seen: SystemTime::UNIX_EPOCH,
            last_seen: SystemTime::UNIX_EPOCH,
            occurrence_count: 1,
            suppressed: false,
        }
    }

    #[test]
    fn saves_and_updates_alerts() {
        let path = temp_db_path();
        let store = Store::open(&path).expect("open store");
        let alert = make_alert();

        store.save_alert(&alert).expect("save alert");
        let updated = store.update_alert(&alert).expect("update alert");
        assert!(updated);

        let alerts = store.get_recent(10, None).expect("get alerts");
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].occurrence_count, 2);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reads_and_writes_daemon_state() {
        let path = temp_db_path();
        let store = Store::open(&path).expect("open store");

        store
            .set_state("last_event_timestamp", "12345")
            .expect("set state");
        let value = store
            .get_state("last_event_timestamp")
            .expect("get state")
            .expect("state value");

        assert_eq!(value, "12345");

        let _ = std::fs::remove_file(path);
    }
}
