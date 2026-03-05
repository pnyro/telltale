use std::collections::HashMap;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Linux,
    MacOS,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub timestamp: SystemTime,
    pub platform: Platform,
    pub source: String,
    pub event_id: Option<u64>,
    pub message: String,
    pub metadata: HashMap<String, String>,
}
