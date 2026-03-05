use crate::event::{Event, Platform, Severity};

pub type MatchFn = fn(&Event) -> bool;
pub type FingerprintFn = fn(&Event) -> String;

pub struct Rule {
    pub id: &'static str,
    pub platform: Platform,
    pub severity: Severity,
    pub title: &'static str,
    pub description: &'static str,
    pub recommended_action: &'static str,
    pub cooldown_secs: u64,
    pub match_fn: MatchFn,
    pub fingerprint_fn: FingerprintFn,
}

impl Rule {
    pub fn matches(&self, event: &Event) -> bool {
        self.platform == event.platform && (self.match_fn)(event)
    }

    pub fn fingerprint(&self, event: &Event) -> String {
        (self.fingerprint_fn)(event)
    }
}

pub fn empty_fingerprint(_: &Event) -> String {
    String::new()
}
