use std::error::Error;
use std::sync::mpsc;
use std::time::Duration;

use telltale_core::Event;

#[cfg(target_os = "linux")]
mod journald;
pub mod simulated;
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

pub fn simulated_source(
    interval: Duration,
    count: u64,
) -> Result<Box<dyn EventSource>, Box<dyn Error + Send + Sync>> {
    Ok(Box::new(simulated::SimulatedSource::new(interval, count)?))
}
