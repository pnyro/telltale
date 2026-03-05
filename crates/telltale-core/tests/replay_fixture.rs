use std::collections::HashSet;

use telltale_core::{knowledge, Engine, Event};

#[test]
fn sample_windows_fixture_replays_against_windows_rules() {
    let json = include_str!("../../../fixtures/sample_windows_events.json");
    let events: Vec<Event> = serde_json::from_str(json).expect("fixture JSON should deserialize");
    assert_eq!(events.len(), 10, "fixture should include ten sample events");

    let mut engine = Engine::new(knowledge::windows_rules());
    let mut matched_rule_ids = HashSet::new();

    for event in &events {
        for alert in engine.process(event) {
            matched_rule_ids.insert(alert.rule_id);
        }
    }

    for expected in [
        "win.kernel_power.dirty_reboot",
        "win.tcpip.port_exhaustion",
        "win.app.crash",
        "win.app.hang",
        "win.update.install_failure",
        "win.volsnap.shadow_copy_failed",
        "win.service.dependency_failure",
        "win.vss.error",
        "win.dns.timeout",
        "win.dotnet.unhandled_exception",
    ] {
        assert!(
            matched_rule_ids.contains(expected),
            "expected fixture to match rule {expected}"
        );
    }
}
