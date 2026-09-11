pub mod command;
pub mod download;
pub mod package_manager;
pub mod platform;
pub mod privilege;
pub mod project_root;
pub mod version;

use std::path::{Path, PathBuf};

use anyhow::Result;

/// Resolve `$HOME` as a `PathBuf`. Errors if HOME is unset or empty.
pub fn home_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("$HOME environment variable is not set or is empty"))?;
    Ok(PathBuf::from(home))
}

/// Resolve the user-level binary directory configured by the bootstrap.
///
/// The environment takes precedence for one-off invocations. Otherwise use
/// the persisted bootstrap setting, falling back to the historical `~/.mybin`.
pub fn user_bin_dir() -> Result<PathBuf> {
    let home = home_dir()?;
    Ok(resolve_user_bin_dir(
        std::env::var_os("BASHC_INSTALL_DIR")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from),
        &home,
    ))
}

fn resolve_user_bin_dir(configured: Option<PathBuf>, home: &Path) -> PathBuf {
    if let Some(path) = configured {
        return path;
    }

    let state_path = home.join(".config/bashc/install_dir");
    if let Ok(value) = std::fs::read_to_string(&state_path) {
        let value = value.trim_end_matches(['\r', '\n']);
        if !value.is_empty() {
            return PathBuf::from(value);
        }
    }

    home.join(".mybin")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_user_bin_dir_trims_the_record_newline() {
        let home = tempfile::tempdir().unwrap();
        let state_dir = home.path().join(".config/bashc");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(state_dir.join("install_dir"), "/custom/bin\n").unwrap();

        assert_eq!(
            resolve_user_bin_dir(None, home.path()),
            PathBuf::from("/custom/bin")
        );
        assert_eq!(
            resolve_user_bin_dir(Some(PathBuf::from("/env/bin")), home.path()),
            PathBuf::from("/env/bin")
        );
    }
}
