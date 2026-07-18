// unlink: remove symlinked configs

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use dialoguer::Confirm;
use dialoguer::theme::ColorfulTheme;

use crate::common::platform::Platform;
use crate::configs::manifest::{load_manifest, select_entries};
use crate::configs::state::{
    SelfManagedEntry, detect_state, is_self_managed, load_self_managed, remove_self_managed,
};
use crate::configs::{
    ConfigEntry, EntryState, display_target, format_source, home_dir, require_target_authority,
};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn run_unlink(
    project_root: &Path,
    platform: &Platform,
    filter_name: Option<&str>,
    yes: bool,
    allow_outside_home: bool,
) -> Result<()> {
    let home_path = home_dir()?;

    let entries = select_entries(load_manifest(project_root, platform)?, filter_name)?;

    require_target_authority(&entries, &home_path, allow_outside_home)?;

    let self_managed = load_self_managed(project_root)?;

    write_unlink(
        &mut std::io::stdout(),
        &entries,
        &self_managed,
        &home_path,
        project_root,
        yes,
        true, // interactive — enable dialoguer prompts
    )
}

// ---------------------------------------------------------------------------
// Core logic (accepts a writer for testability)
// ---------------------------------------------------------------------------

fn write_unlink(
    writer: &mut impl Write,
    entries: &[ConfigEntry],
    self_managed: &[SelfManagedEntry],
    home: &Path,
    project_root: &Path,
    yes: bool,
    interactive: bool,
) -> Result<()> {
    for entry in entries {
        let state = detect_state(entry, self_managed);
        let source_display = format_source(entry);
        let target_display = display_target(&entry.target, home);

        match state {
            EntryState::Linked | EntryState::LinkedMissingSource => {
                let bak_path = PathBuf::from(format!("{}.bak", entry.target.display()));
                let bak_exists = bak_path.exists() || bak_path.symlink_metadata().is_ok();
                let do_restore = if !bak_exists {
                    false
                } else if yes || !interactive {
                    true
                } else {
                    Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt(format!("Restore backup {}?", bak_path.display()))
                        .default(true)
                        .interact()
                        .map_err(|e| anyhow::anyhow!("Prompt failed: {}", e))?
                };

                if do_restore {
                    restore_backup_transactional(entry, &bak_path)?;
                } else {
                    std::fs::remove_file(&entry.target).map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to remove symlink {}: {}",
                            entry.target.display(),
                            e
                        )
                    })?;
                }

                // Also remove from self-managed list if present (clean up stale marker).
                if is_self_managed(self_managed, &entry.target) {
                    remove_self_managed(project_root, &entry.target.to_string_lossy())?;
                }

                if do_restore {
                    writeln!(
                        writer,
                        "  \u{2713} {source_display} \u{2192} {target_display} (unlinked, backup restored)"
                    )?;
                } else {
                    writeln!(
                        writer,
                        "  \u{2713} {source_display} \u{2192} {target_display} (unlinked)"
                    )?;
                }
            }
            EntryState::SelfManaged => {
                let do_remove = if yes || !interactive {
                    true
                } else {
                    Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt(format!(
                            "Remove self-managed marker for {}?",
                            target_display
                        ))
                        .default(false)
                        .interact()
                        .map_err(|e| anyhow::anyhow!("Prompt failed: {}", e))?
                };

                if do_remove {
                    remove_self_managed(project_root, &entry.target.to_string_lossy())?;
                    writeln!(
                        writer,
                        "  \u{25CB} {source_display} \u{2192} {target_display} (self-managed marker removed)"
                    )?;
                } else {
                    writeln!(
                        writer,
                        "  \u{25CB} {source_display} \u{2192} {target_display} (self-managed, skipped)"
                    )?;
                }
            }
            EntryState::NotLinked | EntryState::NotLinkedMissingSource => {
                // Stale self-managed: marker present but file gone — clean up
                // the marker so the user has visible feedback when invoking
                // unlink directly (the shell-startup `check` also prunes these,
                // but a user running `unlink` explicitly should not be left
                // wondering whether anything happened).
                if is_self_managed(self_managed, &entry.target) {
                    remove_self_managed(project_root, &entry.target.to_string_lossy())?;
                    writeln!(
                        writer,
                        "  \u{2713} {source_display} \u{2192} {target_display} (stale self-managed marker removed)"
                    )?;
                } else {
                    writeln!(
                        writer,
                        "  - {source_display} \u{2192} {target_display} (not linked, skipping)"
                    )?;
                }
            }
            EntryState::Conflict | EntryState::WrongSymlink => {
                writeln!(
                    writer,
                    "  - {source_display} \u{2192} {target_display} (not linked, skipping)"
                )?;
            }
        }
    }

    Ok(())
}

fn restore_backup_transactional(entry: &ConfigEntry, backup: &Path) -> Result<()> {
    restore_backup_transactional_with(entry, backup, |from, to| {
        std::fs::rename(from, to).map_err(anyhow::Error::from)
    })
}

fn restore_backup_transactional_with(
    entry: &ConfigEntry,
    backup: &Path,
    restore: impl FnOnce(&Path, &Path) -> Result<()>,
) -> Result<()> {
    let parent = entry.target.parent().ok_or_else(|| {
        anyhow::anyhow!(
            "Cannot stage symlink without a parent: {}",
            entry.target.display()
        )
    })?;
    let staging = tempfile::Builder::new()
        .prefix(".bashc-unlink-")
        .tempdir_in(parent)
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to create unlink staging directory in {}: {}",
                parent.display(),
                e
            )
        })?;
    let staged_link = staging.path().join("managed-link");

    std::fs::rename(&entry.target, &staged_link).map_err(|e| {
        anyhow::anyhow!(
            "Failed to stage symlink {} at {}: {}",
            entry.target.display(),
            staged_link.display(),
            e
        )
    })?;

    if let Err(restore_error) = restore(backup, &entry.target) {
        return match std::fs::rename(&staged_link, &entry.target) {
            Ok(()) => Err(restore_error.context("backup restore failed; managed link restored")),
            Err(rollback_error) => {
                let recovery_dir = staging.keep();
                Err(anyhow::anyhow!(
                    "Backup restore failed: {restore_error:#}. Restoring the managed link also failed: {rollback_error}. The link was kept at {}",
                    recovery_dir.join("managed-link").display()
                ))
            }
        };
    }

    staging
        .close()
        .map_err(|e| anyhow::anyhow!("Backup restored but failed to remove the staged link: {e}"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    use crate::configs::Strategy;
    use crate::configs::state::{SelfManagedEntry, add_self_managed, load_self_managed};

    fn fake_home() -> PathBuf {
        PathBuf::from("/home/testuser")
    }

    fn make_entry(name: &str, source: PathBuf, target: PathBuf) -> ConfigEntry {
        ConfigEntry {
            name: name.to_string(),
            source,
            target,
            strategy: Strategy::Prompt,
        }
    }

    /// Run write_unlink with yes=true and non-interactive (for tests).
    fn capture_unlink(
        entries: &[ConfigEntry],
        self_managed: &[SelfManagedEntry],
        project_root: &Path,
        yes: bool,
    ) -> String {
        let home = fake_home();
        let mut buf: Vec<u8> = Vec::new();
        write_unlink(
            &mut buf,
            entries,
            self_managed,
            &home,
            project_root,
            yes,
            false, // non-interactive: skips dialoguer
        )
        .expect("write_unlink failed");
        String::from_utf8(buf).expect("output is valid UTF-8")
    }

    // ── Test 1: Unlink removes a symlink ─────────────────────────────────────

    #[test]
    fn unlink_removes_symlink() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        symlink(&source, &target).unwrap();
        assert!(
            target.is_symlink(),
            "precondition: target should be symlink"
        );

        let entry = make_entry("test", source.clone(), target.clone());
        let output = capture_unlink(&[entry], &[], dir.path(), true);

        assert!(!target.exists(), "symlink should have been removed");
        assert!(!target.is_symlink(), "symlink should no longer exist");
        assert!(output.contains("\u{2713}"));
        assert!(output.contains("(unlinked)"));
    }

    // ── Test 2: Unlink without .bak prints simple unlinked message ────────────

    #[test]
    fn unlink_without_bak_prints_unlinked() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        symlink(&source, &target).unwrap();

        let entry = make_entry("test", source.clone(), target.clone());
        let output = capture_unlink(&[entry], &[], dir.path(), true);

        assert!(!target.is_symlink());
        assert!(output.contains("(unlinked)"));
        assert!(!output.contains("backup restored"));
    }

    // ── Test 3: Unlink with .bak restores backup when yes=true ───────────────

    #[test]
    fn unlink_with_bak_restores_backup_when_yes() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");
        let bak_path = PathBuf::from(format!("{}.bak", target.display()));

        std::fs::write(&source, "repo content").unwrap();
        std::fs::write(&bak_path, "original content").unwrap();
        symlink(&source, &target).unwrap();

        let entry = make_entry("test", source.clone(), target.clone());
        let output = capture_unlink(&[entry], &[], dir.path(), true);

        // Symlink should be gone.
        assert!(!target.is_symlink(), "symlink should have been removed");
        // Backup should have been restored as a regular file.
        assert!(target.exists(), "backup should have been restored");
        assert!(
            !target.is_symlink(),
            "restored file should not be a symlink"
        );
        let content = std::fs::read_to_string(&target).unwrap();
        assert_eq!(content, "original content");
        // .bak should be gone after restore.
        assert!(!bak_path.exists(), ".bak should be removed after restore");

        assert!(output.contains("(unlinked, backup restored)"));
    }

    // ── Test 4: Unlink with .bak, yes=false (non-interactive) behaves like yes ─

    #[test]
    fn unlink_with_bak_restores_when_non_interactive() {
        // When interactive=false, we treat it as yes=true (for testability).
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");
        let bak_path = PathBuf::from(format!("{}.bak", target.display()));

        std::fs::write(&source, "repo content").unwrap();
        std::fs::write(&bak_path, "original content").unwrap();
        symlink(&source, &target).unwrap();

        let entry = make_entry("test", source.clone(), target.clone());
        // yes=false but non-interactive — should still restore
        let output = capture_unlink(&[entry], &[], dir.path(), false);

        assert!(!target.is_symlink());
        assert!(target.exists());
        let content = std::fs::read_to_string(&target).unwrap();
        assert_eq!(content, "original content");
        assert!(output.contains("(unlinked, backup restored)"));
    }

    #[test]
    fn failed_backup_restore_preserves_managed_symlink() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");
        let backup = PathBuf::from(format!("{}.bak", target.display()));
        std::fs::write(&source, "repo content").unwrap();
        std::fs::write(&backup, "original content").unwrap();
        symlink(&source, &target).unwrap();

        let entry = make_entry("test", source.clone(), target.clone());
        let error = restore_backup_transactional_with(&entry, &backup, |_, _| {
            Err(anyhow::anyhow!("injected restore failure"))
        })
        .expect_err("injected restore failure should be returned");

        assert!(error.to_string().contains("managed link restored"));
        assert!(target.is_symlink());
        assert_eq!(std::fs::read_link(&target).unwrap(), source);
        assert_eq!(
            std::fs::read_to_string(&backup).unwrap(),
            "original content"
        );
    }

    #[test]
    fn failed_backup_and_link_rollback_keeps_recovery_symlink() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");
        let backup = PathBuf::from(format!("{}.bak", target.display()));
        std::fs::write(&source, "repo content").unwrap();
        std::fs::write(&backup, "original content").unwrap();
        symlink(&source, &target).unwrap();

        let entry = make_entry("test", source.clone(), target.clone());
        let error = restore_backup_transactional_with(&entry, &backup, |_, target| {
            std::fs::create_dir(target)?;
            Err(anyhow::anyhow!("injected restore failure"))
        })
        .expect_err("injected restore and rollback failure should be returned");

        assert!(error.to_string().contains("The link was kept at"));
        let recovery_dir = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .find(|candidate| {
                candidate
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".bashc-unlink-")
            })
            .expect("recovery directory should persist");
        let recovery_link = recovery_dir.path().join("managed-link");
        assert!(recovery_link.is_symlink());
        assert_eq!(std::fs::read_link(recovery_link).unwrap(), source);
        assert_eq!(
            std::fs::read_to_string(&backup).unwrap(),
            "original content"
        );
    }

    // ── Test 5: Unlink removes self-managed marker when yes=true ─────────────

    #[test]
    fn unlink_removes_self_managed_marker_when_yes() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "repo").unwrap();
        std::fs::write(&target, "local").unwrap();

        // Register as self-managed.
        add_self_managed(
            dir.path(),
            SelfManagedEntry {
                name: "test".to_string(),
                source: source.to_string_lossy().to_string(),
                target: target.to_string_lossy().to_string(),
            },
        )
        .unwrap();

        let sm = load_self_managed(dir.path()).unwrap();
        assert_eq!(sm.len(), 1, "precondition: sm entry should exist");

        let entry = make_entry("test", source.clone(), target.clone());
        let output = capture_unlink(&[entry], &sm, dir.path(), true);

        // Self-managed entry should be removed.
        let sm_after = load_self_managed(dir.path()).unwrap();
        assert!(sm_after.is_empty(), "self-managed entry should be removed");

        // Local file should remain (we only remove the marker, not the file).
        assert!(target.exists(), "local file should still exist");

        assert!(output.contains("(self-managed marker removed)"));
    }

    // ── Test 6: Unlink skips NotLinked entries ────────────────────────────────

    #[test]
    fn unlink_skips_not_linked_entry() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        // target intentionally not created → NotLinked

        let entry = make_entry("test", source.clone(), target.clone());
        let output = capture_unlink(&[entry], &[], dir.path(), true);

        assert!(!target.exists(), "target should still not exist");
        assert!(output.contains("(not linked, skipping)"));
    }

    // ── Test 7: Unlink skips Conflict entries ─────────────────────────────────

    #[test]
    fn unlink_skips_conflict_entry() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        std::fs::write(&target, "local content").unwrap(); // regular file = Conflict

        let entry = make_entry("test", source.clone(), target.clone());
        let output = capture_unlink(&[entry], &[], dir.path(), true);

        // Regular file should remain untouched.
        assert!(!target.is_symlink());
        let content = std::fs::read_to_string(&target).unwrap();
        assert_eq!(content, "local content");
        assert!(output.contains("(not linked, skipping)"));
    }

    // ── Test 8: Unlink also removes self-managed marker for Linked entry ──────

    #[test]
    fn unlink_removes_self_managed_marker_for_linked_entry() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "repo").unwrap();
        symlink(&source, &target).unwrap();

        // Simulate stale self-managed marker for a linked entry (edge case).
        add_self_managed(
            dir.path(),
            SelfManagedEntry {
                name: "test".to_string(),
                source: source.to_string_lossy().to_string(),
                target: target.to_string_lossy().to_string(),
            },
        )
        .unwrap();

        let sm = load_self_managed(dir.path()).unwrap();
        let entry = make_entry("test", source.clone(), target.clone());
        let output = capture_unlink(&[entry], &sm, dir.path(), true);

        assert!(!target.exists());
        // Self-managed marker should have been cleaned up.
        let sm_after = load_self_managed(dir.path()).unwrap();
        assert!(sm_after.is_empty(), "stale sm marker should be removed");
        assert!(output.contains("(unlinked)"));
    }

    // ── Test 9: Unlink cleans up stale self-managed marker (NotLinked) ───────

    #[test]
    fn unlink_removes_stale_self_managed_marker_when_target_missing() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("missing_target.txt");

        std::fs::write(&source, "hello").unwrap();
        // target intentionally never created → NotLinked

        // Marker present but file is gone — stale.
        add_self_managed(
            dir.path(),
            SelfManagedEntry {
                name: "test".to_string(),
                source: source.to_string_lossy().to_string(),
                target: target.to_string_lossy().to_string(),
            },
        )
        .unwrap();

        let sm = load_self_managed(dir.path()).unwrap();
        assert_eq!(sm.len(), 1, "precondition: marker should exist");

        let entry = make_entry("test", source, target.clone());
        let output = capture_unlink(&[entry], &sm, dir.path(), true);

        // Marker should have been pruned.
        let sm_after = load_self_managed(dir.path()).unwrap();
        assert!(sm_after.is_empty(), "stale marker should be removed");

        // No new file should have been created.
        assert!(!target.exists());

        assert!(
            output.contains("stale self-managed marker removed"),
            "expected stale-marker message, got: {output}"
        );
    }

    // ── Test 10: Unlink skips WrongSymlink entries ────────────────────────────

    #[test]
    fn unlink_skips_wrong_symlink_entry() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let other = dir.path().join("other.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        std::fs::write(&other, "other").unwrap();
        symlink(&other, &target).unwrap(); // WrongSymlink

        let entry = make_entry("test", source.clone(), target.clone());
        let output = capture_unlink(&[entry], &[], dir.path(), true);

        // Wrong symlink should remain untouched.
        assert!(
            target.is_symlink(),
            "wrong symlink should not have been touched"
        );
        let dest = std::fs::read_link(&target).unwrap();
        assert_eq!(
            dest, other,
            "symlink should still point to original destination"
        );
        assert!(output.contains("(not linked, skipping)"));
    }
}
