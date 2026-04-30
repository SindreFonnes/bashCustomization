// check: auto-link safe drift and warn about anything that needs attention.
// Designed for shell-startup invocation.

use std::io::Write;
use std::path::Path;

use anyhow::Result;

use crate::common::platform::Platform;
use crate::configs::link::create_symlink;
use crate::configs::manifest::{load_manifest, load_manifest_unfiltered};
use crate::configs::state::{
    detect_state, load_self_managed, prune_stale_self_managed, remove_self_managed,
    SelfManagedEntry,
};
use crate::configs::{ConfigEntry, EntryState};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn run_check(project_root: &Path, platform: &Platform) -> Result<()> {
    let filtered_entries = load_manifest(project_root, platform)?;
    let unfiltered_entries = load_manifest_unfiltered(project_root)?;
    let self_managed = load_self_managed(project_root)?;

    write_check(
        &mut std::io::stdout(),
        &filtered_entries,
        &unfiltered_entries,
        &self_managed,
        project_root,
    )
}

// ---------------------------------------------------------------------------
// Core logic (accepts a writer for testability)
// ---------------------------------------------------------------------------

fn write_check(
    writer: &mut impl Write,
    filtered_entries: &[ConfigEntry],
    unfiltered_entries: &[ConfigEntry],
    self_managed: &[SelfManagedEntry],
    project_root: &Path,
) -> Result<()> {
    // linked_count tracks the raw number of symlinks created (before name dedup).
    let mut linked_count: usize = 0;
    let mut linked_names: Vec<String> = Vec::new();
    let mut drift_items: Vec<(String, &'static str)> = Vec::new();

    for entry in filtered_entries {
        let state = detect_state(entry, self_managed);

        match state {
            EntryState::Linked => {
                // Silent — no action needed.
            }
            EntryState::LinkedMissingSource => {
                drift_items.push((entry.name.clone(), "missing source"));
            }
            EntryState::SelfManaged => {
                // Silent — no action needed.
            }
            EntryState::NotLinked => {
                create_symlink(entry)?;
                linked_count += 1;
                linked_names.push(entry.name.clone());
                // If a stale SM marker exists for this target, remove it — the
                // entry is now a properly managed symlink, not a local override.
                let target_str = entry.target.to_string_lossy();
                if self_managed.iter().any(|e| e.target == target_str.as_ref()) {
                    remove_self_managed(project_root, &target_str)?;
                }
            }
            EntryState::NotLinkedMissingSource => {
                drift_items.push((entry.name.clone(), "missing source"));
            }
            EntryState::Conflict => {
                drift_items.push((entry.name.clone(), "conflict"));
            }
            EntryState::WrongSymlink => {
                drift_items.push((entry.name.clone(), "wrong symlink"));
            }
        }
    }

    // Prune stale self-managed markers.
    let current_platform_targets: Vec<String> = filtered_entries
        .iter()
        .map(|e| e.target.to_string_lossy().to_string())
        .collect();
    let all_platform_targets: Vec<String> = unfiltered_entries
        .iter()
        .map(|e| e.target.to_string_lossy().to_string())
        .collect();
    prune_stale_self_managed(
        project_root,
        &current_platform_targets,
        &all_platform_targets,
    )?;

    // Emit output lines.
    //
    // The count is the raw number of *entries* (i.e. one per manifest row),
    // while the displayed names are de-duplicated by group. We say "config
    // files" rather than "configs" so the count and names line up: a manifest
    // with two `claude` rows that both auto-link prints "linked 2 config files
    // (claude)" rather than the misleading "linked 2 configs (claude)".
    if linked_count > 0 {
        linked_names.sort();
        linked_names.dedup();
        let names_display = linked_names.join(", ");
        writeln!(
            writer,
            "bashc: linked {linked_count} config files ({names_display})"
        )?;
    }

    if !drift_items.is_empty() {
        let count = drift_items.len();
        let items_display: String = {
            let mut d = drift_items;
            d.sort_by(|a, b| a.0.cmp(&b.0));
            d.iter()
                .map(|(name, tag)| format!("{name}: {tag}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        writeln!(
            writer,
            "bashc: \u{26a0} {count} config files need attention ({items_display}) \u{2014} run 'bashc configs status'"
        )?;
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

    use crate::configs::state::{add_self_managed, load_self_managed, SelfManagedEntry};
    use crate::configs::Strategy;

    fn make_entry(name: &str, source: &Path, target: &Path) -> ConfigEntry {
        ConfigEntry {
            name: name.to_string(),
            source: source.to_path_buf(),
            target: target.to_path_buf(),
            strategy: Strategy::Prompt,
        }
    }

    /// Run write_check with the unfiltered slice equal to the filtered slice
    /// (no cross-platform entries). Captures stdout into a String.
    fn capture_check(
        entries: &[ConfigEntry],
        self_managed: &[SelfManagedEntry],
        project_root: &Path,
    ) -> String {
        let unfiltered = entries.to_vec();
        let mut buf: Vec<u8> = Vec::new();
        write_check(&mut buf, entries, &unfiltered, self_managed, project_root)
            .expect("write_check failed");
        String::from_utf8(buf).expect("output is valid UTF-8")
    }

    // ── Test 1: Silent when all linked ────────────────────────────────────────

    #[test]
    fn check_silent_when_all_linked() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        symlink(&source, &target).unwrap();

        let entry = make_entry("test", &source, &target);
        let output = capture_check(&[entry], &[], dir.path());

        assert!(
            output.is_empty(),
            "output should be empty when all linked, got: {output:?}"
        );
        assert!(target.is_symlink(), "target should still be a symlink");
    }

    // ── Test 2: Silent when self-managed ─────────────────────────────────────

    #[test]
    fn check_silent_when_self_managed() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        std::fs::write(&target, "local content").unwrap();

        let sm = vec![SelfManagedEntry {
            name: "test".to_string(),
            source: source.to_string_lossy().to_string(),
            target: target.to_string_lossy().to_string(),
        }];

        let entry = make_entry("test", &source, &target);
        let output = capture_check(&[entry], &sm, dir.path());

        assert!(
            output.is_empty(),
            "output should be empty for self-managed, got: {output:?}"
        );
        // Target should remain a regular file, not a symlink.
        assert!(
            !target.is_symlink(),
            "target should not have been replaced with a symlink"
        );
        let content = std::fs::read_to_string(&target).unwrap();
        assert_eq!(content, "local content");
    }

    // ── Test 3: Auto-links NotLinked when target absent ───────────────────────

    #[test]
    fn check_auto_links_not_linked_when_target_absent() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        // target intentionally not created

        let entry = make_entry("test", &source, &target);
        let output = capture_check(&[entry], &[], dir.path());

        assert!(target.is_symlink(), "target should become a symlink");
        let link_dest = std::fs::read_link(&target).unwrap();
        assert_eq!(link_dest, source, "symlink should point to source");

        assert!(
            output.contains("linked 1 config files"),
            "output should mention linked count, got: {output:?}"
        );
        assert!(
            output.contains("test"),
            "output should mention the entry name, got: {output:?}"
        );
    }

    // ── Test 4: Auto-links multiple entries, dedups names in summary ──────────

    #[test]
    fn check_auto_links_multiple_entries_and_dedups_names_in_summary() {
        let dir = tempdir().unwrap();

        let source1 = dir.path().join("source1.txt");
        let target1 = dir.path().join("target1.txt");
        let source2 = dir.path().join("source2.txt");
        let target2 = dir.path().join("target2.txt");
        let source3 = dir.path().join("source3.txt");
        let target3 = dir.path().join("target3.txt");

        std::fs::write(&source1, "hello").unwrap();
        std::fs::write(&source2, "hello").unwrap();
        std::fs::write(&source3, "hello").unwrap();
        // No targets created

        let entries = vec![
            make_entry("claude", &source1, &target1),
            make_entry("claude", &source2, &target2),
            make_entry("zellij", &source3, &target3),
        ];

        let mut buf: Vec<u8> = Vec::new();
        let unfiltered = entries.clone();
        write_check(&mut buf, &entries, &unfiltered, &[], dir.path()).expect("write_check failed");
        let output = String::from_utf8(buf).unwrap();

        assert!(target1.is_symlink(), "target1 should be a symlink");
        assert!(target2.is_symlink(), "target2 should be a symlink");
        assert!(target3.is_symlink(), "target3 should be a symlink");

        assert!(
            output.contains("linked 3 config files"),
            "should say '3 config files', got: {output:?}"
        );
        assert!(
            output.contains("claude"),
            "should mention 'claude', got: {output:?}"
        );
        assert!(
            output.contains("zellij"),
            "should mention 'zellij', got: {output:?}"
        );

        // Check order: alphabetized, deduped
        let claude_pos = output.find("claude").unwrap();
        let zellij_pos = output.find("zellij").unwrap();
        assert!(
            claude_pos < zellij_pos,
            "claude should appear before zellij (alphabetical)"
        );

        // Should not have "claude" twice
        let claude_count = output.matches("claude").count();
        assert_eq!(
            claude_count, 1,
            "claude should appear only once (deduped), got: {output:?}"
        );
    }

    // ── Test 5: Warns on conflict without modifying file ──────────────────────

    #[test]
    fn check_warns_on_conflict_without_modifying_file() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "repo content").unwrap();
        std::fs::write(&target, "local content").unwrap();

        let entry = make_entry("myconfig", &source, &target);
        let output = capture_check(&[entry], &[], dir.path());

        // Target should remain unchanged.
        assert!(!target.is_symlink(), "target should not be a symlink");
        let content = std::fs::read_to_string(&target).unwrap();
        assert_eq!(
            content, "local content",
            "target content should be unchanged"
        );

        assert!(
            output.contains("\u{26a0}"),
            "output should contain warning symbol, got: {output:?}"
        );
        assert!(
            output.contains("1 config files need attention"),
            "output should mention count, got: {output:?}"
        );
        assert!(
            output.contains("conflict"),
            "output should mention 'conflict', got: {output:?}"
        );
    }

    // ── Test 6: Warns on wrong symlink without modifying ─────────────────────

    #[test]
    fn check_warns_on_wrong_symlink_without_modifying() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let other = dir.path().join("other.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "repo content").unwrap();
        std::fs::write(&other, "other content").unwrap();
        symlink(&other, &target).unwrap();

        let entry = make_entry("myconfig", &source, &target);
        let output = capture_check(&[entry], &[], dir.path());

        // Symlink should still point to 'other', not to 'source'.
        assert!(target.is_symlink(), "target should still be a symlink");
        let link_dest = std::fs::read_link(&target).unwrap();
        assert_eq!(link_dest, other, "symlink should still point to other");

        assert!(
            output.contains("wrong symlink"),
            "output should mention 'wrong symlink', got: {output:?}"
        );
    }

    // ── Test 7: Mixed auto-links safe and warns unsafe ────────────────────────

    #[test]
    fn check_mixed_auto_links_safe_and_warns_unsafe() {
        let dir = tempdir().unwrap();
        let source1 = dir.path().join("source1.txt");
        let target1 = dir.path().join("target1.txt"); // NotLinked, target absent
        let source2 = dir.path().join("source2.txt");
        let target2 = dir.path().join("target2.txt"); // Conflict, regular file

        std::fs::write(&source1, "repo1").unwrap();
        // target1 intentionally not created
        std::fs::write(&source2, "repo2").unwrap();
        std::fs::write(&target2, "local content").unwrap();

        let entries = vec![
            make_entry("safe", &source1, &target1),
            make_entry("unsafe", &source2, &target2),
        ];

        let mut buf: Vec<u8> = Vec::new();
        let unfiltered = entries.clone();
        write_check(&mut buf, &entries, &unfiltered, &[], dir.path()).expect("write_check failed");
        let output = String::from_utf8(buf).unwrap();

        // First entry should become a symlink.
        assert!(target1.is_symlink(), "target1 should be a symlink");

        // Second entry should remain a regular file.
        assert!(!target2.is_symlink(), "target2 should not be a symlink");
        let content = std::fs::read_to_string(&target2).unwrap();
        assert_eq!(content, "local content");

        // Both output lines should be present.
        assert!(
            output.contains("linked 1 config files"),
            "should have linked line, got: {output:?}"
        );
        assert!(
            output.contains("config files need attention"),
            "should have warning line, got: {output:?}"
        );
    }

    // ── Test 8: Prunes marker when entry no longer in manifest ───────────────

    #[test]
    fn check_prunes_marker_when_entry_no_longer_in_manifest() {
        let dir = tempdir().unwrap();
        let stale_target = dir.path().join("stale_target.txt");
        let stale_target_str = stale_target.to_string_lossy().to_string();

        // Add a self-managed marker for a target that is NOT in any entry slice.
        add_self_managed(
            dir.path(),
            SelfManagedEntry {
                name: "stale".to_string(),
                source: "/some/source".to_string(),
                target: stale_target_str.clone(),
            },
        )
        .unwrap();

        // Entries slice is empty — the marker has no corresponding manifest entry.
        let filtered: Vec<ConfigEntry> = vec![];
        let unfiltered: Vec<ConfigEntry> = vec![];
        let sm = load_self_managed(dir.path()).unwrap();

        let mut buf: Vec<u8> = Vec::new();
        write_check(&mut buf, &filtered, &unfiltered, &sm, dir.path()).expect("write_check failed");
        let output = String::from_utf8(buf).unwrap();

        // Marker should be removed.
        let remaining = load_self_managed(dir.path()).unwrap();
        assert!(remaining.is_empty(), "stale marker should have been pruned");

        // Output should say nothing about pruning.
        assert!(
            !output.contains("prun"),
            "output should not mention pruning, got: {output:?}"
        );
    }

    // ── Test 9: Auto-links stale self-managed entry and removes marker ───────

    #[test]
    fn check_auto_links_stale_self_managed_entry_and_removes_marker() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "hello").unwrap();
        // target NOT created on disk

        // When a self-managed marker exists for a target file that has been deleted,
        // detect_state returns NotLinked because the source still exists, so the
        // auto-link path runs and creates a symlink. The in-loop SM cleanup then
        // removes the now-stale marker.
        add_self_managed(
            dir.path(),
            SelfManagedEntry {
                name: "test".to_string(),
                source: source.to_string_lossy().to_string(),
                target: target.to_string_lossy().to_string(),
            },
        )
        .unwrap();

        let entry = make_entry("test", &source, &target);
        let entries = vec![entry];
        let unfiltered = entries.clone();
        let sm = load_self_managed(dir.path()).unwrap();

        let mut buf: Vec<u8> = Vec::new();
        write_check(&mut buf, &entries, &unfiltered, &sm, dir.path()).expect("write_check failed");

        assert!(
            target.is_symlink(),
            "target should have been auto-linked after write_check"
        );

        // Marker should be removed by the in-loop SM cleanup.
        let remaining = load_self_managed(dir.path()).unwrap();
        assert!(
            remaining.is_empty(),
            "marker should have been pruned when target missing in current manifest"
        );
    }

    // ── Test 10: Preserves marker for other-platform entry ───────────────────

    #[test]
    fn check_preserves_marker_for_other_platform_entry() {
        let dir = tempdir().unwrap();

        // A target that exists in the unfiltered (all-platform) manifest
        // but NOT in the current platform's filtered slice.
        let other_platform_target = dir.path().join("other_platform_target.txt");
        let other_platform_target_str = other_platform_target.to_string_lossy().to_string();

        // Add marker for this cross-platform entry.
        add_self_managed(
            dir.path(),
            SelfManagedEntry {
                name: "macos-only".to_string(),
                source: "/repo/configs/macos-only/config".to_string(),
                target: other_platform_target_str.clone(),
            },
        )
        .unwrap();

        // File does not exist on disk (different platform's path).
        assert!(!other_platform_target.exists());

        // Simulated: current platform (e.g. Linux) sees no entries (filtered is empty).
        // The unfiltered slice contains the other-platform entry via a fake ConfigEntry.
        let source_fake = dir.path().join("fake_source.txt");
        std::fs::write(&source_fake, "fake").unwrap();
        let other_entry = make_entry("macos-only", &source_fake, &other_platform_target);
        let filtered: Vec<ConfigEntry> = vec![]; // not visible on current platform
        let unfiltered: Vec<ConfigEntry> = vec![other_entry]; // exists in all-platform view

        let sm = load_self_managed(dir.path()).unwrap();

        let mut buf: Vec<u8> = Vec::new();
        write_check(&mut buf, &filtered, &unfiltered, &sm, dir.path()).expect("write_check failed");

        // Marker must be preserved — cross-platform safety rule.
        let remaining = load_self_managed(dir.path()).unwrap();
        assert_eq!(
            remaining.len(),
            1,
            "cross-platform marker should be preserved"
        );
        assert_eq!(remaining[0].target, other_platform_target_str);
    }

    // ── Test 11: NotLinked with missing source surfaces as drift, no symlink created

    #[test]
    fn check_does_not_create_dangling_symlink_when_source_missing() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("missing_source.txt");
        let target = dir.path().join("target.txt");

        // source intentionally NOT created — typo or unmigrated entry
        // target intentionally NOT created — would otherwise be NotLinked

        let entry = make_entry("test", &source, &target);
        let output = capture_check(&[entry], &[], dir.path());

        // No symlink should have been created — creating one would result in
        // a dangling managed link.
        assert!(!target.exists(), "target should not exist");
        assert!(
            std::fs::symlink_metadata(&target).is_err(),
            "target should not exist as any kind of filesystem entry"
        );

        // Should be reported as drift with the "missing source" tag.
        assert!(
            output.contains("missing source"),
            "output should report missing source as drift, got: {output:?}"
        );
        assert!(
            output.contains("\u{26a0}"),
            "output should include warning symbol, got: {output:?}"
        );
        assert!(
            !output.contains("linked"),
            "output should not claim to have linked anything, got: {output:?}"
        );
    }

    // ── Test 12: Dangling symlink (Linked + missing source) surfaces as drift ─

    #[test]
    fn check_reports_dangling_symlink_when_source_deleted() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        // Create the symlink first, then remove the source — leaves a
        // symlink whose stored destination still equals `entry.source`.
        std::fs::write(&source, "hello").unwrap();
        symlink(&source, &target).unwrap();
        std::fs::remove_file(&source).unwrap();

        // Sanity: detect_state still classifies this as Linked because
        // read_link only inspects the stored path, not its resolution.
        assert!(target.is_symlink(), "target should still be a symlink");
        assert!(!source.exists(), "source should be gone");

        let entry = make_entry("test", &source, &target);
        let output = capture_check(&[entry], &[], dir.path());

        assert!(
            output.contains("missing source"),
            "output should report missing source as drift, got: {output:?}"
        );
        assert!(
            output.contains("\u{26a0}"),
            "output should include warning symbol, got: {output:?}"
        );
        assert!(
            !output.contains("linked"),
            "output should not claim to have linked anything, got: {output:?}"
        );
    }

    // ── Test 14: Returns Ok even with unresolved drift ───────────────────────

    #[test]
    fn check_returns_ok_even_with_unresolved_drift() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");

        std::fs::write(&source, "repo content").unwrap();
        std::fs::write(&target, "local content").unwrap(); // Conflict

        let entry = make_entry("myconfig", &source, &target);
        let entries = vec![entry];
        let unfiltered = entries.clone();

        let mut buf: Vec<u8> = Vec::new();
        let result = write_check(&mut buf, &entries, &unfiltered, &[], dir.path());

        assert!(
            result.is_ok(),
            "write_check should return Ok even with unresolved drift"
        );
    }
}
