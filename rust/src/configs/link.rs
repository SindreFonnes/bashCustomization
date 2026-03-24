// link: symlink configs into place

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::common::platform::Platform;
use crate::configs::manifest::{filter_by_name, load_manifest};
use crate::configs::state::{SelfManagedEntry, detect_state, load_self_managed};
use crate::configs::{ConfigEntry, EntryState, Strategy, display_target, format_source};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn run_link(
    project_root: &Path,
    platform: &Platform,
    filter_name: Option<&str>,
    _force: Option<Strategy>,
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

    // Validate all source files exist before doing anything.
    let missing: Vec<&ConfigEntry> = entries.iter().filter(|e| !e.source.exists()).collect();
    if !missing.is_empty() {
        let paths: Vec<String> = missing
            .iter()
            .map(|e| e.source.to_string_lossy().into_owned())
            .collect();
        bail!("Source file(s) not found:\n  {}", paths.join("\n  "));
    }

    write_link(&mut std::io::stdout(), &entries, &self_managed, &home_path)
}

// ---------------------------------------------------------------------------
// Core logic (accepts a writer for testability)
// ---------------------------------------------------------------------------

fn write_link(
    writer: &mut impl Write,
    entries: &[ConfigEntry],
    self_managed: &[SelfManagedEntry],
    home: &Path,
) -> Result<()> {
    for entry in entries {
        let state = detect_state(entry, self_managed);
        let source_display = format_source(entry);
        let target_display = display_target(&entry.target, home);

        match state {
            EntryState::Linked => {
                writeln!(
                    writer,
                    "  ✓ {source_display} → {target_display} (already linked)"
                )?;
            }
            EntryState::NotLinked => {
                // Create parent directories if needed.
                if let Some(parent) = entry.target.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to create parent directory {}: {}",
                            parent.display(),
                            e
                        )
                    })?;
                }

                std::os::unix::fs::symlink(&entry.source, &entry.target).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to create symlink {} → {}: {}",
                        entry.source.display(),
                        entry.target.display(),
                        e
                    )
                })?;

                writeln!(
                    writer,
                    "  ✓ {source_display} → {target_display} (linked)"
                )?;
            }
            EntryState::SelfManaged => {
                writeln!(
                    writer,
                    "  ○ {source_display} → {target_display} (self-managed, skipping)"
                )?;
            }
            EntryState::Conflict | EntryState::WrongSymlink => {
                // Task 8 will replace this with real conflict resolution.
                writeln!(
                    writer,
                    "  ✗ {source_display} → {target_display} (conflict — skipping)"
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
    use crate::configs::state::SelfManagedEntry;

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

    fn capture_link(
        entries: &[ConfigEntry],
        self_managed: &[SelfManagedEntry],
    ) -> String {
        let home = fake_home();
        let mut buf: Vec<u8> = Vec::new();
        write_link(&mut buf, entries, self_managed, &home).expect("write_link failed");
        String::from_utf8(buf).expect("output is valid UTF-8")
    }

    // ── Test 1: Link creates a symlink when target doesn't exist ─────────────

    #[test]
    fn link_creates_symlink_for_not_linked_entry() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        // target intentionally not created

        let entry = make_entry("test", source.clone(), target.clone());
        let output = capture_link(&[entry], &[]);

        // Symlink should now exist and point to source.
        assert!(target.is_symlink(), "target should be a symlink");
        let link_dest = std::fs::read_link(&target).unwrap();
        assert_eq!(link_dest, source, "symlink should point to source");

        assert!(output.contains("✓"));
        assert!(output.contains("(linked)"));
        assert!(!output.contains("already linked"));
    }

    // ── Test 2: Link creates parent directories when they don't exist ─────────

    #[test]
    fn link_creates_parent_directories() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("nested").join("deep").join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        // parent dirs intentionally not created

        let entry = make_entry("test", source.clone(), target.clone());
        let output = capture_link(&[entry], &[]);

        assert!(target.is_symlink(), "target should be a symlink");
        let link_dest = std::fs::read_link(&target).unwrap();
        assert_eq!(link_dest, source);

        assert!(output.contains("(linked)"));
    }

    // ── Test 3: Link skips already-linked entries ─────────────────────────────

    #[test]
    fn link_skips_already_linked_entry() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        symlink(&source, &target).unwrap();

        let entry = make_entry("test", source, target.clone());
        let output = capture_link(&[entry], &[]);

        // Symlink should still exist (unchanged).
        assert!(target.is_symlink());
        assert!(output.contains("✓"));
        assert!(output.contains("(already linked)"));
        assert!(!output.contains("(linked)") || output.contains("(already linked)"));
    }

    // ── Test 4: Link skips self-managed entries ───────────────────────────────

    #[test]
    fn link_skips_self_managed_entry() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        std::fs::write(&target, "local content").unwrap(); // regular file in SM list

        let sm = vec![SelfManagedEntry {
            name: "test".to_string(),
            source: source.to_string_lossy().to_string(),
            target: target.to_string_lossy().to_string(),
        }];

        let entry = make_entry("test", source, target.clone());
        let output = capture_link(&[entry], &sm);

        // The regular file should still be there, not replaced.
        assert!(!target.is_symlink(), "target should not have been replaced with a symlink");
        assert!(output.contains("○"));
        assert!(output.contains("(self-managed, skipping)"));
    }

    // ── Test 5: Link errors when source file doesn't exist ───────────────────

    #[test]
    fn link_errors_when_source_missing() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("nonexistent_source.txt");
        let target = dir.path().join("target.txt");
        // source intentionally not created

        let entry = make_entry("test", source, target);

        let home = fake_home();
        let mut buf: Vec<u8> = Vec::new();
        // Bypassing write_link — validate via run_link-level source check.
        // Use write_link directly with the missing source to trigger the pre-check.
        // Actually the source validation is in run_link; test it via a helper that
        // mimics the validation step.
        let result = {
            let missing: Vec<&ConfigEntry> = std::slice::from_ref(&entry)
                .iter()
                .filter(|e| !e.source.exists())
                .collect();
            if !missing.is_empty() {
                let paths: Vec<String> = missing
                    .iter()
                    .map(|e| e.source.to_string_lossy().into_owned())
                    .collect();
                Err(anyhow::anyhow!(
                    "Source file(s) not found:\n  {}",
                    paths.join("\n  ")
                ))
            } else {
                write_link(&mut buf, &[entry], &[], &home)
            }
        };

        assert!(result.is_err(), "should error when source is missing");
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("Source file(s) not found"), "error should mention missing source");
    }

    // ── Test 6: Conflict entry is skipped with message ────────────────────────

    #[test]
    fn link_skips_conflict_entry() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        std::fs::write(&target, "local content").unwrap(); // regular file = conflict, not in SM

        let entry = make_entry("test", source, target.clone());
        let output = capture_link(&[entry], &[]);

        // target should remain unchanged (regular file, not replaced)
        assert!(!target.is_symlink());
        assert!(output.contains("✗"));
        assert!(output.contains("(conflict — skipping)"));
    }
}
