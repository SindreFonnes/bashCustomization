// manifest: config file loading and parsing

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::common::platform::{Os, Platform};
use crate::configs::{ConfigEntry, Strategy};

// ---------------------------------------------------------------------------
// Raw TOML deserialization types (private)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct RawManifest {
    config: Vec<RawConfigEntry>,
}

#[derive(Debug, Deserialize)]
struct RawConfigEntry {
    name: String,
    source: String,
    target: String,
    platform: Option<String>,
    strategy: Option<Strategy>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Load and filter the manifest from `<project_root>/configs/manifest.toml`.
///
/// Entries whose `platform` field doesn't match `platform` are excluded.
/// Source paths are resolved to `<project_root>/configs/<source>`.
/// Tilde in target paths is expanded to `$HOME`.
pub fn load_manifest(project_root: &Path, platform: &Platform) -> Result<Vec<ConfigEntry>> {
    let manifest_path = project_root.join("configs").join("manifest.toml");
    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read manifest at {}", manifest_path.display()))?;

    let home = crate::configs::home_dir()?;
    load_manifest_from_str(&content, project_root, platform, &home.to_string_lossy())
}

/// Return entries matching the given name (cloned).
pub fn filter_by_name(entries: &[ConfigEntry], name: &str) -> Vec<ConfigEntry> {
    entries
        .iter()
        .filter(|e| e.name == name)
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Parse and filter a manifest from a TOML string.
/// `home` is passed explicitly so tests can override it without touching `$HOME`.
fn load_manifest_from_str(
    content: &str,
    project_root: &Path,
    platform: &Platform,
    home: &str,
) -> Result<Vec<ConfigEntry>> {
    let raw: RawManifest =
        toml::from_str(content).context("Failed to parse manifest.toml")?;

    let configs_dir = project_root.join("configs");
    let mut entries = Vec::new();

    for raw_entry in raw.config {
        // Platform filtering
        if !platform_matches(&raw_entry.platform, platform) {
            continue;
        }

        // Resolve source to absolute path
        let source = configs_dir.join(&raw_entry.source);

        // Warn if source doesn't exist, but continue
        if !source.exists() {
            eprintln!(
                "Warning: source file does not exist: {}",
                source.display()
            );
        }

        // Expand leading ~ in target
        let target = expand_tilde(&raw_entry.target, home);

        entries.push(ConfigEntry {
            name: raw_entry.name,
            source,
            target,
            strategy: raw_entry.strategy.unwrap_or_default(),
        });
    }

    Ok(entries)
}

/// Returns true if the raw platform string matches the current `Platform`.
///
/// - `None`/omitted → matches all platforms
/// - `"macos"` → matches only `Os::MacOs`
/// - `"linux"` → matches `Os::Linux(_)` and `Os::Wsl(_)`
fn platform_matches(raw: &Option<String>, platform: &Platform) -> bool {
    match raw.as_deref() {
        None => true,
        Some("macos") => matches!(platform.os, Os::MacOs),
        Some("linux") => matches!(platform.os, Os::Linux(_) | Os::Wsl(_)),
        Some(other) => {
            eprintln!("Warning: unknown platform filter '{other}' — skipping entry");
            false
        }
    }
}

/// Replace a leading `~` with the given home directory.
fn expand_tilde(path: &str, home: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        PathBuf::from(format!("{home}/{rest}"))
    } else if path == "~" {
        PathBuf::from(home)
    } else {
        PathBuf::from(path)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::platform::{Arch, Distro, Os, Platform};

    const FAKE_HOME: &str = "/home/testuser";

    fn mac_platform() -> Platform {
        Platform { os: Os::MacOs, arch: Arch::Aarch64 }
    }

    fn linux_platform() -> Platform {
        Platform { os: Os::Linux(Distro::Ubuntu), arch: Arch::X86_64 }
    }

    fn wsl_platform() -> Platform {
        Platform { os: Os::Wsl(Distro::Ubuntu), arch: Arch::X86_64 }
    }

    /// A project root that won't have real files — used for path-resolution tests.
    fn fake_root() -> PathBuf {
        PathBuf::from("/fake/project")
    }

    // -----------------------------------------------------------------------
    // Parsing
    // -----------------------------------------------------------------------

    #[test]
    fn parse_valid_manifest() {
        let toml = r#"
[[config]]
name = "claude"
source = "claude/CLAUDE.md"
target = "~/.claude/CLAUDE.md"

[[config]]
name = "zellij"
source = "zellij/config.kdl"
target = "~/.config/zellij/config.kdl"
"#;
        let entries =
            load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
                .expect("should parse");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "claude");
        assert_eq!(entries[1].name, "zellij");
    }

    #[test]
    fn missing_strategy_defaults_to_prompt() {
        let toml = r#"
[[config]]
name = "claude"
source = "claude/CLAUDE.md"
target = "~/.claude/CLAUDE.md"
"#;
        let entries =
            load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
                .expect("should parse");
        assert_eq!(entries[0].strategy, Strategy::Prompt);
    }

    #[test]
    fn explicit_strategy_is_preserved() {
        let toml = r#"
[[config]]
name = "claude"
source = "claude/CLAUDE.md"
target = "~/.claude/CLAUDE.md"
strategy = "replace"
"#;
        let entries =
            load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
                .expect("should parse");
        assert_eq!(entries[0].strategy, Strategy::Replace);
    }

    // -----------------------------------------------------------------------
    // Platform filtering
    // -----------------------------------------------------------------------

    #[test]
    fn no_platform_field_matches_all_platforms() {
        let toml = r#"
[[config]]
name = "any"
source = "any/config"
target = "~/.any"
"#;
        for platform in [mac_platform(), linux_platform(), wsl_platform()] {
            let entries =
                load_manifest_from_str(toml, &fake_root(), &platform, FAKE_HOME)
                    .expect("should parse");
            assert_eq!(entries.len(), 1, "should include entry for every platform");
        }
    }

    #[test]
    fn macos_entry_excluded_on_linux() {
        let toml = r#"
[[config]]
name = "ghostty"
source = "ghostty/config"
target = "~/Library/Application Support/com.mitchellh.ghostty/config"
platform = "macos"
"#;
        let entries =
            load_manifest_from_str(toml, &fake_root(), &linux_platform(), FAKE_HOME)
                .expect("should parse");
        assert!(entries.is_empty(), "macos entry should be filtered on Linux");
    }

    #[test]
    fn macos_entry_included_on_macos() {
        let toml = r#"
[[config]]
name = "ghostty"
source = "ghostty/config"
target = "~/Library/Application Support/com.mitchellh.ghostty/config"
platform = "macos"
"#;
        let entries =
            load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
                .expect("should parse");
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn linux_entry_matches_native_linux() {
        let toml = r#"
[[config]]
name = "linux-thing"
source = "linux/config"
target = "~/.config/linux-thing"
platform = "linux"
"#;
        let entries =
            load_manifest_from_str(toml, &fake_root(), &linux_platform(), FAKE_HOME)
                .expect("should parse");
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn linux_entry_matches_wsl() {
        let toml = r#"
[[config]]
name = "linux-thing"
source = "linux/config"
target = "~/.config/linux-thing"
platform = "linux"
"#;
        let entries =
            load_manifest_from_str(toml, &fake_root(), &wsl_platform(), FAKE_HOME)
                .expect("should parse");
        assert_eq!(
            entries.len(),
            1,
            "linux platform filter should match WSL too"
        );
    }

    #[test]
    fn linux_entry_excluded_on_macos() {
        let toml = r#"
[[config]]
name = "linux-thing"
source = "linux/config"
target = "~/.config/linux-thing"
platform = "linux"
"#;
        let entries =
            load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
                .expect("should parse");
        assert!(entries.is_empty(), "linux entry should be filtered on macOS");
    }

    // -----------------------------------------------------------------------
    // Path handling
    // -----------------------------------------------------------------------

    #[test]
    fn tilde_is_expanded_in_target() {
        let toml = r#"
[[config]]
name = "claude"
source = "claude/CLAUDE.md"
target = "~/.claude/CLAUDE.md"
"#;
        let entries =
            load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
                .expect("should parse");
        assert_eq!(
            entries[0].target,
            PathBuf::from("/home/testuser/.claude/CLAUDE.md")
        );
    }

    #[test]
    fn bare_tilde_expands_to_home() {
        let toml = r#"
[[config]]
name = "home"
source = "home/something"
target = "~"
"#;
        let entries =
            load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
                .expect("should parse");
        assert_eq!(entries[0].target, PathBuf::from(FAKE_HOME));
    }

    #[test]
    fn absolute_target_is_unchanged() {
        let toml = r#"
[[config]]
name = "absolute"
source = "some/config"
target = "/etc/myconfig"
"#;
        let entries =
            load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
                .expect("should parse");
        assert_eq!(entries[0].target, PathBuf::from("/etc/myconfig"));
    }

    #[test]
    fn source_resolved_relative_to_configs_dir() {
        let toml = r#"
[[config]]
name = "claude"
source = "claude/CLAUDE.md"
target = "~/.claude/CLAUDE.md"
"#;
        let root = PathBuf::from("/my/project");
        let entries =
            load_manifest_from_str(toml, &root, &mac_platform(), FAKE_HOME)
                .expect("should parse");
        assert_eq!(
            entries[0].source,
            PathBuf::from("/my/project/configs/claude/CLAUDE.md")
        );
    }

    // -----------------------------------------------------------------------
    // filter_by_name
    // -----------------------------------------------------------------------

    #[test]
    fn filter_by_name_returns_matching_entries() {
        let toml = r#"
[[config]]
name = "claude"
source = "claude/CLAUDE.md"
target = "~/.claude/CLAUDE.md"

[[config]]
name = "claude"
source = "claude/settings.json"
target = "~/.claude/settings.json"

[[config]]
name = "zellij"
source = "zellij/config.kdl"
target = "~/.config/zellij/config.kdl"
"#;
        let all =
            load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
                .expect("should parse");

        let claude = filter_by_name(&all, "claude");
        assert_eq!(claude.len(), 2);
        assert!(claude.iter().all(|e| e.name == "claude"));

        let zellij = filter_by_name(&all, "zellij");
        assert_eq!(zellij.len(), 1);

        let none = filter_by_name(&all, "nonexistent");
        assert!(none.is_empty());
    }

    #[test]
    fn filter_by_name_clones_entries() {
        let toml = r#"
[[config]]
name = "claude"
source = "claude/CLAUDE.md"
target = "~/.claude/CLAUDE.md"
"#;
        let all =
            load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
                .expect("should parse");
        // filter_by_name should return independent clones
        let filtered = filter_by_name(&all, "claude");
        assert_eq!(filtered.len(), 1);
        // The original slice still has the entry
        assert_eq!(all.len(), 1);
    }
}
