mod link;
pub mod manifest;
mod state;
mod status;
mod unlink;

use std::path::PathBuf;

use serde::Deserialize;

/// How to resolve a conflict when the target already exists.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Strategy {
    /// Show an interactive menu (default).
    Prompt,
    /// Backup the existing file and replace it.
    Replace,
    /// Replace without making a backup.
    Discard,
    /// Mark the existing file as self-managed; leave it in place.
    Keep,
}

impl Default for Strategy {
    fn default() -> Self {
        Strategy::Prompt
    }
}

/// A resolved config entry ready for use at runtime.
#[derive(Debug, Clone)]
pub struct ConfigEntry {
    /// Group name, e.g. "claude".
    pub name: String,
    /// Absolute path to the source file inside the repo's `configs/` directory.
    pub source: PathBuf,
    /// Absolute path to the target location on the system (tilde expanded).
    pub target: PathBuf,
    /// Conflict-resolution strategy for this entry.
    pub strategy: Strategy,
}
