use crate::event::{Event, Platform, Severity};
use crate::rule::Rule;

fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

fn default_fingerprint(event: &Event) -> String {
    event
        .metadata
        .get("entity")
        .cloned()
        .unwrap_or_else(|| event.source.clone())
}

fn matches_oom(event: &Event) -> bool {
    contains_ignore_ascii_case(&event.message, "out of memory: killed process")
}

fn matches_ext4(event: &Event) -> bool {
    contains_ignore_ascii_case(&event.message, "ext4-fs error")
}

fn matches_auth(event: &Event) -> bool {
    contains_ignore_ascii_case(&event.message, "authentication failure")
        || contains_ignore_ascii_case(&event.message, "failed password")
}

fn matches_service_failure(event: &Event) -> bool {
    contains_ignore_ascii_case(&event.message, "failed with result")
}

pub fn rules() -> Vec<Rule> {
    vec![
        Rule {
            id: "linux.oom_killer",
            platform: Platform::Linux,
            severity: Severity::Critical,
            title: "Out-of-memory kill detected",
            description: "The kernel killed a process because the system ran out of memory.",
            recommended_action: "Inspect memory usage and consider reducing load or adding RAM.",
            cooldown_secs: 300,
            match_fn: matches_oom,
            fingerprint_fn: default_fingerprint,
        },
        Rule {
            id: "linux.ext4_error",
            platform: Platform::Linux,
            severity: Severity::Critical,
            title: "EXT4 filesystem error detected",
            description: "The filesystem reported an EXT4 error.",
            recommended_action: "Back up important data and run filesystem checks.",
            cooldown_secs: 600,
            match_fn: matches_ext4,
            fingerprint_fn: default_fingerprint,
        },
        Rule {
            id: "linux.auth_failure",
            platform: Platform::Linux,
            severity: Severity::Warning,
            title: "Authentication failures detected",
            description: "Repeated authentication failures may indicate access issues or brute force attempts.",
            recommended_action: "Review authentication logs and verify account security.",
            cooldown_secs: 300,
            match_fn: matches_auth,
            fingerprint_fn: default_fingerprint,
        },
        Rule {
            id: "linux.systemd_service_failure",
            platform: Platform::Linux,
            severity: Severity::Warning,
            title: "Service failure detected",
            description: "A systemd service reported a failure result.",
            recommended_action: "Check unit logs and restart policy for the affected service.",
            cooldown_secs: 300,
            match_fn: matches_service_failure,
            fingerprint_fn: default_fingerprint,
        },
    ]
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::SystemTime;

    use crate::event::{Event, Platform};

    use super::rules;

    #[test]
    fn oom_rule_matches_expected_message() {
        let rules = rules();
        let event = Event {
            timestamp: SystemTime::UNIX_EPOCH,
            platform: Platform::Linux,
            source: "kernel".to_string(),
            event_id: None,
            message: "Out of memory: Killed process 1234 (foo)".to_string(),
            metadata: HashMap::new(),
        };

        assert!(
            rules
                .iter()
                .any(|rule| rule.id == "linux.oom_killer" && rule.matches(&event))
        );
    }
}
