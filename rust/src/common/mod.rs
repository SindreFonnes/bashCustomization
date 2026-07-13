pub mod command;
pub mod download;
pub mod package_manager;
pub mod platform;
pub mod privilege;
pub mod project_root;
pub mod version;

use std::path::PathBuf;

use anyhow::Result;

/// Resolve `$HOME` as a `PathBuf`. Errors if HOME is unset or empty.
pub fn home_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("$HOME environment variable is not set or is empty"))?;
    Ok(PathBuf::from(home))
}
