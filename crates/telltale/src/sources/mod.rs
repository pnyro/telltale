use std::error::Error;
use std::time::Duration;

pub mod simulated;

pub use telltale_core::sources::{EventSource, default_historical_source, default_source};

pub fn simulated_source(
    interval: Duration,
    count: u64,
) -> Result<Box<dyn EventSource>, Box<dyn Error + Send + Sync>> {
    Ok(Box::new(simulated::SimulatedSource::new(interval, count)?))
}
