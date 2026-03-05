use crate::event::{Event, Platform, Severity};
use crate::rule::Rule;

fn source_eq(event: &Event, expected: &str) -> bool {
    event.source.eq_ignore_ascii_case(expected)
}

fn matches_disk_bad_block(event: &Event) -> bool {
    event.event_id == Some(7) && source_eq(event, "disk")
}

fn matches_ntfs_corruption(event: &Event) -> bool {
    event.event_id == Some(55) && source_eq(event, "ntfs")
}

fn matches_unexpected_shutdown(event: &Event) -> bool {
    event.event_id == Some(6008) && source_eq(event, "eventlog")
}

fn matches_whea_hardware_error(event: &Event) -> bool {
    let whea_event = matches!(event.event_id, Some(17 | 18 | 19 | 20));
    whea_event && source_eq(event, "microsoft-windows-whea-logger")
}

fn matches_bugcheck_summary(event: &Event) -> bool {
    event.event_id == Some(1001) && source_eq(event, "bugcheck")
}

fn metadata_first(event: &Event, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| event.metadata.get(*key))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn fingerprint_disk(event: &Event) -> String {
    metadata_first(event, &["device", "disk", "disk_id", "entity", "computer"])
        .unwrap_or_else(|| event.source.clone())
}

fn fingerprint_ntfs(event: &Event) -> String {
    metadata_first(event, &["volume", "drive", "entity", "computer"])
        .unwrap_or_else(|| event.source.clone())
}

fn fingerprint_system(event: &Event) -> String {
    metadata_first(event, &["computer", "entity"]).unwrap_or_else(|| event.source.clone())
}

fn fingerprint_whea(event: &Event) -> String {
    metadata_first(
        event,
        &["processor", "bank", "apic_id", "entity", "computer"],
    )
    .unwrap_or_else(|| event.source.clone())
}

fn fingerprint_bugcheck(event: &Event) -> String {
    metadata_first(event, &["bugcheck_code", "entity", "computer"])
        .unwrap_or_else(|| event.source.clone())
}

pub fn rules() -> Vec<Rule> {
    vec![
        Rule {
            id: "win.disk.bad_block",
            platform: Platform::Windows,
            severity: Severity::Critical,
            title: "Bad disk block reported",
            description: "Windows logged a bad block on a storage device.",
            recommended_action: "Back up important data immediately and run vendor disk diagnostics.",
            cooldown_secs: 3_600,
            match_fn: matches_disk_bad_block,
            fingerprint_fn: fingerprint_disk,
        },
        Rule {
            id: "win.ntfs.corruption",
            platform: Platform::Windows,
            severity: Severity::Critical,
            title: "NTFS corruption detected",
            description: "NTFS reported a file system consistency error.",
            recommended_action: "Back up data and run chkdsk on the affected volume.",
            cooldown_secs: 1_800,
            match_fn: matches_ntfs_corruption,
            fingerprint_fn: fingerprint_ntfs,
        },
        Rule {
            id: "win.system.unexpected_shutdown",
            platform: Platform::Windows,
            severity: Severity::Warning,
            title: "Unexpected shutdown detected",
            description: "Windows detected that the previous shutdown was unexpected (Event ID 6008).",
            recommended_action: "Check power stability, crash history, and hardware health if this repeats.",
            cooldown_secs: 900,
            match_fn: matches_unexpected_shutdown,
            fingerprint_fn: fingerprint_system,
        },
        Rule {
            id: "win.whea.hardware_error",
            platform: Platform::Windows,
            severity: Severity::Critical,
            title: "Hardware error reported by WHEA",
            description: "WHEA logged a hardware error event.",
            recommended_action: "Inspect CPU, memory, and motherboard health; update firmware and drivers.",
            cooldown_secs: 1_800,
            match_fn: matches_whea_hardware_error,
            fingerprint_fn: fingerprint_whea,
        },
        Rule {
            id: "win.bugcheck.summary",
            platform: Platform::Windows,
            severity: Severity::Critical,
            title: "BugCheck (BSOD) summary detected",
            description: "Windows logged a bugcheck summary event.",
            recommended_action: "Review crash dump details and recent driver or hardware changes.",
            cooldown_secs: 900,
            match_fn: matches_bugcheck_summary,
            fingerprint_fn: fingerprint_bugcheck,
        },
    ]
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::SystemTime;

    use crate::event::{Event, Platform};

    use super::rules;

    fn make_event(source: &str, event_id: u64) -> Event {
        Event {
            timestamp: SystemTime::UNIX_EPOCH,
            platform: Platform::Windows,
            source: source.to_string(),
            event_id: Some(event_id),
            message: "test".to_string(),
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn matches_disk_bad_block_rule() {
        let event = make_event("Disk", 7);
        assert!(
            rules()
                .iter()
                .any(|rule| rule.id == "win.disk.bad_block" && rule.matches(&event))
        );
    }

    #[test]
    fn matches_ntfs_corruption_rule() {
        let event = make_event("Ntfs", 55);
        assert!(
            rules()
                .iter()
                .any(|rule| rule.id == "win.ntfs.corruption" && rule.matches(&event))
        );
    }

    #[test]
    fn matches_unexpected_shutdown_rule() {
        let event = make_event("EventLog", 6008);
        assert!(
            rules()
                .iter()
                .any(|rule| rule.id == "win.system.unexpected_shutdown" && rule.matches(&event))
        );
    }

    #[test]
    fn matches_whea_hardware_error_rule() {
        let event = make_event("Microsoft-Windows-WHEA-Logger", 18);
        assert!(
            rules()
                .iter()
                .any(|rule| rule.id == "win.whea.hardware_error" && rule.matches(&event))
        );
    }

    #[test]
    fn matches_bugcheck_rule() {
        let event = make_event("BugCheck", 1001);
        assert!(
            rules()
                .iter()
                .any(|rule| rule.id == "win.bugcheck.summary" && rule.matches(&event))
        );
    }
}
