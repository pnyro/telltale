use crate::event::{Event, Platform, Severity};
use crate::rule::{empty_fingerprint, Rule};

fn source_eq(event: &Event, expected: &str) -> bool {
    event.source.eq_ignore_ascii_case(expected)
}

fn metadata_first(event: &Event, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| event.metadata.get(*key))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn extract_between_case_insensitive<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let haystack = text.to_ascii_lowercase();
    let start_lc = start.to_ascii_lowercase();
    let end_lc = end.to_ascii_lowercase();

    let start_idx = haystack.find(&start_lc)?;
    let content_start = start_idx + start.len();
    let end_rel = haystack[content_start..].find(&end_lc)?;
    let content_end = content_start + end_rel;

    let value = text[content_start..content_end].trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn extract_kb_or_package_identifier(message: &str) -> Option<String> {
    for token in message.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')) {
        if token.is_empty() {
            continue;
        }

        let upper = token.to_ascii_uppercase();
        if upper.starts_with("KB") && upper[2..].chars().all(|ch| ch.is_ascii_digit()) {
            return Some(upper);
        }

        if token.starts_with("Package_for_") {
            return Some(token.to_string());
        }
    }

    None
}

fn extract_volume_letter(message: &str) -> Option<String> {
    let bytes = message.as_bytes();
    for pair in bytes.windows(2) {
        if pair[0].is_ascii_alphabetic() && pair[1] == b':' {
            return Some(format!("{}:", (pair[0] as char).to_ascii_uppercase()));
        }
    }
    None
}

fn extract_domain_name(message: &str) -> Option<String> {
    if let Some(domain) = extract_between_case_insensitive(message, "for the name ", " timed out") {
        let cleaned =
            domain.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-');
        if !cleaned.is_empty() {
            return Some(cleaned.to_string());
        }
    }

    for token in message.split_whitespace() {
        let cleaned =
            token.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-');
        if cleaned.contains('.') {
            return Some(cleaned.to_string());
        }
    }

    None
}

fn extract_dotnet_app_name(message: &str) -> Option<String> {
    if let Some(name) =
        extract_between_case_insensitive(message, "application:", " framework version")
    {
        return Some(name.to_string());
    }

    if let Some(tail) = message
        .to_ascii_lowercase()
        .find("application:")
        .map(|idx| &message[idx + "application:".len()..])
    {
        let token = tail
            .trim_start()
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-' && ch != '_'
            });
        if !token.is_empty() {
            return Some(token.to_string());
        }
    }

    None
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

fn matches_kernel_power_dirty_reboot(event: &Event) -> bool {
    event.event_id == Some(41) && source_eq(event, "Microsoft-Windows-Kernel-Power")
}

fn matches_tcpip_port_exhaustion(event: &Event) -> bool {
    matches!(event.event_id, Some(4231 | 4266)) && source_eq(event, "Tcpip")
}

fn matches_application_crash(event: &Event) -> bool {
    event.event_id == Some(1000) && source_eq(event, "Application Error")
}

fn matches_application_hang(event: &Event) -> bool {
    event.event_id == Some(1002) && source_eq(event, "Application Hang")
}

fn matches_windows_update_install_failure(event: &Event) -> bool {
    event.event_id == Some(20) && source_eq(event, "Microsoft-Windows-WindowsUpdateClient")
}

fn matches_volsnap_shadow_copy_failure(event: &Event) -> bool {
    event.event_id == Some(36) && source_eq(event, "Volsnap")
}

fn matches_service_dependency_failure(event: &Event) -> bool {
    event.event_id == Some(7001) && source_eq(event, "Service Control Manager")
}

fn matches_vss_error(event: &Event) -> bool {
    event.event_id == Some(8193) && source_eq(event, "VSS")
}

fn matches_dns_timeout(event: &Event) -> bool {
    event.event_id == Some(1014) && source_eq(event, "Microsoft-Windows-DNS-Client")
}

fn matches_dotnet_unhandled_exception(event: &Event) -> bool {
    event.event_id == Some(1026) && source_eq(event, ".NET Runtime")
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

fn fingerprint_tcpip_transport(event: &Event) -> String {
    match event.event_id {
        Some(4231) => "tcp".to_string(),
        Some(4266) => "udp".to_string(),
        _ => String::new(),
    }
}

fn fingerprint_application_name(event: &Event) -> String {
    metadata_first(event, &["device"]).unwrap_or_default()
}

fn fingerprint_update_identifier(event: &Event) -> String {
    extract_kb_or_package_identifier(&event.message).unwrap_or_default()
}

fn fingerprint_volsnap_volume(event: &Event) -> String {
    extract_volume_letter(&event.message).unwrap_or_default()
}

fn fingerprint_service_name(event: &Event) -> String {
    if let Some(name) =
        extract_between_case_insensitive(&event.message, "The ", " service depends on")
    {
        return name.to_string();
    }

    metadata_first(event, &["device"]).unwrap_or_default()
}

fn fingerprint_dns_domain(event: &Event) -> String {
    extract_domain_name(&event.message).unwrap_or_default()
}

fn fingerprint_dotnet_app(event: &Event) -> String {
    extract_dotnet_app_name(&event.message)
        .or_else(|| metadata_first(event, &["device"]))
        .unwrap_or_default()
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
        Rule {
            id: "win.kernel_power.dirty_reboot",
            platform: Platform::Windows,
            severity: Severity::Critical,
            title: "Unexpected reboot (Kernel-Power)",
            description: "The system rebooted without cleanly shutting down. This is the kernel-level companion to Event 6008 and indicates a crash, power loss, or forced reset.",
            recommended_action: "Check for recent BSODs, verify power supply stability, and review crash dumps in C:\\Windows\\Minidump.",
            cooldown_secs: 3_600,
            match_fn: matches_kernel_power_dirty_reboot,
            fingerprint_fn: empty_fingerprint,
        },
        Rule {
            id: "win.tcpip.port_exhaustion",
            platform: Platform::Windows,
            severity: Severity::Warning,
            title: "Ephemeral port exhaustion",
            description: "All available ephemeral ports are in use. Applications may fail to make new network connections until ports are freed.",
            recommended_action: "Identify connections with netstat -ano, check for connection leaks, or increase the dynamic port range with netsh int ipv4 set dynamic tcp start=10000 num=55536.",
            cooldown_secs: 300,
            match_fn: matches_tcpip_port_exhaustion,
            fingerprint_fn: fingerprint_tcpip_transport,
        },
        Rule {
            id: "win.app.crash",
            platform: Platform::Windows,
            severity: Severity::Warning,
            title: "Application crash",
            description: "An application terminated unexpectedly due to an unhandled exception or access violation.",
            recommended_action: "Check the faulting module listed in the event for updates or known issues. If this repeats for the same application, consider reinstalling it.",
            cooldown_secs: 300,
            match_fn: matches_application_crash,
            fingerprint_fn: fingerprint_application_name,
        },
        Rule {
            id: "win.app.hang",
            platform: Platform::Windows,
            severity: Severity::Info,
            title: "Application stopped responding",
            description: "An application stopped responding and was closed by Windows.",
            recommended_action: "If this happens frequently for the same application, check for updates, reduce system load, or increase available memory.",
            cooldown_secs: 600,
            match_fn: matches_application_hang,
            fingerprint_fn: fingerprint_application_name,
        },
        Rule {
            id: "win.update.install_failure",
            platform: Platform::Windows,
            severity: Severity::Warning,
            title: "Windows Update installation failed",
            description: "A Windows Update failed to install. Repeated failures may leave security patches unapplied.",
            recommended_action: "Go to Settings > Windows Update and retry. If the error persists, search the error code online or run sfc /scannow.",
            cooldown_secs: 3_600,
            match_fn: matches_windows_update_install_failure,
            fingerprint_fn: fingerprint_update_identifier,
        },
        Rule {
            id: "win.volsnap.shadow_copy_failed",
            platform: Platform::Windows,
            severity: Severity::Warning,
            title: "Shadow copy storage limit reached",
            description: "Shadow copies (used by System Restore and backup) were aborted because storage ran out or hit a size limit.",
            recommended_action: "Increase shadow copy storage with vssadmin resize shadowstorage, or free disk space. System Restore points may be missing.",
            cooldown_secs: 3_600,
            match_fn: matches_volsnap_shadow_copy_failure,
            fingerprint_fn: fingerprint_volsnap_volume,
        },
        Rule {
            id: "win.service.dependency_failure",
            platform: Platform::Windows,
            severity: Severity::Warning,
            title: "Service failed to start (dependency)",
            description: "A Windows service could not start because a service it depends on failed or is disabled.",
            recommended_action: "Check which dependency service failed in the event details. Re-enable it in services.msc or troubleshoot why it's failing.",
            cooldown_secs: 1_800,
            match_fn: matches_service_dependency_failure,
            fingerprint_fn: fingerprint_service_name,
        },
        Rule {
            id: "win.vss.error",
            platform: Platform::Windows,
            severity: Severity::Warning,
            title: "Volume Shadow Copy Service error",
            description: "The Volume Shadow Copy Service encountered an error. Backups and System Restore may not function correctly.",
            recommended_action: "Run vssadmin list writers to check VSS writer status. Restart the VSS service if needed.",
            cooldown_secs: 1_800,
            match_fn: matches_vss_error,
            fingerprint_fn: empty_fingerprint,
        },
        Rule {
            id: "win.dns.timeout",
            platform: Platform::Windows,
            severity: Severity::Info,
            title: "DNS resolution timeout",
            description: "A DNS lookup timed out. This may indicate network connectivity issues or DNS server problems.",
            recommended_action: "Check network connection and DNS server settings. Try ipconfig /flushdns to clear the DNS cache.",
            cooldown_secs: 300,
            match_fn: matches_dns_timeout,
            fingerprint_fn: fingerprint_dns_domain,
        },
        Rule {
            id: "win.dotnet.unhandled_exception",
            platform: Platform::Windows,
            severity: Severity::Warning,
            title: ".NET application crashed",
            description: "A .NET application terminated due to an unhandled exception.",
            recommended_action: "Check the exception details in the event. Update the application or report the crash to the developer.",
            cooldown_secs: 300,
            match_fn: matches_dotnet_unhandled_exception,
            fingerprint_fn: fingerprint_dotnet_app,
        },
    ]
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::SystemTime;

    use crate::event::{Event, Platform};
    use crate::rule::Rule;

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

    fn make_event_with(
        source: &str,
        event_id: u64,
        message: &str,
        metadata: HashMap<String, String>,
    ) -> Event {
        Event {
            timestamp: SystemTime::UNIX_EPOCH,
            platform: Platform::Windows,
            source: source.to_string(),
            event_id: Some(event_id),
            message: message.to_string(),
            metadata,
        }
    }

    fn find_rule<'a>(rules: &'a [Rule], id: &str) -> &'a Rule {
        rules
            .iter()
            .find(|rule| rule.id == id)
            .expect("expected rule to exist")
    }

    #[test]
    fn matches_disk_bad_block_rule() {
        let event = make_event("Disk", 7);
        assert!(rules()
            .iter()
            .any(|rule| rule.id == "win.disk.bad_block" && rule.matches(&event)));
    }

    #[test]
    fn matches_ntfs_corruption_rule() {
        let event = make_event("Ntfs", 55);
        assert!(rules()
            .iter()
            .any(|rule| rule.id == "win.ntfs.corruption" && rule.matches(&event)));
    }

    #[test]
    fn matches_unexpected_shutdown_rule() {
        let event = make_event("EventLog", 6008);
        assert!(rules()
            .iter()
            .any(|rule| rule.id == "win.system.unexpected_shutdown" && rule.matches(&event)));
    }

    #[test]
    fn matches_whea_hardware_error_rule() {
        let event = make_event("Microsoft-Windows-WHEA-Logger", 18);
        assert!(rules()
            .iter()
            .any(|rule| rule.id == "win.whea.hardware_error" && rule.matches(&event)));
    }

    #[test]
    fn matches_bugcheck_rule() {
        let event = make_event("BugCheck", 1001);
        assert!(rules()
            .iter()
            .any(|rule| rule.id == "win.bugcheck.summary" && rule.matches(&event)));
    }

    #[test]
    fn matches_kernel_power_dirty_reboot_rule() {
        let all_rules = rules();
        let rule = find_rule(&all_rules, "win.kernel_power.dirty_reboot");

        let positive = make_event("Microsoft-Windows-Kernel-Power", 41);
        assert!(rule.matches(&positive));

        let negative = make_event("Microsoft-Windows-Kernel-Power", 42);
        assert!(!rule.matches(&negative));
    }

    #[test]
    fn matches_tcpip_port_exhaustion_rule() {
        let all_rules = rules();
        let rule = find_rule(&all_rules, "win.tcpip.port_exhaustion");

        let positive = make_event("Tcpip", 4231);
        assert!(rule.matches(&positive));

        let negative = make_event("Tcpip", 4227);
        assert!(!rule.matches(&negative));
    }

    #[test]
    fn matches_application_crash_rule() {
        let all_rules = rules();
        let rule = find_rule(&all_rules, "win.app.crash");

        let mut metadata = HashMap::new();
        metadata.insert("device".to_string(), "chrome.exe".to_string());
        let positive = make_event_with(
            "Application Error",
            1000,
            "Faulting application name: chrome.exe, version: 124.0.0.0",
            metadata.clone(),
        );
        assert!(rule.matches(&positive));

        let negative = make_event_with("Application Error", 1002, "test", metadata);
        assert!(!rule.matches(&negative));
    }

    #[test]
    fn matches_application_hang_rule() {
        let all_rules = rules();
        let rule = find_rule(&all_rules, "win.app.hang");

        let mut metadata = HashMap::new();
        metadata.insert("device".to_string(), "explorer.exe".to_string());
        let positive = make_event_with(
            "Application Hang",
            1002,
            "The program explorer.exe version 10.0.0.0 stopped interacting with Windows.",
            metadata.clone(),
        );
        assert!(rule.matches(&positive));

        let negative = make_event_with("Application Hang", 1000, "test", metadata);
        assert!(!rule.matches(&negative));
    }

    #[test]
    fn matches_windows_update_install_failure_rule() {
        let all_rules = rules();
        let rule = find_rule(&all_rules, "win.update.install_failure");

        let positive = make_event_with(
            "Microsoft-Windows-WindowsUpdateClient",
            20,
            "Installation Failure: Windows failed to install the following update with error 0x80070005: 2026-03 Cumulative Update for Windows 11 for x64-based Systems (KB5039999).",
            HashMap::new(),
        );
        assert!(rule.matches(&positive));

        let negative = make_event_with(
            "Microsoft-Windows-WindowsUpdateClient",
            19,
            "Installation successful",
            HashMap::new(),
        );
        assert!(!rule.matches(&negative));
    }

    #[test]
    fn matches_volsnap_shadow_copy_failed_rule() {
        let all_rules = rules();
        let rule = find_rule(&all_rules, "win.volsnap.shadow_copy_failed");

        let positive = make_event_with(
            "Volsnap",
            36,
            "The shadow copies of volume C: were aborted because of insufficient storage.",
            HashMap::new(),
        );
        assert!(rule.matches(&positive));

        let negative = make_event_with("Volsnap", 35, "test", HashMap::new());
        assert!(!rule.matches(&negative));
    }

    #[test]
    fn matches_service_dependency_failure_rule() {
        let all_rules = rules();
        let rule = find_rule(&all_rules, "win.service.dependency_failure");

        let positive = make_event_with(
            "Service Control Manager",
            7001,
            "The Print Spooler service depends on the RPC Endpoint Mapper service which failed to start because of the following error: The service cannot be started.",
            HashMap::new(),
        );
        assert!(rule.matches(&positive));

        let negative = make_event_with("Service Control Manager", 7000, "test", HashMap::new());
        assert!(!rule.matches(&negative));
    }

    #[test]
    fn matches_vss_error_rule() {
        let all_rules = rules();
        let rule = find_rule(&all_rules, "win.vss.error");

        let positive = make_event_with(
            "VSS",
            8193,
            "Volume Shadow Copy Service error: Unexpected error querying for the IVssWriterCallback interface.",
            HashMap::new(),
        );
        assert!(rule.matches(&positive));

        let negative = make_event_with("VSS", 8224, "test", HashMap::new());
        assert!(!rule.matches(&negative));
    }

    #[test]
    fn matches_dns_timeout_rule() {
        let all_rules = rules();
        let rule = find_rule(&all_rules, "win.dns.timeout");

        let positive = make_event_with(
            "Microsoft-Windows-DNS-Client",
            1014,
            "Name resolution for the name api.example.com timed out after none of the configured DNS servers responded.",
            HashMap::new(),
        );
        assert!(rule.matches(&positive));

        let negative = make_event_with(
            "Microsoft-Windows-DNS-Client",
            1012,
            "Name resolution completed",
            HashMap::new(),
        );
        assert!(!rule.matches(&negative));
    }

    #[test]
    fn matches_dotnet_unhandled_exception_rule() {
        let all_rules = rules();
        let rule = find_rule(&all_rules, "win.dotnet.unhandled_exception");

        let positive = make_event_with(
            ".NET Runtime",
            1026,
            "Application: AcmeSync.exe Framework Version: v4.0.30319 Description: The process was terminated due to an unhandled exception.",
            HashMap::new(),
        );
        assert!(rule.matches(&positive));

        let negative = make_event_with(".NET Runtime", 1000, "test", HashMap::new());
        assert!(!rule.matches(&negative));
    }
}
