use std::error::Error;
use std::ffi::c_void;
use std::iter;
use std::sync::mpsc;
use std::time::{Duration, SystemTime};

use crate::{Event, Platform};
use windows::Win32::Foundation::{HANDLE, WAIT_FAILED, WAIT_OBJECT_0};
use windows::Win32::System::EventLog::{
    EVT_HANDLE, EvtClose, EvtNext, EvtQuery, EvtQueryChannelPath, EvtQueryReverseDirection,
    EvtRender, EvtRenderEventXml, EvtSubscribe, EvtSubscribeToFutureEvents,
};
use windows::Win32::System::Threading::{CreateEventW, INFINITE, WaitForMultipleObjects};
use windows::core::PCWSTR;

use super::{EventSource, HistoricalEventSource};

pub struct WindowsEventSource;

impl WindowsEventSource {
    pub fn new() -> Self {
        Self
    }
}

impl EventSource for WindowsEventSource {
    fn name(&self) -> &'static str {
        "windows-event-log"
    }

    fn watch(&mut self, sender: mpsc::Sender<Event>) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut subscriptions = vec![
            Subscription::new("System", "*")?,
            Subscription::new("Application", "*")?,
        ];

        let handles: Vec<HANDLE> = subscriptions.iter().map(|sub| sub.signal).collect();

        loop {
            let wait = unsafe { WaitForMultipleObjects(&handles, false, INFINITE) };
            if wait == WAIT_FAILED {
                return Err("WaitForMultipleObjects failed".into());
            }

            let index = (wait.0 - WAIT_OBJECT_0.0) as usize;
            if index >= subscriptions.len() {
                continue;
            }

            subscriptions[index].drain_into(&sender)?;
        }
    }
}

impl HistoricalEventSource for WindowsEventSource {
    fn name(&self) -> &'static str {
        "windows-event-log"
    }

    fn scan(&mut self, hours: u64) -> Result<Vec<Event>, Box<dyn Error + Send + Sync>> {
        let query = build_time_window_xpath(hours);
        let mut all_events = Vec::new();

        for channel in ["System", "Application"] {
            let channel_w = to_wide(channel);
            let query_w = to_wide(&query);
            let flags = EvtQueryChannelPath.0 | EvtQueryReverseDirection.0;

            let handle = unsafe {
                EvtQuery(
                    None,
                    PCWSTR(channel_w.as_ptr()),
                    PCWSTR(query_w.as_ptr()),
                    flags,
                )?
            };

            let events = collect_events(handle, channel)?;
            unsafe {
                let _ = EvtClose(handle);
            }
            all_events.extend(events);
        }

        Ok(all_events)
    }
}

struct Subscription {
    channel: &'static str,
    signal: HANDLE,
    handle: EVT_HANDLE,
}

impl Subscription {
    fn new(channel: &'static str, query: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let channel_w = to_wide(channel);
        let query_w = to_wide(query);

        let signal = unsafe { CreateEventW(None, false, false, PCWSTR::null())? };
        let handle = unsafe {
            EvtSubscribe(
                None,
                signal,
                PCWSTR(channel_w.as_ptr()),
                PCWSTR(query_w.as_ptr()),
                None,
                None,
                None,
                EvtSubscribeToFutureEvents.0,
            )?
        };

        Ok(Self {
            channel,
            signal,
            handle,
        })
    }

    fn drain_into(
        &mut self,
        sender: &mpsc::Sender<Event>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        for event in collect_events(self.handle, self.channel)? {
            if sender.send(event).is_err() {
                return Ok(());
            }
        }

        Ok(())
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        unsafe {
            let _ = EvtClose(self.handle);
            let _ = windows::Win32::Foundation::CloseHandle(self.signal);
        }
    }
}

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(iter::once(0)).collect()
}

fn build_time_window_xpath(hours: u64) -> String {
    let millis = hours
        .saturating_mul(60)
        .saturating_mul(60)
        .saturating_mul(1000);
    format!("*[System[TimeCreated[timediff(@SystemTime) <= {millis}]]]")
}

fn collect_events(
    handle: EVT_HANDLE,
    channel_hint: &str,
) -> Result<Vec<Event>, Box<dyn Error + Send + Sync>> {
    let mut events = Vec::new();
    let mut raw_events = [0isize; 32];

    loop {
        let mut returned = 0u32;
        let next = unsafe { EvtNext(handle, &mut raw_events, 0, 0, &mut returned) };
        if next.is_err() {
            break;
        }

        for raw_handle in raw_events.iter().take(returned as usize) {
            let event_handle = EVT_HANDLE(*raw_handle);
            let xml = render_event_xml(event_handle)?;
            unsafe {
                let _ = EvtClose(event_handle);
            }
            events.push(parse_event_xml(channel_hint, &xml));
        }
    }

    Ok(events)
}

fn render_event_xml(event: EVT_HANDLE) -> Result<String, Box<dyn Error + Send + Sync>> {
    let mut used = 0u32;
    let mut props = 0u32;

    let _ = unsafe {
        EvtRender(
            None,
            event,
            EvtRenderEventXml.0,
            0,
            None,
            &mut used,
            &mut props,
        )
    };

    if used == 0 {
        return Ok(String::new());
    }

    let wchar_len = (used as usize / 2).saturating_add(1);
    let mut buffer = vec![0u16; wchar_len];

    unsafe {
        EvtRender(
            None,
            event,
            EvtRenderEventXml.0,
            used,
            Some(buffer.as_mut_ptr() as *mut c_void),
            &mut used,
            &mut props,
        )?;
    }

    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Ok(String::from_utf16_lossy(&buffer[..len]))
}

fn parse_event_xml(channel_hint: &str, xml: &str) -> Event {
    let source = extract_provider_name(xml).unwrap_or_else(|| "unknown".to_string());
    let event_id = extract_tag_text_with_attributes(xml, "EventID")
        .or_else(|| extract_text(xml, "EventID"))
        .and_then(|raw| raw.parse::<u64>().ok());
    let channel = extract_text(xml, "Channel").unwrap_or_else(|| channel_hint.to_string());
    let computer = extract_text(xml, "Computer").unwrap_or_default();
    let message = extract_text(xml, "Message")
        .or_else(|| extract_first_data(xml))
        .unwrap_or_else(|| xml.to_string());

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("provider".to_string(), source.clone());
    metadata.insert("channel".to_string(), channel);
    metadata.insert("computer".to_string(), computer.clone());
    metadata.insert("entity".to_string(), source.clone());
    metadata.insert("xml".to_string(), xml.to_string());

    if let Some(record_id) = extract_text(xml, "EventRecordID") {
        metadata.insert("event_record_id".to_string(), record_id);
    }

    if let Some(device) = extract_first_data(xml) {
        metadata.insert("device".to_string(), device);
    }

    if !computer.is_empty() {
        metadata.insert("host".to_string(), computer);
    }

    let timestamp = extract_time_created(xml).unwrap_or_else(SystemTime::now);

    Event {
        timestamp,
        platform: Platform::Windows,
        source,
        event_id,
        message,
        metadata,
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::{build_time_window_xpath, extract_time_created};

    #[test]
    fn builds_expected_time_window_xpath_for_hours() {
        let xpath = build_time_window_xpath(48);
        assert_eq!(
            xpath,
            "*[System[TimeCreated[timediff(@SystemTime) <= 172800000]]]"
        );
    }

    #[test]
    fn builds_zero_hour_xpath_without_clamping() {
        let xpath = build_time_window_xpath(0);
        assert_eq!(xpath, "*[System[TimeCreated[timediff(@SystemTime) <= 0]]]");
    }

    #[test]
    fn parses_time_created_with_fractional_seconds() {
        let xml = r#"<Event><System><TimeCreated SystemTime='2026-03-05T14:06:55.1234567Z'/></System></Event>"#;
        let ts = extract_time_created(xml).unwrap();
        // 2026-03-05T14:06:55Z = days from epoch + time
        let epoch_secs = ts.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        // 2026-03-05 = 20517 days from epoch (1970-01-01)
        // 14:06:55 = 50815 seconds
        assert_eq!(epoch_secs, 20517 * 86400 + 50815);
    }

    #[test]
    fn parses_time_created_without_fractional_seconds() {
        let xml = r#"<TimeCreated SystemTime='2024-01-01T00:00:00Z'/>"#;
        let ts = extract_time_created(xml).unwrap();
        let epoch_secs = ts.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        // 2024-01-01 = 19723 days from epoch
        assert_eq!(epoch_secs, 19723 * 86400);
    }

    #[test]
    fn returns_none_for_missing_time_created() {
        let xml = r#"<Event><System><Provider Name='Test'/></System></Event>"#;
        assert!(extract_time_created(xml).is_none());
    }
}

/// Parse `<TimeCreated SystemTime='2026-03-05T10:30:00.123456789Z'/>` into SystemTime.
/// Handles the ISO 8601 format with fractional seconds that Windows Event Log uses.
fn extract_time_created(xml: &str) -> Option<SystemTime> {
    let tag = "<TimeCreated ";
    let start = xml.find(tag)?;
    let after = &xml[start..];
    let raw = extract_attribute(after, "SystemTime")?;

    // Format: "2026-03-05T10:30:00.1234567Z" or "2026-03-05T10:30:00Z"
    let s = raw.trim_end_matches('Z');
    let (datetime, frac) = if let Some((dt, f)) = s.split_once('.') {
        (dt, f)
    } else {
        (s, "0")
    };

    let mut parts = datetime.split('T');
    let date = parts.next()?;
    let time = parts.next()?;

    let date_parts: Vec<&str> = date.split('-').collect();
    if date_parts.len() != 3 {
        return None;
    }
    let year: u64 = date_parts[0].parse().ok()?;
    let month: u64 = date_parts[1].parse().ok()?;
    let day: u64 = date_parts[2].parse().ok()?;

    let time_parts: Vec<&str> = time.split(':').collect();
    if time_parts.len() != 3 {
        return None;
    }
    let hour: u64 = time_parts[0].parse().ok()?;
    let min: u64 = time_parts[1].parse().ok()?;
    let sec: u64 = time_parts[2].parse().ok()?;

    // Fractional seconds → nanoseconds (Windows gives up to 7 digits)
    let frac_padded = format!("{:0<9}", frac);
    let nanos: u64 = frac_padded[..9].parse().ok()?;

    // Days from year 1970 (simplified, no leap second handling)
    let days = days_from_epoch(year, month, day)?;
    let total_secs = days * 86400 + hour * 3600 + min * 60 + sec;

    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(total_secs) + Duration::from_nanos(nanos))
}

/// Days from Unix epoch to the given date. Handles leap years.
fn days_from_epoch(year: u64, month: u64, day: u64) -> Option<u64> {
    if month < 1 || month > 12 || day < 1 || day > 31 || year < 1970 {
        return None;
    }

    let days_in_months = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    let mut days: u64 = 0;
    for y in 1970..year {
        days += if is_leap(y) { 366 } else { 365 };
    }
    for m in 1..month {
        days += days_in_months[m as usize] as u64;
        if m == 2 && is_leap(year) {
            days += 1;
        }
    }
    days += day - 1;

    Some(days)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn extract_provider_name(xml: &str) -> Option<String> {
    let provider_tag = "<Provider ";
    let start = xml.find(provider_tag)?;
    let after = &xml[start..];

    extract_attribute(after, "Name")
}

fn extract_attribute(xml_fragment: &str, attr: &str) -> Option<String> {
    let needle_single = format!("{attr}='");
    if let Some(start) = xml_fragment.find(&needle_single) {
        let rest = &xml_fragment[start + needle_single.len()..];
        let end = rest.find('\'')?;
        return Some(rest[..end].to_string());
    }

    let needle_double = format!("{attr}=\"");
    let start = xml_fragment.find(&needle_double)?;
    let rest = &xml_fragment[start + needle_double.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn extract_text(xml: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{tag}>");
    let end_tag = format!("</{tag}>");
    let start = xml.find(&start_tag)?;
    let content_start = start + start_tag.len();
    let end = xml[content_start..].find(&end_tag)?;
    Some(xml[content_start..content_start + end].trim().to_string())
}

fn extract_first_data(xml: &str) -> Option<String> {
    let start = xml.find("<Data")?;
    let data_section = &xml[start..];
    let gt_index = data_section.find('>')?;
    let content_start = start + gt_index + 1;
    let end = xml[content_start..].find("</Data>")?;
    Some(xml[content_start..content_start + end].trim().to_string())
}

fn extract_tag_text_with_attributes(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let start = xml.find(&open)?;
    let after_open = &xml[start + open.len()..];
    let gt = after_open.find('>')?;
    let content_start = start + open.len() + gt + 1;
    let close = format!("</{tag}>");
    let end = xml[content_start..].find(&close)?;
    Some(xml[content_start..content_start + end].trim().to_string())
}
