use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use crate::event::{Event, Severity};
use crate::rule::Rule;

#[derive(Debug, Clone)]
pub struct Alert {
    pub rule_id: String,
    pub fingerprint: String,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub recommended_action: String,
    pub first_seen: SystemTime,
    pub last_seen: SystemTime,
    pub occurrence_count: u32,
    pub suppressed: bool,
}

#[derive(Debug, Clone)]
struct AlertState {
    first_seen: SystemTime,
    last_seen: SystemTime,
    occurrence_count: u32,
}

pub struct Engine {
    rules: Vec<Rule>,
    active: HashMap<(String, String), AlertState>,
}

impl Engine {
    pub fn new(rules: Vec<Rule>) -> Self {
        Self {
            rules,
            active: HashMap::new(),
        }
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    pub fn seed_alert_state(
        &mut self,
        rule_id: &str,
        fingerprint: &str,
        first_seen: SystemTime,
        last_seen: SystemTime,
        occurrence_count: u32,
    ) {
        self.active.insert(
            (rule_id.to_string(), fingerprint.to_string()),
            AlertState {
                first_seen,
                last_seen,
                occurrence_count,
            },
        );
    }

    pub fn process(&mut self, event: &Event) -> Vec<Alert> {
        let mut alerts = Vec::new();

        for rule in &self.rules {
            if !rule.matches(event) {
                continue;
            }

            let now = event.timestamp;
            let fingerprint = rule.fingerprint(event);
            let key = (rule.id.to_string(), fingerprint.clone());

            let (first_seen, last_seen, occurrence_count, suppressed) =
                match self.active.get_mut(&key) {
                    Some(state) => {
                        let within_cooldown =
                            within_cooldown(state.last_seen, now, rule.cooldown_secs);
                        state.last_seen = now;
                        state.occurrence_count = state.occurrence_count.saturating_add(1);

                        (
                            state.first_seen,
                            state.last_seen,
                            state.occurrence_count,
                            within_cooldown,
                        )
                    }
                    None => {
                        self.active.insert(
                            key.clone(),
                            AlertState {
                                first_seen: now,
                                last_seen: now,
                                occurrence_count: 1,
                            },
                        );
                        (now, now, 1, false)
                    }
                };

            alerts.push(Alert {
                rule_id: rule.id.to_string(),
                fingerprint,
                severity: rule.severity,
                title: rule.title.to_string(),
                description: rule.description.to_string(),
                recommended_action: rule.recommended_action.to_string(),
                first_seen,
                last_seen,
                occurrence_count,
                suppressed,
            });
        }

        alerts
    }
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::{Duration, SystemTime};

    use crate::event::{Event, Platform, Severity};
    use crate::rule::{Rule, empty_fingerprint};

    use super::Engine;

    fn test_rule() -> Rule {
        Rule {
            id: "linux.test.error",
            platform: Platform::Linux,
            severity: Severity::Warning,
            title: "Test",
            description: "Desc",
            recommended_action: "Act",
            cooldown_secs: 60,
            match_fn: |e| e.message.contains("error"),
            fingerprint_fn: empty_fingerprint,
        }
    }

    fn event_at(base: SystemTime, secs: u64, message: &str) -> Event {
        Event {
            timestamp: base + Duration::from_secs(secs),
            platform: Platform::Linux,
            source: "journald".to_string(),
            event_id: None,
            message: message.to_string(),
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn suppresses_repeat_within_cooldown() {
        let mut engine = Engine::new(vec![test_rule()]);
        let base = SystemTime::UNIX_EPOCH;

        let first = engine.process(&event_at(base, 0, "error happened"));
        assert_eq!(first.len(), 1);
        assert!(!first[0].suppressed);
        assert_eq!(first[0].occurrence_count, 1);

        let second = engine.process(&event_at(base, 10, "error happened again"));
        assert_eq!(second.len(), 1);
        assert!(second[0].suppressed);
        assert_eq!(second[0].occurrence_count, 2);
    }

    #[test]
    fn alerts_again_after_cooldown() {
        let mut engine = Engine::new(vec![test_rule()]);
        let base = SystemTime::UNIX_EPOCH;

        let _ = engine.process(&event_at(base, 0, "error happened"));
        let third = engine.process(&event_at(base, 61, "error happened later"));

        assert_eq!(third.len(), 1);
        assert!(!third[0].suppressed);
        assert_eq!(third[0].occurrence_count, 2);
    }

    #[test]
    fn dedup_uses_rule_and_fingerprint() {
        let rule = Rule {
            id: "linux.service.failed",
            platform: Platform::Linux,
            severity: Severity::Warning,
            title: "Service failed",
            description: "A service failed.",
            recommended_action: "Inspect service logs.",
            cooldown_secs: 300,
            match_fn: |e| e.message.contains("Failed with result"),
            fingerprint_fn: |e| {
                e.metadata
                    .get("entity")
                    .cloned()
                    .unwrap_or_else(String::new)
            },
        };

        let mut engine = Engine::new(vec![rule]);
        let base = SystemTime::UNIX_EPOCH;

        let mut one_meta = HashMap::new();
        one_meta.insert("entity".to_string(), "ssh.service".to_string());
        let one = Event {
            timestamp: base,
            platform: Platform::Linux,
            source: "systemd".to_string(),
            event_id: None,
            message: "Failed with result 'exit-code'".to_string(),
            metadata: one_meta,
        };

        let mut two_meta = HashMap::new();
        two_meta.insert("entity".to_string(), "nginx.service".to_string());
        let two = Event {
            timestamp: base + Duration::from_secs(10),
            platform: Platform::Linux,
            source: "systemd".to_string(),
            event_id: None,
            message: "Failed with result 'exit-code'".to_string(),
            metadata: two_meta,
        };

        let a = engine.process(&one);
        let b = engine.process(&two);

        assert!(!a[0].suppressed);
        assert!(!b[0].suppressed);
        assert_ne!(a[0].fingerprint, b[0].fingerprint);
    }
}
