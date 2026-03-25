// status: report on linked/unlinked config state

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::common::platform::Platform;
use crate::configs::manifest::{filter_by_name, load_manifest};
use crate::configs::state::{SelfManagedEntry, detect_state, is_self_managed, load_self_managed};
use crate::configs::{ConfigEntry, EntryState, display_target, format_source};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn run_status(
    project_root: &Path,
    platform: &Platform,
    filter_name: Option<&str>,
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

    write_status(&mut std::io::stdout(), &entries, &self_managed, &home_path)
}

// ---------------------------------------------------------------------------
// Core formatting logic (accepts a writer for testability)
// ---------------------------------------------------------------------------

/// Group entries by name (preserving order of first appearance) and write
/// the formatted status table to `writer`.
pub(crate) fn write_status(
    writer: &mut impl Write,
    entries: &[ConfigEntry],
    self_managed: &[SelfManagedEntry],
    home: &Path,
) -> Result<()> {
    // Collect group names in order of first appearance.
    let mut group_order: Vec<&str> = Vec::new();
    for entry in entries {
        let name = entry.name.as_str();
        if !group_order.contains(&name) {
            group_order.push(name);
        }
    }

    for group_name in group_order {
        writeln!(writer, "{group_name}:")?;

        let group_entries: Vec<&ConfigEntry> =
            entries.iter().filter(|e| e.name == group_name).collect();

        for entry in group_entries {
            let state = detect_state(entry, self_managed);

            // Stale self-managed: in the SM list but file gone → note it.
            let stale_sm = matches!(state, EntryState::NotLinked)
                && is_self_managed(self_managed, &entry.target);

            let source_display = format_source(entry);
            let target_display = display_target(&entry.target, home);

            let line = match state {
                EntryState::Linked => {
                    format!("  ✓ {source_display} → {target_display}")
                }
                EntryState::SelfManaged => {
                    format!("  ○ {source_display} → {target_display} [self-managed]")
                }
                EntryState::WrongSymlink => {
                    format!(
                        "  ✗ {source_display} → {target_display} [conflict: symlink points elsewhere]"
                    )
                }
                EntryState::Conflict => {
                    format!(
                        "  ✗ {source_display} → {target_display} [conflict: local file exists]"
                    )
                }
                EntryState::NotLinked if stale_sm => {
                    format!(
                        "  - {source_display} → {target_display} [not linked] (stale self-managed entry)"
                    )
                }
                EntryState::NotLinked => {
                    format!("  - {source_display} → {target_display} [not linked]")
                }
            };

            writeln!(writer, "{line}")?;
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

    const FAKE_HOME: &str = "/home/testuser";

    fn fake_home() -> PathBuf {
        PathBuf::from(FAKE_HOME)
    }

    /// Build a minimal ConfigEntry for tests.
    fn make_entry(name: &str, source: PathBuf, target: PathBuf) -> ConfigEntry {
        ConfigEntry {
            name: name.to_string(),
            source,
            target,
            strategy: Strategy::Prompt,
        }
    }

    // ── display_target ────────────────────────────────────────────────────────

    #[test]
    fn display_target_replaces_home_prefix() {
        let home = fake_home();
        let target = PathBuf::from("/home/testuser/.claude/CLAUDE.md");
        assert_eq!(display_target(&target, &home), "~/.claude/CLAUDE.md");
    }

    #[test]
    fn display_target_keeps_non_home_path_unchanged() {
        let home = fake_home();
        let target = PathBuf::from("/etc/myconfig");
        assert_eq!(display_target(&target, &home), "/etc/myconfig");
    }

    #[test]
    fn display_target_handles_exact_home() {
        let home = fake_home();
        let target = fake_home();
        // strip_prefix on equal paths yields "" → displayed as "~/"
        let result = display_target(&target, &home);
        assert!(result.starts_with('~'), "expected ~ prefix, got: {result}");
    }

    // ── format_source ─────────────────────────────────────────────────────────

    #[test]
    fn format_source_returns_last_two_components() {
        let entry = make_entry(
            "claude",
            PathBuf::from("/repo/configs/claude/CLAUDE.md"),
            PathBuf::from("/home/testuser/.claude/CLAUDE.md"),
        );
        assert_eq!(format_source(&entry), "claude/CLAUDE.md");
    }

    #[test]
    fn format_source_handles_shallow_path() {
        // If path has fewer than 2 components (unlikely in practice) fall back to full path.
        let entry = make_entry(
            "x",
            PathBuf::from("config"),
            PathBuf::from("/home/testuser/.config"),
        );
        assert_eq!(format_source(&entry), "config");
    }

    // ── write_status output ───────────────────────────────────────────────────

    fn capture_status(
        entries: &[ConfigEntry],
        self_managed: &[SelfManagedEntry],
    ) -> String {
        let home = fake_home();
        let mut buf: Vec<u8> = Vec::new();
        write_status(&mut buf, entries, self_managed, &home).expect("write_status failed");
        String::from_utf8(buf).expect("output is valid UTF-8")
    }

    #[test]
    fn linked_entry_shows_checkmark() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("CLAUDE.md");
        let target = dir.path().join("target_CLAUDE.md");
        std::fs::write(&source, "# content").unwrap();
        symlink(&source, &target).unwrap();

        let entry = make_entry(
            "claude",
            source,
            target,
        );

        let output = capture_status(&[entry], &[]);
        assert!(output.contains("claude:"), "missing group header");
        assert!(output.contains("✓"), "missing check mark for linked state");
        assert!(!output.contains("[not linked]"), "should not show not-linked");
    }

    #[test]
    fn not_linked_entry_shows_dash() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("config.kdl");
        let target = dir.path().join("missing_target.kdl");
        std::fs::write(&source, "").unwrap();
        // target intentionally absent

        let entry = make_entry("zellij", source, target);

        let output = capture_status(&[entry], &[]);
        assert!(output.contains("zellij:"));
        assert!(output.contains("- "));
        assert!(output.contains("[not linked]"));
    }

    #[test]
    fn conflict_entry_shows_cross_with_message() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.kdl");
        let target = dir.path().join("target.kdl");
        std::fs::write(&source, "").unwrap();
        std::fs::write(&target, "local content").unwrap(); // regular file = conflict

        let entry = make_entry("zellij", source, target);

        let output = capture_status(&[entry], &[]);
        assert!(output.contains("✗"));
        assert!(output.contains("[conflict: local file exists]"));
    }

    #[test]
    fn wrong_symlink_shows_cross_with_message() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.kdl");
        let other = dir.path().join("other.kdl");
        let target = dir.path().join("target.kdl");
        std::fs::write(&source, "").unwrap();
        std::fs::write(&other, "").unwrap();
        symlink(&other, &target).unwrap(); // points to wrong place

        let entry = make_entry("zellij", source, target);

        let output = capture_status(&[entry], &[]);
        assert!(output.contains("✗"));
        assert!(output.contains("[conflict: symlink points elsewhere]"));
    }

    #[test]
    fn self_managed_entry_shows_circle() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("settings.json");
        let target = dir.path().join("settings_target.json");
        std::fs::write(&source, "{}").unwrap();
        std::fs::write(&target, "{}").unwrap(); // regular file in SM list

        let sm = vec![SelfManagedEntry {
            name: "claude".to_string(),
            source: source.to_string_lossy().to_string(),
            target: target.to_string_lossy().to_string(),
        }];

        let entry = make_entry("claude", source, target);

        let output = capture_status(&[entry], &sm);
        assert!(output.contains("○"));
        assert!(output.contains("[self-managed]"));
    }

    #[test]
    fn stale_self_managed_shows_note() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.json");
        let target = dir.path().join("gone_target.json");
        std::fs::write(&source, "{}").unwrap();
        // target never created (stale SM entry)

        let sm = vec![SelfManagedEntry {
            name: "claude".to_string(),
            source: source.to_string_lossy().to_string(),
            target: target.to_string_lossy().to_string(),
        }];

        let entry = make_entry("claude", source, target);

        let output = capture_status(&[entry], &sm);
        assert!(output.contains("[not linked]"));
        assert!(output.contains("stale self-managed entry"));
    }

    #[test]
    fn multiple_groups_shown_in_order() {
        let dir = tempdir().unwrap();

        let s1 = dir.path().join("CLAUDE.md");
        let t1 = dir.path().join("t1");
        std::fs::write(&s1, "").unwrap();

        let s2 = dir.path().join("config.kdl");
        let t2 = dir.path().join("t2");
        std::fs::write(&s2, "").unwrap();

        let entries = vec![
            make_entry("claude", s1, t1),
            make_entry("zellij", s2, t2),
        ];

        let output = capture_status(&entries, &[]);

        let pos_claude = output.find("claude:").expect("claude group missing");
        let pos_zellij = output.find("zellij:").expect("zellij group missing");
        assert!(
            pos_claude < pos_zellij,
            "claude should appear before zellij"
        );
    }

    #[test]
    fn multiple_entries_in_same_group() {
        let dir = tempdir().unwrap();

        let s1 = dir.path().join("CLAUDE.md");
        let t1 = dir.path().join("t_md");
        std::fs::write(&s1, "").unwrap();

        let s2 = dir.path().join("settings.json");
        let t2 = dir.path().join("t_json");
        std::fs::write(&s2, "").unwrap();
        symlink(&s2, &t2).unwrap(); // second one linked

        let entries = vec![
            make_entry("claude", s1, t1),        // not linked
            make_entry("claude", s2, t2),        // linked
        ];

        let output = capture_status(&entries, &[]);

        // Header appears only once
        assert_eq!(output.matches("claude:").count(), 1);
        // Both states represented
        assert!(output.contains("[not linked]"));
        assert!(output.contains("✓"));
    }

    #[test]
    fn target_path_shown_with_tilde() {
        let dir = tempdir().unwrap();
        let home = PathBuf::from(FAKE_HOME);

        // Create a "target" under the fake home directory so the path
        // substitution has something real to work with.
        let target = home.join(".claude").join("CLAUDE.md");

        let source = dir.path().join("CLAUDE.md");
        std::fs::write(&source, "").unwrap();

        let entry = make_entry("claude", source, target);

        let mut buf: Vec<u8> = Vec::new();
        write_status(&mut buf, &[entry], &[], &home).expect("write_status failed");
        let output = String::from_utf8(buf).unwrap();

        assert!(
            output.contains("~/.claude/CLAUDE.md"),
            "expected tilde path, got:\n{output}"
        );
    }
}
