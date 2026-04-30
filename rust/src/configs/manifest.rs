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

/// Load the manifest from `<project_root>/configs/manifest.toml` without
/// applying any platform filter. Used by cross-platform safety checks
/// (e.g., self-managed marker cleanup) that must reason about all entries
/// regardless of the current OS.
pub fn load_manifest_unfiltered(project_root: &Path) -> Result<Vec<ConfigEntry>> {
    let manifest_path = project_root.join("configs").join("manifest.toml");
    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read manifest at {}", manifest_path.display()))?;

    let home = crate::configs::home_dir()?;
    load_manifest_from_str_unfiltered(&content, project_root, &home.to_string_lossy())
}

/// Return entries matching the given name (cloned).
pub fn filter_by_name(entries: &[ConfigEntry], name: &str) -> Vec<ConfigEntry> {
    entries.iter().filter(|e| e.name == name).cloned().collect()
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
    parse_manifest_entries(content, project_root, home, Some(platform))
}

/// Parse a manifest from a TOML string without applying any platform filter.
/// `home` is passed explicitly so tests can override it without touching `$HOME`.
fn load_manifest_from_str_unfiltered(
    content: &str,
    project_root: &Path,
    home: &str,
) -> Result<Vec<ConfigEntry>> {
    parse_manifest_entries(content, project_root, home, None)
}

/// Core manifest parser shared by `load_manifest_from_str` and
/// `load_manifest_from_str_unfiltered`.
///
/// When `platform_filter` is `Some(p)`, entries that do not match `p` are
/// skipped. When it is `None`, all entries are returned.
fn parse_manifest_entries(
    content: &str,
    project_root: &Path,
    home: &str,
    platform_filter: Option<&Platform>,
) -> Result<Vec<ConfigEntry>> {
    let raw: RawManifest = toml::from_str(content).context("Failed to parse manifest.toml")?;

    let configs_dir = project_root.join("configs");
    let mut entries = Vec::new();

    for raw_entry in raw.config {
        // Platform filtering — only applied when a filter is supplied
        if platform_filter.is_some_and(|p| !platform_matches(&raw_entry.platform, p)) {
            continue;
        }

        // Resolve source to absolute path.
        //
        // Missing source files are NOT warned about here: `bashc configs
        // check` runs on every interactive shell startup, so a single
        // missing source would spam the terminal on every launch. Validation
        // is owned by the commands that act on the manifest — `link` bails
        // hard, `check`/`status` surface it as drift — which is enough
        // without duplicating the signal at load time.
        let source = configs_dir.join(&raw_entry.source);

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
/// - any other value → non-matching, silently skipped
///
/// Unknown values are dropped without a warning because `load_manifest`
/// runs on every interactive shell startup via `bashc configs check`; a
/// single typo in `manifest.toml` would otherwise spam the terminal on
/// every launch. Invalid entries become invisible to current-platform
/// commands, which is the safe default — manifest typos surface via
/// `bashc configs status` (the entry is missing from the listing) rather
/// than via repeated startup noise.
fn platform_matches(raw: &Option<String>, platform: &Platform) -> bool {
    match raw.as_deref() {
        None => true,
        Some("macos") => matches!(platform.os, Os::MacOs),
        Some("linux") => matches!(platform.os, Os::Linux(_) | Os::Wsl(_)),
        Some(_) => false,
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
        Platform {
            os: Os::MacOs,
            arch: Arch::Aarch64,
        }
    }

    fn linux_platform() -> Platform {
        Platform {
            os: Os::Linux(Distro::Ubuntu),
            arch: Arch::X86_64,
        }
    }

    fn wsl_platform() -> Platform {
        Platform {
            os: Os::Wsl(Distro::Ubuntu),
            arch: Arch::X86_64,
        }
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
        let entries = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
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
        let entries = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
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
        let entries = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
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
            let entries = load_manifest_from_str(toml, &fake_root(), &platform, FAKE_HOME)
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
        let entries = load_manifest_from_str(toml, &fake_root(), &linux_platform(), FAKE_HOME)
            .expect("should parse");
        assert!(
            entries.is_empty(),
            "macos entry should be filtered on Linux"
        );
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
        let entries = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
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
        let entries = load_manifest_from_str(toml, &fake_root(), &linux_platform(), FAKE_HOME)
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
        let entries = load_manifest_from_str(toml, &fake_root(), &wsl_platform(), FAKE_HOME)
            .expect("should parse");
        assert_eq!(
            entries.len(),
            1,
            "linux platform filter should match WSL too"
        );
    }

    #[test]
    fn unknown_platform_filter_is_silently_skipped() {
        // Unknown platform values must not warn (check runs on every shell
        // startup — per-load noise would spam the terminal). The entry is
        // simply filtered out on every real platform.
        let toml = r#"
[[config]]
name = "typo"
source = "typo/config"
target = "~/.typo"
platform = "macosX"
"#;
        for platform in [mac_platform(), linux_platform(), wsl_platform()] {
            let entries = load_manifest_from_str(toml, &fake_root(), &platform, FAKE_HOME)
                .expect("should parse");
            assert!(
                entries.is_empty(),
                "unknown platform filter should be filtered out, got: {entries:?}"
            );
        }
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
        let entries = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
            .expect("should parse");
        assert!(
            entries.is_empty(),
            "linux entry should be filtered on macOS"
        );
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
        let entries = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
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
        let entries = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
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
        let entries = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
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
            load_manifest_from_str(toml, &root, &mac_platform(), FAKE_HOME).expect("should parse");
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
        let all = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
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
        let all = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
            .expect("should parse");
        // filter_by_name should return independent clones
        let filtered = filter_by_name(&all, "claude");
        assert_eq!(filtered.len(), 1);
        // The original slice still has the entry
        assert_eq!(all.len(), 1);
    }

    // -----------------------------------------------------------------------
    // load_manifest_unfiltered
    // -----------------------------------------------------------------------

    #[test]
    fn load_unfiltered_returns_all_entries_regardless_of_platform() {
        let toml = r#"
[[config]]
name = "ghostty"
source = "ghostty/config"
target = "~/Library/Application Support/com.mitchellh.ghostty/config"
platform = "macos"

[[config]]
name = "linux-thing"
source = "linux/config"
target = "~/.config/linux-thing"
platform = "linux"

[[config]]
name = "universal"
source = "universal/config"
target = "~/.config/universal"
"#;
        let entries =
            load_manifest_from_str_unfiltered(toml, &fake_root(), FAKE_HOME).expect("should parse");
        assert_eq!(
            entries.len(),
            3,
            "all three entries should be returned regardless of platform"
        );
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"ghostty"));
        assert!(names.contains(&"linux-thing"));
        assert!(names.contains(&"universal"));
    }

    #[test]
    fn load_unfiltered_still_expands_tilde() {
        let toml = r#"
[[config]]
name = "foo"
source = "foo/config"
target = "~/.foo"
"#;
        let entries =
            load_manifest_from_str_unfiltered(toml, &fake_root(), FAKE_HOME).expect("should parse");
        assert_eq!(
            entries[0].target,
            PathBuf::from("/home/testuser/.foo"),
            "tilde should be expanded to home dir"
        );
    }

    #[test]
    fn load_unfiltered_still_resolves_sources_to_configs_dir() {
        let toml = r#"
[[config]]
name = "claude"
source = "claude/CLAUDE.md"
target = "~/.claude/CLAUDE.md"
"#;
        let root = PathBuf::from("/fake/project");
        let entries =
            load_manifest_from_str_unfiltered(toml, &root, FAKE_HOME).expect("should parse");
        assert_eq!(
            entries[0].source,
            PathBuf::from("/fake/project/configs/claude/CLAUDE.md"),
            "source should be resolved relative to <root>/configs/"
        );
    }
}
