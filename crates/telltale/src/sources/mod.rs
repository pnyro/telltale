use std::error::Error;
use std::sync::mpsc;

use telltale_core::Event;

#[cfg(target_os = "linux")]
mod journald;
#[cfg(target_os = "windows")]
mod windows;

pub trait EventSource: Send {
    fn name(&self) -> &'static str;
    fn watch(&mut self, sender: mpsc::Sender<Event>) -> Result<(), Box<dyn Error + Send + Sync>>;
}

pub fn default_source() -> Result<Box<dyn EventSource>, Box<dyn Error + Send + Sync>> {
    #[cfg(target_os = "linux")]
    {
        return Ok(Box::new(journald::JournaldSource::new()));
    }

    #[cfg(target_os = "windows")]
    {
        return Ok(Box::new(windows::WindowsEventSource::new()));
    }

    #[allow(unreachable_code)]
    Err("no default source for this OS".into())
}
