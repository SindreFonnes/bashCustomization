// manifest: config file loading and parsing

use std::path::{Component, Path, PathBuf};

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
    let mut seen_targets: Vec<(PathBuf, Option<String>, String)> = Vec::new();

    for raw_entry in raw.config {
        validate_platform_selector(&raw_entry.platform, &raw_entry.name)?;

        validate_relative_source(&raw_entry.source, &raw_entry.name)?;

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
        validate_absolute_target(&target, &raw_entry.target, &raw_entry.name)?;

        if let Some((_, previous_platform, previous_name)) =
            seen_targets
                .iter()
                .find(|(previous_target, previous_platform, _)| {
                    previous_target == &target
                        && platform_scopes_overlap(previous_platform, &raw_entry.platform)
                })
        {
            anyhow::bail!(
                "Duplicate active target {} for configs '{}' ({}) and '{}' ({}); target paths must be unique within each platform",
                target.display(),
                previous_name,
                platform_scope_label(previous_platform),
                raw_entry.name,
                platform_scope_label(&raw_entry.platform)
            );
        }

        seen_targets.push((
            target.clone(),
            raw_entry.platform.clone(),
            raw_entry.name.clone(),
        ));

        // Platform filtering is deliberately applied only after validation so
        // typos and conflicting entries cannot become invisible on one OS.
        if platform_filter.is_some_and(|p| !platform_matches(&raw_entry.platform, p)) {
            continue;
        }

        entries.push(ConfigEntry {
            name: raw_entry.name,
            source,
            target,
            strategy: raw_entry.strategy.unwrap_or_default(),
        });
    }

    Ok(entries)
}

/// Validate the manifest's small, explicit platform vocabulary.
fn validate_platform_selector(platform: &Option<String>, name: &str) -> Result<()> {
    match platform.as_deref() {
        None | Some("macos" | "linux") => Ok(()),
        Some(other) => anyhow::bail!(
            "Invalid platform selector for config '{}': '{}'; expected 'macos', 'linux', or no platform field",
            name,
            other
        ),
    }
}

/// Return whether two manifest entries can be active on the same platform.
fn platform_scopes_overlap(left: &Option<String>, right: &Option<String>) -> bool {
    left.is_none() || right.is_none() || left == right
}

fn platform_scope_label(platform: &Option<String>) -> &str {
    platform.as_deref().unwrap_or("all platforms")
}

/// Validate that a manifest source remains inside the repo's `configs/` tree.
fn validate_relative_source(source: &str, name: &str) -> Result<()> {
    let path = Path::new(source);

    if source.is_empty() || path.is_absolute() {
        anyhow::bail!(
            "Invalid source for config '{}': source must be relative to configs/: {}",
            name,
            source
        );
    }

    if path
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_)))
    {
        anyhow::bail!(
            "Invalid source for config '{}': source must not escape configs/: {}",
            name,
            source
        );
    }

    Ok(())
}

/// Validate that a manifest target resolves to an absolute filesystem path.
fn validate_absolute_target(target: &Path, raw_target: &str, name: &str) -> Result<()> {
    if !target.is_absolute() {
        anyhow::bail!(
            "Invalid target for config '{}': target must be absolute or start with ~/: {}",
            name,
            raw_target
        );
    }

    if target
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        anyhow::bail!(
            "Invalid target for config '{}': target must not contain '..': {}",
            name,
            raw_target
        );
    }

    Ok(())
}

/// Validate an existing source against the real filesystem, following
/// symlinks. Callers use this immediately before an operation could create or
/// replace a target.
pub(crate) fn validate_source_filesystem_containment(
    source: &Path,
    project_root: &Path,
) -> Result<()> {
    let configs_dir = std::fs::canonicalize(project_root.join("configs")).with_context(|| {
        format!(
            "Failed to resolve configs directory under {}",
            project_root.display()
        )
    })?;
    let resolved_source = std::fs::canonicalize(source)
        .with_context(|| format!("Failed to resolve config source {}", source.display()))?;

    if !resolved_source.starts_with(&configs_dir) {
        anyhow::bail!(
            "Config source {} resolves outside the repository configs directory (resolved to {})",
            source.display(),
            resolved_source.display()
        );
    }

    Ok(())
}

/// Returns true if the raw platform string matches the current `Platform`.
///
/// - `None`/omitted → matches all platforms
/// - `"macos"` → matches only `Os::MacOs`
/// - `"linux"` → matches `Os::Linux(_)` and `Os::Wsl(_)`
///
/// Invalid values are rejected before this helper is called.
fn platform_matches(raw: &Option<String>, platform: &Platform) -> bool {
    match raw.as_deref() {
        None => true,
        Some("macos") => matches!(platform.os, Os::MacOs),
        Some("linux") => matches!(platform.os, Os::Linux(_) | Os::Wsl(_)),
        Some(_) => unreachable!("platform selectors are validated before filtering"),
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
    fn unknown_platform_filter_is_rejected() {
        let toml = r#"
[[config]]
name = "typo"
source = "typo/config"
target = "~/.typo"
platform = "macosX"
"#;
        for platform in [mac_platform(), linux_platform(), wsl_platform()] {
            let err = load_manifest_from_str(toml, &fake_root(), &platform, FAKE_HOME)
                .expect_err("unknown platform selectors must be rejected");
            assert!(
                err.to_string().contains("Invalid platform selector"),
                "unexpected error: {err}"
            );
        }
    }

    #[test]
    fn invalid_platform_is_rejected_even_when_entry_would_be_filtered() {
        let toml = r#"
[[config]]
name = "typo"
source = "typo/config"
target = "~/.typo"
platform = "windows"
"#;
        let err = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
            .expect_err("validation must happen before filtering");
        assert!(err.to_string().contains("expected 'macos', 'linux'"));
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

    #[test]
    fn absolute_source_is_rejected() {
        let toml = r#"
[[config]]
name = "bad"
source = "/etc/passwd"
target = "~/.bad"
"#;
        let err = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
            .expect_err("absolute sources must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("source must be relative"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn parent_dir_source_is_rejected() {
        let toml = r#"
[[config]]
name = "bad"
source = "../outside"
target = "~/.bad"
"#;
        let err = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
            .expect_err("sources must not escape configs/");
        let msg = err.to_string();
        assert!(msg.contains("must not escape"), "unexpected error: {msg}");
    }

    #[test]
    fn relative_target_is_rejected() {
        let toml = r#"
[[config]]
name = "bad"
source = "bad/config"
target = ".config/bad"
"#;
        let err = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
            .expect_err("relative targets must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("target must be absolute"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn target_with_parent_traversal_is_rejected() {
        let toml = r#"
[[config]]
name = "bad"
source = "bad/config"
target = "~/.config/../outside"
"#;
        let err = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
            .expect_err("parent traversal in targets must be rejected");
        assert!(err.to_string().contains("must not contain '..'"));
    }

    #[test]
    fn filesystem_source_symlink_escape_is_rejected() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let configs = root.path().join("configs");
        let outside = root.path().join("outside.txt");
        std::fs::create_dir_all(&configs).unwrap();
        std::fs::write(&outside, "external").unwrap();
        let source = configs.join("escaped.txt");
        symlink(&outside, &source).unwrap();

        let err = validate_source_filesystem_containment(&source, root.path())
            .expect_err("source symlinks must not escape configs");
        assert!(err.to_string().contains("resolves outside"));
    }

    #[test]
    fn duplicate_universal_targets_are_rejected() {
        let toml = r#"
[[config]]
name = "first"
source = "first/config"
target = "~/.shared"

[[config]]
name = "second"
source = "second/config"
target = "~/.shared"
"#;
        let err = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
            .expect_err("duplicate targets must be rejected");
        let message = err.to_string();
        assert!(message.contains("Duplicate active target"));
        assert!(message.contains("first"));
        assert!(message.contains("second"));
    }

    #[test]
    fn universal_and_platform_specific_duplicate_targets_are_rejected() {
        let toml = r#"
[[config]]
name = "universal"
source = "universal/config"
target = "~/.shared"

[[config]]
name = "linux-only"
source = "linux/config"
target = "~/.shared"
platform = "linux"
"#;
        let err = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
            .expect_err("overlapping platform scopes must be rejected before filtering");
        assert!(err.to_string().contains("Duplicate active target"));
    }

    #[test]
    fn same_target_is_allowed_for_disjoint_platforms() {
        let toml = r#"
[[config]]
name = "settings"
source = "settings/macos"
target = "~/.settings"
platform = "macos"

[[config]]
name = "settings"
source = "settings/linux"
target = "~/.settings"
platform = "linux"
"#;
        let mac_entries = load_manifest_from_str(toml, &fake_root(), &mac_platform(), FAKE_HOME)
            .expect("disjoint target scopes should be valid");
        assert_eq!(mac_entries.len(), 1);
        assert!(mac_entries[0].source.ends_with("settings/macos"));

        let all_entries = load_manifest_from_str_unfiltered(toml, &fake_root(), FAKE_HOME)
            .expect("unfiltered loading must preserve both disjoint entries");
        assert_eq!(all_entries.len(), 2);
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
