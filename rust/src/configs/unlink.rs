// unlink: remove symlinked configs

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use dialoguer::Confirm;
use dialoguer::theme::ColorfulTheme;

use crate::common::platform::Platform;
use crate::configs::manifest::{filter_by_name, load_manifest};
use crate::configs::state::{SelfManagedEntry, detect_state, is_self_managed, load_self_managed, remove_self_managed};
use crate::configs::{ConfigEntry, EntryState, display_target, format_source};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn run_unlink(
    project_root: &Path,
    platform: &Platform,
    filter_name: Option<&str>,
    yes: bool,
) -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/root"));
    let home_path = PathBuf::from(&home);

    let all_entries = load_manifest(project_root, platform)?;

    let entries: Vec<ConfigEntry> = if let Some(name) = filter_name {
        let filtered = filter_by_name(&all_entries, name);
        if filtered.is_empty() {
            let available: Vec<&str> = {
                let mut names: Vec<&str> =
                    all_entries.iter().map(|e| e.name.as_str()).collect();
                names.dedup();
                names
            };
            bail!(
                "No config named '{}'. Available: {}",
                name,
                available.join(", ")
            );
        }
        filtered
    } else {
        all_entries
    };

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
            EntryState::Linked => {
                // Remove the symlink.
                std::fs::remove_file(&entry.target).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to remove symlink {}: {}",
                        entry.target.display(),
                        e
                    )
                })?;

                // Also remove from self-managed list if present (clean up stale marker).
                if is_self_managed(self_managed, &entry.target) {
                    remove_self_managed(project_root, &entry.target.to_string_lossy())?;
                }

                // Check for a .bak file to restore.
                let bak_path = PathBuf::from(format!("{}.bak", entry.target.display()));
                let bak_exists = bak_path.exists() || bak_path.symlink_metadata().is_ok();

                if bak_exists {
                    let do_restore = if yes || !interactive {
                        true
                    } else {
                        Confirm::with_theme(&ColorfulTheme::default())
                            .with_prompt(format!(
                                "Restore backup {}?",
                                bak_path.display()
                            ))
                            .default(true)
                            .interact()
                            .map_err(|e| anyhow::anyhow!("Prompt failed: {}", e))?
                    };

                    if do_restore {
                        std::fs::rename(&bak_path, &entry.target).map_err(|e| {
                            anyhow::anyhow!(
                                "Failed to restore backup {} to {}: {}",
                                bak_path.display(),
                                entry.target.display(),
                                e
                            )
                        })?;
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
            EntryState::NotLinked | EntryState::Conflict | EntryState::WrongSymlink => {
                writeln!(
                    writer,
                    "  - {source_display} \u{2192} {target_display} (not linked, skipping)"
                )?;
            }
        }
    }

    Ok(())
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
        assert!(target.is_symlink(), "precondition: target should be symlink");

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
        assert!(!target.is_symlink(), "restored file should not be a symlink");
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

    // ── Test 9: Unlink skips WrongSymlink entries ─────────────────────────────

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
        assert!(target.is_symlink(), "wrong symlink should not have been touched");
        let dest = std::fs::read_link(&target).unwrap();
        assert_eq!(dest, other, "symlink should still point to original destination");
        assert!(output.contains("(not linked, skipping)"));
    }
}
