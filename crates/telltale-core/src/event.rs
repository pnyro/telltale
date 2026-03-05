use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    Windows,
    Linux,
    MacOS,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    #[serde(with = "system_time_seconds")]
    pub timestamp: SystemTime,
    pub platform: Platform,
    pub source: String,
    pub event_id: Option<u64>,
    pub message: String,
    pub metadata: HashMap<String, String>,
}

mod system_time_seconds {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let secs = match value.duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs() as i64,
            Err(err) => -(err.duration().as_secs() as i64),
        };
        serializer.serialize_i64(secs)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = i64::deserialize(deserializer)?;
        if secs >= 0 {
            Ok(UNIX_EPOCH + Duration::from_secs(secs as u64))
        } else {
            Ok(UNIX_EPOCH - Duration::from_secs((-secs) as u64))
        }
    }
}
