use std::path::PathBuf;

use anyhow::{Context, Result, bail};

/// Resolve the bashCustomization project root directory.
///
/// Resolution order:
///   1. `BASHC_ROOT` environment variable (if set and non-empty)
///   2. `$HOME/bashCustomization` (fallback)
///
/// Errors if the resolved path does not exist or is not a directory.
pub fn project_root() -> Result<PathBuf> {
    let bashc_root = std::env::var("BASHC_ROOT").ok().filter(|s| !s.is_empty());
    let home = std::env::var("HOME").ok().filter(|s| !s.is_empty());
    resolve_root(bashc_root.as_deref(), home.as_deref())
}

/// Inner resolution logic, parameterised for testability.
fn resolve_root(bashc_root: Option<&str>, home: Option<&str>) -> Result<PathBuf> {
    let path = if let Some(root) = bashc_root {
        PathBuf::from(root)
    } else {
        let home_dir = home.context("$HOME is not set and BASHC_ROOT is not set")?;
        PathBuf::from(home_dir).join("bashCustomization")
    };

    if !path.exists() {
        bail!("project root does not exist: {}", path.display());
    }

    if !path.is_dir() {
        bail!("project root is not a directory: {}", path.display());
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_bashc_root_env_returns_existing_dir() {
        let dir = tempdir().expect("failed to create temp dir");
        let result = resolve_root(Some(dir.path().to_str().unwrap()), None);
        assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        assert_eq!(result.unwrap(), dir.path());
    }

    #[test]
    fn test_home_fallback_uses_bashcustomization_subdir() {
        let home = tempdir().expect("failed to create temp dir for home");
        // Create the expected subdirectory so the path validates.
        let bashc_dir = home.path().join("bashCustomization");
        std::fs::create_dir(&bashc_dir).expect("failed to create bashCustomization dir");

        let result = resolve_root(None, Some(home.path().to_str().unwrap()));
        assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        assert_eq!(result.unwrap(), bashc_dir);
    }

    #[test]
    fn test_error_when_resolved_path_does_not_exist() {
        let dir = tempdir().expect("failed to create temp dir");
        let nonexistent = dir.path().join("does_not_exist");

        let result = resolve_root(Some(nonexistent.to_str().unwrap()), None);
        assert!(result.is_err(), "expected Err, got Ok");
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("does not exist"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn test_error_when_home_not_set_and_no_bashc_root() {
        let result = resolve_root(None, None);
        assert!(result.is_err(), "expected Err, got Ok");
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("HOME") || msg.contains("not set"),
            "unexpected error message: {msg}"
        );
    }
}
