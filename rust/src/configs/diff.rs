// diff: unified diff between repo and local config files

use std::io::Write;
use std::path::Path;

use anyhow::{Result, bail};
use similar::TextDiff;

use crate::common::platform::Platform;
use crate::configs::manifest::{filter_by_name, load_manifest};
use crate::configs::state::{detect_state, load_self_managed};
use crate::configs::{ConfigEntry, EntryState, display_target, format_source, home_dir};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Show diffs for configs where the local file differs from the repo version.
///
/// Only meaningful for `Conflict` and `SelfManaged` states — in those cases
/// there is a local file that may differ from the repo version. `Linked`
/// entries are symlinks (always identical), and `NotLinked` entries have no
/// local file to compare.
pub fn run_diff(
    project_root: &Path,
    platform: &Platform,
    filter_name: Option<&str>,
) -> Result<()> {
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
    let mut found_any = false;

    for entry in entries {
        let state = detect_state(entry, self_managed);
        let source_display = format_source(entry);
        let target_display = display_target(&entry.target, home);

        match state {
            EntryState::Conflict | EntryState::SelfManaged | EntryState::WrongSymlink => {
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
            EntryState::NotLinked => {
                writeln!(
                    writer,
                    "  - {source_display} \u{2192} {target_display} (not linked, nothing to compare)"
                )?;
            }
        }
    }

    if !found_any {
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
    use crate::configs::Strategy;
    use crate::configs::state::SelfManagedEntry;
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    fn fake_home() -> std::path::PathBuf {
        std::path::PathBuf::from("/home/testuser")
    }

    fn make_entry(name: &str, source: std::path::PathBuf, target: std::path::PathBuf) -> ConfigEntry {
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

        assert!(output.contains("-local version"), "should show local as removed: {output}");
        assert!(output.contains("+repo version"), "should show repo as added: {output}");
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

        assert!(output.contains("identical"), "should note files are identical: {output}");
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

        assert!(output.contains("symlinked"), "should note it's symlinked: {output}");
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

        assert!(output.contains("not linked"), "should note not linked: {output}");
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

        assert!(output.contains("-local"), "should show diff for self-managed: {output}");
        assert!(output.contains("+repo"), "should show diff for self-managed: {output}");
    }
}
