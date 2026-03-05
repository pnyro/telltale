use std::error::Error;
use std::sync::mpsc;

use telltale_core::Event;

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

    fn watch(&mut self, _sender: mpsc::Sender<Event>) -> Result<(), Box<dyn Error + Send + Sync>> {
        Err("windows source is not implemented yet (Milestone B)".into())
    }
}
