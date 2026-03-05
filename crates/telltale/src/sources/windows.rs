use std::error::Error;
use std::ffi::c_void;
use std::iter;
use std::sync::mpsc;
use std::time::SystemTime;

use telltale_core::{Event, Platform};
use windows::Win32::Foundation::{HANDLE, WAIT_FAILED, WAIT_OBJECT_0};
use windows::Win32::System::EventLog::{
    EVT_HANDLE, EvtClose, EvtNext, EvtRender, EvtRenderEventXml, EvtSubscribe,
    EvtSubscribeToFutureEvents,
};
use windows::Win32::System::Threading::{CreateEventW, INFINITE, WaitForMultipleObjects};
use windows::core::PCWSTR;

use super::EventSource;

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
        let mut events = [0isize; 8];
        loop {
            let mut returned = 0u32;
            let next = unsafe { EvtNext(self.handle, &mut events, 0, 0, &mut returned) };
            if next.is_err() {
                break;
            }

            for raw_handle in events.iter().take(returned as usize) {
                let event_handle = EVT_HANDLE(*raw_handle);
                let xml = render_event_xml(event_handle);
                unsafe {
                    let _ = EvtClose(event_handle);
                }
                let event = parse_event_xml(self.channel, &xml?);

                if sender.send(event).is_err() {
                    return Ok(());
                }
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

    Event {
        timestamp: SystemTime::now(),
        platform: Platform::Windows,
        source,
        event_id,
        message,
        metadata,
    }
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
