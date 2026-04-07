// state: tracks which configs are currently linked

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::{ConfigEntry, EntryState};

/// A single entry in `local/managed_configs.toml` that the user asked to keep locally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SelfManagedEntry {
    pub name: String,
    pub source: String,
    pub target: String,
}

/// Root structure of `local/managed_configs.toml`.
#[derive(Debug, Default, Serialize, Deserialize)]
struct SelfManagedFile {
    #[serde(default)]
    self_managed: Vec<SelfManagedEntry>,
}

fn managed_configs_path(project_root: &Path) -> PathBuf {
    project_root.join("local").join("managed_configs.toml")
}

/// Load self-managed entries from `<project_root>/local/managed_configs.toml`.
/// Returns an empty Vec if the file does not exist.
pub(crate) fn load_self_managed(project_root: &Path) -> Result<Vec<SelfManagedEntry>> {
    let path = managed_configs_path(project_root);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let file: SelfManagedFile =
        toml::from_str(&contents).with_context(|| format!("parsing {}", path.display()))?;
    Ok(file.self_managed)
}

/// Add an entry to `local/managed_configs.toml`.
/// Does nothing if an entry with the same target already exists.
pub(crate) fn add_self_managed(project_root: &Path, entry: SelfManagedEntry) -> Result<()> {
    let path = managed_configs_path(project_root);
    let mut file = if path.exists() {
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        toml::from_str::<SelfManagedFile>(&contents)
            .with_context(|| format!("parsing {}", path.display()))?
    } else {
        SelfManagedFile::default()
    };

    // Deduplicate: don't add if same target already present.
    if file.self_managed.iter().any(|e| e.target == entry.target) {
        return Ok(());
    }

    file.self_managed.push(entry);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }

    let serialized =
        toml::to_string_pretty(&file).context("serializing managed_configs.toml")?;
    std::fs::write(&path, serialized)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Remove the entry matching `target` from `local/managed_configs.toml`.
/// If the file becomes empty, it is deleted.
pub(crate) fn remove_self_managed(project_root: &Path, target: &str) -> Result<()> {
    let path = managed_configs_path(project_root);
    if !path.exists() {
        return Ok(());
    }

    let contents =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut file: SelfManagedFile =
        toml::from_str(&contents).with_context(|| format!("parsing {}", path.display()))?;

    file.self_managed.retain(|e| e.target != target);

    if file.self_managed.is_empty() {
        std::fs::remove_file(&path)
            .with_context(|| format!("deleting {}", path.display()))?;
    } else {
        let serialized =
            toml::to_string_pretty(&file).context("serializing managed_configs.toml")?;
        std::fs::write(&path, serialized)
            .with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(())
}

/// Returns `true` if any entry's target matches the given path (compared as strings).
pub(crate) fn is_self_managed(entries: &[SelfManagedEntry], target: &Path) -> bool {
    let target_str = target.to_string_lossy();
    entries.iter().any(|e| e.target == target_str.as_ref())
}

/// Prune entries from `local/managed_configs.toml` that are no longer
/// reachable. An entry is stale when **either**:
///   1. its `target` is NOT in `all_platform_targets` (entry no longer in
///      the manifest at all), OR
///   2. its `target` IS in `current_platform_targets` AND the file at
///      that target does not exist on disk.
///
/// Cross-platform safety: a marker whose target appears in
/// `all_platform_targets` but NOT in `current_platform_targets` is
/// preserved unconditionally (it belongs to a different OS's view of the
/// manifest, and we make no judgement about whether the file should
/// exist on this machine).
///
/// `all_platform_targets` MUST come from `load_manifest_unfiltered`.
/// `current_platform_targets` MUST come from `load_manifest` for the
/// current platform.
///
/// Returns the number of entries removed (for testing/observability).
/// Silent — does not print anything.
pub(crate) fn prune_stale_self_managed(
    project_root: &Path,
    current_platform_targets: &[String],
    all_platform_targets: &[String],
) -> Result<usize> {
    let entries = load_self_managed(project_root)?;
    if entries.is_empty() {
        return Ok(0);
    }

    // Iterate over a snapshot — collect stale targets first, then remove.
    let stale: Vec<String> = entries
        .iter()
        .filter(|e| {
            let in_all = all_platform_targets.iter().any(|t| t == &e.target);
            let in_current = current_platform_targets.iter().any(|t| t == &e.target);

            // Condition 1: not in the unfiltered manifest at all.
            if !in_all {
                return true;
            }
            // Cross-platform safety: if it's in the unfiltered manifest but
            // not the current platform's view, preserve it unconditionally.
            if !in_current {
                return false;
            }
            // Condition 2: in current platform manifest but file missing on disk.
            !Path::new(&e.target).exists()
        })
        .map(|e| e.target.clone())
        .collect();

    for target in &stale {
        remove_self_managed(project_root, target)?;
    }

    Ok(stale.len())
}

/// Detect the current state of a config entry.
///
/// Precedence (highest to lowest):
/// 1. Symlink at target pointing to the correct source -> `Linked`
/// 2. Target exists AND is in self_managed list -> `SelfManaged`
/// 3. Target is a symlink pointing elsewhere -> `WrongSymlink`
/// 4. Target exists as a regular file/dir -> `Conflict`
/// 5. Target is in self_managed list but file no longer exists -> `NotLinked`
/// 6. Target does not exist -> `NotLinked`
pub(crate) fn detect_state(entry: &ConfigEntry, self_managed: &[SelfManagedEntry]) -> EntryState {
    // Check whether a symlink (or any filesystem object) exists at target.
    // symlink_metadata does NOT follow symlinks, so it returns Ok even for broken links.
    let meta = std::fs::symlink_metadata(&entry.target);
    let target_exists = meta.is_ok();

    // 1. Check if target is a symlink pointing to the correct source.
    if let Ok(link_dest) = std::fs::read_link(&entry.target) {
        if link_dest == entry.source {
            return EntryState::Linked;
        }
        // It's a symlink, but points somewhere else.
        // Before returning WrongSymlink check self-managed (rule 2 only applies to non-symlink
        // files; a symlink to a wrong place is WrongSymlink regardless).
        return EntryState::WrongSymlink;
    }

    // Target is not a symlink (or doesn't exist at all).
    if target_exists {
        // 2. Target exists as a regular file/dir.
        if is_self_managed(self_managed, &entry.target) {
            return EntryState::SelfManaged;
        }
        // 4. Regular file/dir, not managed by us.
        return EntryState::Conflict;
    }

    // 5 & 6. Target does not exist — NotLinked regardless of self_managed list.
    EntryState::NotLinked
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    /// Build a minimal ConfigEntry for test purposes.
    fn make_entry(source: PathBuf, target: PathBuf) -> ConfigEntry {
        ConfigEntry {
            name: "test".to_string(),
            source,
            target,
            strategy: crate::configs::Strategy::Prompt,
        }
    }

    // ── detect_state tests ────────────────────────────────────────────────────

    #[test]
    fn detect_linked_when_symlink_points_to_source() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        symlink(&source, &target).unwrap();

        let entry = make_entry(source, target);
        assert_eq!(detect_state(&entry, &[]), EntryState::Linked);
    }

    #[test]
    fn detect_not_linked_when_target_absent() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        // target intentionally not created

        let entry = make_entry(source, target);
        assert_eq!(detect_state(&entry, &[]), EntryState::NotLinked);
    }

    #[test]
    fn detect_conflict_when_target_is_regular_file() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        std::fs::write(&target, "other").unwrap();

        let entry = make_entry(source, target);
        assert_eq!(detect_state(&entry, &[]), EntryState::Conflict);
    }

    #[test]
    fn detect_wrong_symlink_when_target_points_elsewhere() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let other = dir.path().join("other.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        std::fs::write(&other, "other").unwrap();
        symlink(&other, &target).unwrap();

        let entry = make_entry(source, target);
        assert_eq!(detect_state(&entry, &[]), EntryState::WrongSymlink);
    }

    #[test]
    fn detect_self_managed_when_target_exists_and_in_list() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        std::fs::write(&target, "local").unwrap();

        let sm = vec![SelfManagedEntry {
            name: "test".to_string(),
            source: source.to_string_lossy().to_string(),
            target: target.to_string_lossy().to_string(),
        }];
        let entry = make_entry(source, target);
        assert_eq!(detect_state(&entry, &sm), EntryState::SelfManaged);
    }

    #[test]
    fn detect_linked_takes_precedence_over_self_managed() {
        // Even if the entry is in self_managed list, a correct symlink means Linked.
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        symlink(&source, &target).unwrap();

        let sm = vec![SelfManagedEntry {
            name: "test".to_string(),
            source: source.to_string_lossy().to_string(),
            target: target.to_string_lossy().to_string(),
        }];
        let entry = make_entry(source, target);
        assert_eq!(detect_state(&entry, &sm), EntryState::Linked);
    }

    #[test]
    fn detect_not_linked_for_stale_self_managed() {
        // Entry is in self_managed list but the file has been deleted.
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        // target never created (or was deleted)

        let sm = vec![SelfManagedEntry {
            name: "test".to_string(),
            source: source.to_string_lossy().to_string(),
            target: target.to_string_lossy().to_string(),
        }];
        let entry = make_entry(source, target);
        assert_eq!(detect_state(&entry, &sm), EntryState::NotLinked);
    }

    // ── load_self_managed tests ───────────────────────────────────────────────

    #[test]
    fn load_returns_empty_vec_when_file_missing() {
        let dir = tempdir().unwrap();
        let result = load_self_managed(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    // ── add / load round-trip tests ───────────────────────────────────────────

    #[test]
    fn add_then_load_round_trips() {
        let dir = tempdir().unwrap();
        let entry = SelfManagedEntry {
            name: "nvim".to_string(),
            source: "/repo/configs/nvim/init.lua".to_string(),
            target: "/home/user/.config/nvim/init.lua".to_string(),
        };

        add_self_managed(dir.path(), entry.clone()).unwrap();

        let loaded = load_self_managed(dir.path()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, entry.name);
        assert_eq!(loaded[0].source, entry.source);
        assert_eq!(loaded[0].target, entry.target);
    }

    #[test]
    fn add_does_not_duplicate_same_target() {
        let dir = tempdir().unwrap();
        let entry = SelfManagedEntry {
            name: "nvim".to_string(),
            source: "/repo/configs/nvim/init.lua".to_string(),
            target: "/home/user/.config/nvim/init.lua".to_string(),
        };

        add_self_managed(dir.path(), entry.clone()).unwrap();
        add_self_managed(dir.path(), entry.clone()).unwrap();

        let loaded = load_self_managed(dir.path()).unwrap();
        assert_eq!(loaded.len(), 1);
    }

    // ── remove_self_managed tests ─────────────────────────────────────────────

    #[test]
    fn remove_deletes_entry_by_target() {
        let dir = tempdir().unwrap();
        let e1 = SelfManagedEntry {
            name: "nvim".to_string(),
            source: "/src/nvim".to_string(),
            target: "/home/user/.config/nvim".to_string(),
        };
        let e2 = SelfManagedEntry {
            name: "tmux".to_string(),
            source: "/src/tmux".to_string(),
            target: "/home/user/.tmux.conf".to_string(),
        };

        add_self_managed(dir.path(), e1.clone()).unwrap();
        add_self_managed(dir.path(), e2.clone()).unwrap();

        remove_self_managed(dir.path(), &e1.target).unwrap();

        let loaded = load_self_managed(dir.path()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].target, e2.target);
    }

    #[test]
    fn remove_deletes_file_when_empty() {
        let dir = tempdir().unwrap();
        let entry = SelfManagedEntry {
            name: "nvim".to_string(),
            source: "/src/nvim".to_string(),
            target: "/home/user/.config/nvim".to_string(),
        };

        add_self_managed(dir.path(), entry.clone()).unwrap();
        remove_self_managed(dir.path(), &entry.target).unwrap();

        let path = managed_configs_path(dir.path());
        assert!(!path.exists(), "file should be deleted when empty");
    }

    #[test]
    fn remove_is_noop_when_file_missing() {
        let dir = tempdir().unwrap();
        // Should not error even if file doesn't exist.
        remove_self_managed(dir.path(), "/some/target").unwrap();
    }

    // ── prune_stale_self_managed tests ────────────────────────────────────────

    /// Helper: add a SelfManagedEntry with all string fields.
    fn make_sm_entry(name: &str, source: &str, target: &str) -> SelfManagedEntry {
        SelfManagedEntry {
            name: name.to_string(),
            source: source.to_string(),
            target: target.to_string(),
        }
    }

    #[test]
    fn prune_returns_zero_when_no_markers() {
        // Empty self-managed list — prune should return 0, no error, no file written.
        let dir = tempdir().unwrap();
        let removed = prune_stale_self_managed(dir.path(), &[], &[]).unwrap();
        assert_eq!(removed, 0);
        assert!(!managed_configs_path(dir.path()).exists());
    }

    #[test]
    fn prune_removes_entry_with_missing_target_when_in_current_filtered_manifest() {
        // Condition 2: target IS in current_platform_targets AND file does NOT exist on disk.
        let dir = tempdir().unwrap();
        let target = dir.path().join("nvim_init.lua");
        let target_str = target.to_string_lossy().to_string();

        // Add a marker for target — but do NOT create the file on disk.
        add_self_managed(dir.path(), make_sm_entry("nvim", "/src/nvim/init.lua", &target_str))
            .unwrap();

        let current = vec![target_str.clone()];
        let all = vec![target_str.clone()];

        let removed = prune_stale_self_managed(dir.path(), &current, &all).unwrap();
        assert_eq!(removed, 1);

        let remaining = load_self_managed(dir.path()).unwrap();
        assert!(remaining.is_empty(), "marker should have been removed");
    }

    #[test]
    fn prune_removes_entry_not_in_unfiltered_manifest() {
        // Condition 1: target is NOT in all_platform_targets (removed from manifest entirely).
        let dir = tempdir().unwrap();
        let target = dir.path().join("tmux_conf");
        let target_str = target.to_string_lossy().to_string();

        // Create the file on disk — but it's not in any manifest slice.
        std::fs::write(&target, "local config").unwrap();

        add_self_managed(dir.path(), make_sm_entry("tmux", "/src/tmux", &target_str)).unwrap();

        // Target absent from both slices — fully removed from manifest.
        let removed = prune_stale_self_managed(dir.path(), &[], &[]).unwrap();
        assert_eq!(removed, 1);

        let remaining = load_self_managed(dir.path()).unwrap();
        assert!(remaining.is_empty(), "marker should have been removed");
    }

    #[test]
    fn prune_preserves_marker_for_other_platform_entry_with_missing_file() {
        // Cross-platform safety: target NOT in current_platform_targets, IS in all_platform_targets,
        // and file does NOT exist on disk (e.g. macOS path checked from Linux).
        let dir = tempdir().unwrap();
        let target_str = "/nonexistent/macos/only/path".to_string();

        add_self_managed(dir.path(), make_sm_entry("macos-cfg", "/src/cfg", &target_str)).unwrap();

        // Not in current platform, but IS in all-platform unfiltered manifest.
        let current: Vec<String> = vec![];
        let all = vec![target_str.clone()];

        let removed = prune_stale_self_managed(dir.path(), &current, &all).unwrap();
        assert_eq!(removed, 0, "cross-platform marker must be preserved");

        let remaining = load_self_managed(dir.path()).unwrap();
        assert_eq!(remaining.len(), 1, "marker should still be present");
    }

    #[test]
    fn prune_preserves_marker_for_other_platform_entry_with_existing_file() {
        // Cross-platform safety: target NOT in current_platform_targets, IS in all_platform_targets,
        // and file exists on disk.
        let dir = tempdir().unwrap();
        let target = dir.path().join("other_platform_cfg");
        let target_str = target.to_string_lossy().to_string();

        std::fs::write(&target, "some config").unwrap();

        add_self_managed(dir.path(), make_sm_entry("other-cfg", "/src/cfg", &target_str)).unwrap();

        // Not in current platform, but IS in all-platform unfiltered manifest.
        let current: Vec<String> = vec![];
        let all = vec![target_str.clone()];

        let removed = prune_stale_self_managed(dir.path(), &current, &all).unwrap();
        assert_eq!(removed, 0, "cross-platform marker must be preserved");

        let remaining = load_self_managed(dir.path()).unwrap();
        assert_eq!(remaining.len(), 1, "marker should still be present");
    }

    #[test]
    fn prune_preserves_marker_when_in_current_manifest_and_file_exists() {
        // Baseline happy path: target in both slices AND file exists — should not be pruned.
        let dir = tempdir().unwrap();
        let target = dir.path().join("nvim_init.lua");
        let target_str = target.to_string_lossy().to_string();

        std::fs::write(&target, "local config").unwrap();

        add_self_managed(dir.path(), make_sm_entry("nvim", "/src/nvim/init.lua", &target_str))
            .unwrap();

        let current = vec![target_str.clone()];
        let all = vec![target_str.clone()];

        let removed = prune_stale_self_managed(dir.path(), &current, &all).unwrap();
        assert_eq!(removed, 0, "healthy marker must be preserved");

        let remaining = load_self_managed(dir.path()).unwrap();
        assert_eq!(remaining.len(), 1, "marker should still be present");
    }
}
