// diff: unified diff between repo and local config files

use std::io::Write;
use std::path::Path;

use anyhow::{bail, Result};
use similar::TextDiff;

use crate::common::platform::Platform;
use crate::configs::manifest::{filter_by_name, load_manifest};
use crate::configs::state::{detect_state, load_self_managed};
use crate::configs::{display_target, format_source, home_dir, ConfigEntry, EntryState};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Show diffs for configs where the local file differs from the repo version.
///
/// Only meaningful for `Conflict`, `SelfManaged`, and `WrongSymlink` states —
/// in those cases there is a local file (or a symlink pointing to one) that
/// may differ from the repo version. `Linked` entries are symlinks to the
/// repo source (always identical), and `NotLinked` entries have no local
/// file to compare.
pub fn run_diff(project_root: &Path, platform: &Platform, filter_name: Option<&str>) -> Result<()> {
    let home_path = home_dir()?;

    let all_entries = load_manifest(project_root, platform)?;

    let entries: Vec<ConfigEntry> = if let Some(name) = filter_name {
        let filtered = filter_by_name(&all_entries, name);
        if filtered.is_empty() {
            let mut names: Vec<&str> = all_entries.iter().map(|e| e.name.as_str()).collect();
            names.sort();
            names.dedup();
            bail!(
                "No config named '{}'. Available: {}",
                name,
                names.join(", ")
            );
        }
        filtered
    } else {
        all_entries
    };

    let self_managed = load_self_managed(project_root)?;

    write_diff(&mut std::io::stdout(), &entries, &self_managed, &home_path)
}

// ---------------------------------------------------------------------------
// Core logic (accepts a writer for testability)
// ---------------------------------------------------------------------------

fn write_diff(
    writer: &mut impl Write,
    entries: &[ConfigEntry],
    self_managed: &[crate::configs::state::SelfManagedEntry],
    home: &Path,
) -> Result<()> {
    let mut compared_any = false;
    let mut found_any = false;

    for entry in entries {
        let state = detect_state(entry, self_managed);
        let source_display = format_source(entry);
        let target_display = display_target(&entry.target, home);

        match state {
            EntryState::Conflict | EntryState::SelfManaged | EntryState::WrongSymlink => {
                compared_any = true;
                let diff_output = compute_diff(&entry.source, &entry.target)?;
                match diff_output {
                    Some(diff) => {
                        found_any = true;
                        writeln!(writer, "{source_display} \u{2194} {target_display}:")?;
                        write!(writer, "{diff}")?;
                        writeln!(writer)?;
                    }
                    None => {
                        writeln!(
                            writer,
                            "  \u{2713} {source_display} \u{2194} {target_display} (identical)"
                        )?;
                    }
                }
            }
            EntryState::Linked => {
                writeln!(
                    writer,
                    "  \u{2713} {source_display} \u{2192} {target_display} (symlinked)"
                )?;
            }
            EntryState::LinkedMissingSource => {
                writeln!(
                    writer,
                    "  \u{2717} {source_display} \u{2192} {target_display} (source missing, dangling symlink)"
                )?;
            }
            EntryState::NotLinked => {
                writeln!(
                    writer,
                    "  - {source_display} \u{2192} {target_display} (not linked, nothing to compare)"
                )?;
            }
            EntryState::NotLinkedMissingSource => {
                writeln!(
                    writer,
                    "  \u{2717} {source_display} \u{2192} {target_display} (source missing, nothing to compare)"
                )?;
            }
        }
    }

    // Only print the summary when at least one entry was actually compared.
    // Otherwise (e.g. all entries Linked or NotLinked) the per-entry lines
    // already explain why nothing was diffed, and "No differences found."
    // would be misleading in scripted use.
    if compared_any && !found_any {
        writeln!(writer, "No differences found.")?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Diff computation
// ---------------------------------------------------------------------------

/// Compute a unified diff between two files.
/// Returns `None` if the files are identical, `Some(diff_string)` otherwise.
/// Returns an error if either file can't be read or is binary.
pub(crate) fn compute_diff(source: &Path, target: &Path) -> Result<Option<String>> {
    let source_content = read_text_file(source)?;
    let target_content = read_text_file(target)?;

    if source_content == target_content {
        return Ok(None);
    }

    let diff = TextDiff::from_lines(&target_content, &source_content);
    let mut output = String::new();

    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        output.push_str(&format!("{hunk}"));
    }

    Ok(Some(output))
}

/// Read a file as UTF-8 text. Returns a message for binary files.
fn read_text_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))?;

    String::from_utf8(bytes)
        .map_err(|_| anyhow::anyhow!("{} appears to be a binary file", path.display()))
}

/// Print a unified diff between source and target to stderr (for interactive menu use).
pub(crate) fn print_diff_to_stderr(source: &Path, target: &Path) -> Result<()> {
    let diff_output = compute_diff(source, target)?;
    match diff_output {
        Some(diff) => {
            eprintln!("\n--- local ({})", target.display());
            eprintln!("+++ repo ({})", source.display());
            eprint!("{diff}");
            eprintln!();
        }
        None => {
            eprintln!("\nFiles are identical.");
        }
    }
    Ok(())
}

/// Print a file's contents to stderr (for interactive menu use).
pub(crate) fn print_file_to_stderr(path: &Path, label: &str) -> Result<()> {
    const MAX_LINES: usize = 100;

    eprintln!("\n--- {} ({}) ---", label, path.display());

    let bytes = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))?;

    match String::from_utf8(bytes.clone()) {
        Ok(text) => {
            let lines: Vec<&str> = text.lines().collect();
            let total = lines.len();
            let display_lines = if total > MAX_LINES {
                &lines[..MAX_LINES]
            } else {
                &lines
            };
            for line in display_lines {
                eprintln!("{line}");
            }
            if total > MAX_LINES {
                eprintln!("... ({} more lines not shown)", total - MAX_LINES);
            }
        }
        Err(_) => {
            eprintln!("(binary file, {} bytes)", bytes.len());
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
    use crate::configs::state::SelfManagedEntry;
    use crate::configs::Strategy;
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    fn fake_home() -> std::path::PathBuf {
        std::path::PathBuf::from("/home/testuser")
    }

    fn make_entry(
        name: &str,
        source: std::path::PathBuf,
        target: std::path::PathBuf,
    ) -> ConfigEntry {
        ConfigEntry {
            name: name.to_string(),
            source,
            target,
            strategy: Strategy::Prompt,
        }
    }

    #[test]
    fn compute_diff_identical_files() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        std::fs::write(&a, "hello\nworld\n").unwrap();
        std::fs::write(&b, "hello\nworld\n").unwrap();

        let result = compute_diff(&a, &b).unwrap();
        assert!(result.is_none(), "identical files should return None");
    }

    #[test]
    fn compute_diff_different_files() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        std::fs::write(&a, "hello\nworld\n").unwrap();
        std::fs::write(&b, "hello\nrust\n").unwrap();

        let result = compute_diff(&a, &b).unwrap();
        assert!(result.is_some(), "different files should return Some");
        let diff = result.unwrap();
        assert!(diff.contains("-rust"), "diff should show removed line");
        assert!(diff.contains("+world"), "diff should show added line");
    }

    #[test]
    fn write_diff_shows_diff_for_conflict() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");
        std::fs::write(&source, "repo version\n").unwrap();
        std::fs::write(&target, "local version\n").unwrap();

        let entry = make_entry("test", source, target);
        let mut buf: Vec<u8> = Vec::new();
        write_diff(&mut buf, &[entry], &[], &fake_home()).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(
            output.contains("-local version"),
            "should show local as removed: {output}"
        );
        assert!(
            output.contains("+repo version"),
            "should show repo as added: {output}"
        );
    }

    #[test]
    fn write_diff_shows_identical_for_matching_conflict() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");
        std::fs::write(&source, "same content\n").unwrap();
        std::fs::write(&target, "same content\n").unwrap();

        let entry = make_entry("test", source, target);
        let mut buf: Vec<u8> = Vec::new();
        write_diff(&mut buf, &[entry], &[], &fake_home()).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(
            output.contains("identical"),
            "should note files are identical: {output}"
        );
    }

    #[test]
    fn write_diff_shows_symlinked_for_linked() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");
        std::fs::write(&source, "content\n").unwrap();
        symlink(&source, &target).unwrap();

        let entry = make_entry("test", source, target);
        let mut buf: Vec<u8> = Vec::new();
        write_diff(&mut buf, &[entry], &[], &fake_home()).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(
            output.contains("symlinked"),
            "should note it's symlinked: {output}"
        );
    }

    #[test]
    fn write_diff_reports_dangling_managed_symlink() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("missing_source.txt");
        let target = dir.path().join("target.txt");
        symlink(&source, &target).unwrap();

        let entry = make_entry("test", source, target);
        let mut buf: Vec<u8> = Vec::new();
        write_diff(&mut buf, &[entry], &[], &fake_home()).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(
            output.contains("source missing"),
            "should report missing source: {output}"
        );
        assert!(
            output.contains("dangling symlink"),
            "should report dangling symlink: {output}"
        );
        assert!(
            !output.contains("symlinked"),
            "must not report dangling link as healthy: {output}"
        );
    }

    #[test]
    fn write_diff_shows_not_linked() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("nonexistent.txt");
        std::fs::write(&source, "content\n").unwrap();

        let entry = make_entry("test", source, target);
        let mut buf: Vec<u8> = Vec::new();
        write_diff(&mut buf, &[entry], &[], &fake_home()).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(
            output.contains("not linked"),
            "should note not linked: {output}"
        );
    }

    #[test]
    fn write_diff_reports_not_linked_missing_source() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("missing_source.txt");
        let target = dir.path().join("missing_target.txt");

        let entry = make_entry("test", source, target);
        let mut buf: Vec<u8> = Vec::new();
        write_diff(&mut buf, &[entry], &[], &fake_home()).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(
            output.contains("source missing"),
            "should report missing source: {output}"
        );
        assert!(
            !output.contains("not linked, nothing to compare"),
            "must not report missing-source entries as ordinary not-linked entries: {output}"
        );
    }

    #[test]
    fn write_diff_does_not_print_summary_when_nothing_compared() {
        // All entries are NotLinked — nothing to diff. The "No differences found."
        // summary should NOT appear, since no comparison was actually performed.
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("missing_target.txt");
        std::fs::write(&source, "content\n").unwrap();
        // target intentionally not created

        let entry = make_entry("test", source, target);
        let mut buf: Vec<u8> = Vec::new();
        write_diff(&mut buf, &[entry], &[], &fake_home()).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(
            output.contains("not linked"),
            "should explain per-entry why nothing was compared: {output}"
        );
        assert!(
            !output.contains("No differences found."),
            "should NOT print summary when nothing comparable: {output}"
        );
    }

    #[test]
    fn write_diff_prints_summary_when_compared_entries_match() {
        // Entry IS comparable (Conflict) and matches — summary should still
        // appear because at least one comparison was performed.
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");
        std::fs::write(&source, "same\n").unwrap();
        std::fs::write(&target, "same\n").unwrap();

        let entry = make_entry("test", source, target);
        let mut buf: Vec<u8> = Vec::new();
        write_diff(&mut buf, &[entry], &[], &fake_home()).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(output.contains("identical"));
        assert!(
            output.contains("No differences found."),
            "should print summary when at least one entry was compared: {output}"
        );
    }

    #[test]
    fn write_diff_compares_wrong_symlink_against_repo_source() {
        // WrongSymlink should be diffed (its symlink-resolved content vs the
        // repo source) — this matches the documented behavior.
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let other = dir.path().join("other.txt");
        let target = dir.path().join("target.txt");
        std::fs::write(&source, "repo version\n").unwrap();
        std::fs::write(&other, "other version\n").unwrap();
        symlink(&other, &target).unwrap(); // points to wrong place

        let entry = make_entry("test", source, target);
        let mut buf: Vec<u8> = Vec::new();
        write_diff(&mut buf, &[entry], &[], &fake_home()).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(
            output.contains("-other version"),
            "should show wrong-symlink target as removed: {output}"
        );
        assert!(
            output.contains("+repo version"),
            "should show repo as added: {output}"
        );
    }

    #[test]
    fn print_diff_to_stderr_returns_err_for_binary_target() {
        // The interactive `bashc configs link` View-diff option calls
        // print_diff_to_stderr; binary targets fail UTF-8 decoding and bubble
        // an error back. The link command's prompt loop catches this so the
        // whole command isn't aborted — this test pins the error path.
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("binary.bin");
        std::fs::write(&source, "hello\n").unwrap();
        std::fs::write(&target, [0xff, 0xfe, 0x00, 0x01]).unwrap();

        let result = print_diff_to_stderr(&source, &target);
        assert!(result.is_err(), "binary target should fail diff");
    }

    #[test]
    fn print_file_to_stderr_returns_err_for_missing_path() {
        // The View-local / View-repo options call print_file_to_stderr;
        // unreadable paths bubble an error back. The link command's prompt
        // loop catches this so the whole command isn't aborted.
        let dir = tempdir().unwrap();
        let missing = dir.path().join("does_not_exist.txt");

        let result = print_file_to_stderr(&missing, "local");
        assert!(result.is_err(), "missing path should fail");
    }

    #[test]
    fn write_diff_shows_diff_for_self_managed() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let target = dir.path().join("target.txt");
        std::fs::write(&source, "repo\n").unwrap();
        std::fs::write(&target, "local\n").unwrap();

        let sm = vec![SelfManagedEntry {
            name: "test".to_string(),
            source: source.to_string_lossy().to_string(),
            target: target.to_string_lossy().to_string(),
        }];

        let entry = make_entry("test", source, target);
        let mut buf: Vec<u8> = Vec::new();
        write_diff(&mut buf, &[entry], &sm, &fake_home()).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(
            output.contains("-local"),
            "should show diff for self-managed: {output}"
        );
        assert!(
            output.contains("+repo"),
            "should show diff for self-managed: {output}"
        );
    }
}
