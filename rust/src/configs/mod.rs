pub(crate) mod check;
pub(crate) mod diff;
pub(crate) mod link;
pub mod manifest;
pub(crate) mod state;
pub(crate) mod status;
pub(crate) mod unlink;

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::ValueEnum;
use serde::Deserialize;

/// Resolve `$HOME` as a `PathBuf`. Errors if HOME is unset or empty.
///
/// Centralized so the configs commands fail fast instead of silently
/// operating against an unrelated default like `/root` when running under
/// sudo or in a minimal environment.
pub(crate) fn home_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("$HOME environment variable is not set or is empty"))?;
    Ok(PathBuf::from(home))
}

/// The current linkage state of a config entry.
#[derive(Debug, Clone, PartialEq)]
pub enum EntryState {
    /// Symlink at target points to the correct source.
    Linked,
    /// User chose to keep their local file (recorded in managed_configs.toml).
    SelfManaged,
    /// Symlink at target points to a different location.
    WrongSymlink,
    /// A regular file exists at the target (not managed by bashc).
    Conflict,
    /// No file exists at the target.
    NotLinked,
}

/// How to resolve a conflict when the target already exists.
#[derive(Debug, Clone, PartialEq, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Strategy {
    /// Show an interactive menu (default).
    #[clap(skip)]
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

// ---------------------------------------------------------------------------
// Shared display helpers used by status and link
// ---------------------------------------------------------------------------

/// Format the source path for display as `<name>/<filename>`.
///
/// The source in `ConfigEntry` is the full absolute path inside the repo's
/// `configs/` directory, e.g. `/repo/configs/claude/CLAUDE.md`. We want to
/// show just `claude/CLAUDE.md` (relative to `configs/`).
///
/// We do this by taking the last two components of the path when they exist,
/// otherwise falling back to the full path string.
pub(crate) fn format_source(entry: &ConfigEntry) -> String {
    let mut components: Vec<&str> = entry
        .source
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    if components.len() >= 2 {
        let last_two = components.split_off(components.len() - 2);
        last_two.join("/")
    } else {
        entry.source.to_string_lossy().into_owned()
    }
}

/// Replace a `$HOME` prefix in `target` with `~` for compact display.
pub(crate) fn display_target(target: &Path, home: &Path) -> String {
    if let Ok(rel) = target.strip_prefix(home) {
        format!("~/{}", rel.display())
    } else {
        target.to_string_lossy().into_owned()
    }
}
