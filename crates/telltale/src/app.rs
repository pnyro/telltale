use std::error::Error;
use std::path::PathBuf;

use telltale_core::{Rule, knowledge};

pub fn rules_for_current_os() -> Vec<Rule> {
    match std::env::consts::OS {
        "linux" => knowledge::linux_rules(),
        "windows" => knowledge::windows_rules(),
        _ => Vec::new(),
    }
}

pub fn data_dir() -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let mut dir = dirs::data_dir().ok_or("failed to resolve data directory")?;
    dir.push("telltale");
    Ok(dir)
}

pub fn database_path() -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let mut path = data_dir()?;
    path.push("telltale.db");
    Ok(path)
}
