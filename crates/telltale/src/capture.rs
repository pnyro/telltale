use std::error::Error;
use std::fs;
use std::path::Path;

use crate::sources;

pub fn run(hours: u64, output: &Path) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut source = sources::default_historical_source()?;
    let events = source.scan(hours)?;

    if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(&events)?;
    fs::write(output, json)?;

    println!(
        "Captured {} events from last {}h to {}",
        events.len(),
        hours,
        output.display()
    );

    Ok(())
}
