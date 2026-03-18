# `bashc` Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `bashc` Rust binary with all install subcommands, CI/CD for precompiled releases, and a bootstrap init script.

**Architecture:** Single Rust crate with clap subcommands. Each tool installer implements an `Installer` trait. `bashc install all` runs installers in parallel where possible, collects errors, and reports a summary. Sudo requirements are checked upfront before any work begins. Distributed via GitHub Releases; a small POSIX init.sh bootstraps fresh machines.

**Tech Stack:** Rust, clap, reqwest, sha2, semver, serde/serde_json, tokio, indicatif, dialoguer

**Spec:** `docs/specs/2026-03-18-bashc-binary-design.md`

---

## File structure

```
rust/
  Cargo.toml
  src/
    main.rs                    # clap CLI, tokio runtime entry
    install/
      mod.rs                   # Installer trait, registry, parallel orchestrator, sudo pre-flight
      go.rs
      kubectl.rs
      rust_lang.rs
      docker.rs
      azure.rs
      dotnet.rs
      neovim.rs
      obsidian.rs
      brew.rs
      java.rs
      github_cli.rs
      terraform.rs
      postgres.rs
      javascript.rs            # nvm, pnpm, bun, yarn
    common/
      mod.rs
      platform.rs              # Platform struct, OS/arch detection
      version.rs               # semver comparison
      download.rs              # HTTP download + SHA256 verification
      package_manager.rs       # brew/apt helpers
      command.rs               # run subprocess, capture output
.github/
  workflows/
    release.yml                # cross-compile + GitHub Release
init.sh                        # POSIX bootstrap for fresh machines
```

---

## Chunk 1: Scaffold and shared libraries

### Task 1: Initialize the Rust crate

**Files:**
- Create: `rust/Cargo.toml`
- Create: `rust/src/main.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "bashc"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.12", features = ["blocking", "json"] }
sha2 = "0.10"
semver = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
indicatif = "0.17"
dialoguer = "0.11"
anyhow = "1"
```

- [ ] **Step 2: Create minimal main.rs**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bashc", about = "Shell customization toolkit")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install development tools
    Install {
        /// Tool to install, or "all" for everything
        tool: String,
        /// Show interactive selection menu
        #[arg(long)]
        interactive: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Install { tool, interactive } => {
            if interactive {
                println!("Interactive mode not yet implemented");
            } else {
                println!("Would install: {tool}");
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 3: Verify it compiles and runs**

Run: `cd rust && cargo build`
Expected: Compiles without errors

Run: `cargo run -- install go`
Expected: Prints "Would install: go"

- [ ] **Step 4: Commit**

```bash
git add rust/Cargo.toml rust/src/main.rs
git commit -m "feat: scaffold bashc Rust crate with clap CLI"
```

---

### Task 2: Platform detection module

**Files:**
- Create: `rust/src/common/mod.rs`
- Create: `rust/src/common/platform.rs`
- Modify: `rust/src/main.rs`

- [ ] **Step 1: Create common/mod.rs**

```rust
pub mod platform;
```

- [ ] **Step 2: Create platform.rs with types and detection**

```rust
use anyhow::{bail, Result};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Os {
    MacOs,
    Linux,
    Wsl,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Arch {
    X86_64,
    Aarch64,
}

#[derive(Debug, Clone, Copy)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}/{:?}", self.os, self.arch)
    }
}

impl Platform {
    pub fn detect() -> Result<Self> {
        let os = detect_os()?;
        let arch = detect_arch()?;
        Ok(Self { os, arch })
    }

    pub fn is_mac(&self) -> bool {
        self.os == Os::MacOs
    }

    pub fn is_linux(&self) -> bool {
        matches!(self.os, Os::Linux | Os::Wsl)
    }

    pub fn is_wsl(&self) -> bool {
        self.os == Os::Wsl
    }

    /// Returns the Go-style OS string (e.g., "darwin", "linux")
    pub fn go_os(&self) -> &str {
        match self.os {
            Os::MacOs => "darwin",
            Os::Linux | Os::Wsl => "linux",
        }
    }

    /// Returns the Go-style arch string (e.g., "amd64", "arm64")
    pub fn go_arch(&self) -> &str {
        match self.arch {
            Arch::X86_64 => "amd64",
            Arch::Aarch64 => "arm64",
        }
    }
}

fn detect_os() -> Result<Os> {
    if cfg!(target_os = "macos") {
        return Ok(Os::MacOs);
    }

    if cfg!(target_os = "linux") {
        // Check for WSL
        if let Ok(version) = std::fs::read_to_string("/proc/version") {
            if version.to_lowercase().contains("wsl") {
                return Ok(Os::Wsl);
            }
        }
        return Ok(Os::Linux);
    }

    bail!(
        "Unsupported operating system. Supported: macOS, Linux, WSL. \
         Detected: {}",
        std::env::consts::OS
    )
}

fn detect_arch() -> Result<Arch> {
    match std::env::consts::ARCH {
        "x86_64" => Ok(Arch::X86_64),
        "aarch64" => Ok(Arch::Aarch64),
        other => bail!(
            "Unsupported architecture: {other}. Supported: x86_64, aarch64"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_valid_platform() {
        let platform = Platform::detect().unwrap();
        // We must be on a supported platform to run tests
        assert!(matches!(platform.os, Os::MacOs | Os::Linux | Os::Wsl));
        assert!(matches!(platform.arch, Arch::X86_64 | Arch::Aarch64));
    }

    #[test]
    fn go_strings_are_correct() {
        let mac_arm = Platform { os: Os::MacOs, arch: Arch::Aarch64 };
        assert_eq!(mac_arm.go_os(), "darwin");
        assert_eq!(mac_arm.go_arch(), "arm64");

        let linux_x86 = Platform { os: Os::Linux, arch: Arch::X86_64 };
        assert_eq!(linux_x86.go_os(), "linux");
        assert_eq!(linux_x86.go_arch(), "amd64");
    }
}
```

- [ ] **Step 3: Wire into main.rs**

Add `mod common;` to the top of `main.rs` and print detected platform:

```rust
mod common;

// In main(), before the match:
let platform = common::platform::Platform::detect()?;
println!("Detected platform: {platform}");
```

- [ ] **Step 4: Run tests**

Run: `cd rust && cargo test`
Expected: All tests pass

Run: `cargo run -- install go`
Expected: Prints "Detected platform: MacOs/Aarch64" (or your actual platform)

- [ ] **Step 5: Commit**

```bash
git add rust/src/common/
git commit -m "feat: add platform detection with OS and arch"
```

---

### Task 3: Command execution module

**Files:**
- Create: `rust/src/common/command.rs`
- Modify: `rust/src/common/mod.rs`

- [ ] **Step 1: Create command.rs**

```rust
use anyhow::{bail, Context, Result};
use std::process::Command;

/// Run a command and return stdout. Fails if exit code is non-zero.
pub fn run(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute: {program}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "{program} {} failed (exit {}): {}",
            args.join(" "),
            output.status.code().unwrap_or(-1),
            stderr.trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run a command inheriting stdin/stdout/stderr (visible to user).
pub fn run_visible(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("Failed to execute: {program}"))?;

    if !status.success() {
        bail!(
            "{program} {} failed (exit {})",
            args.join(" "),
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}

/// Check if a command exists on PATH.
pub fn exists(program: &str) -> bool {
    Command::new("which")
        .arg(program)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run a command with sudo. Inherits stdin/stdout/stderr.
pub fn run_sudo(program: &str, args: &[&str]) -> Result<()> {
    let mut sudo_args = vec![program];
    sudo_args.extend_from_slice(args);
    run_visible("sudo", &sudo_args)
}

/// Check if the current process is running as root.
pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}
```

- [ ] **Step 2: Add libc dependency to Cargo.toml**

Add to `[dependencies]`:
```toml
libc = "0.2"
```

- [ ] **Step 3: Update common/mod.rs**

```rust
pub mod command;
pub mod platform;
```

- [ ] **Step 4: Run tests**

Run: `cd rust && cargo build`
Expected: Compiles

- [ ] **Step 5: Commit**

```bash
git add rust/src/common/command.rs rust/src/common/mod.rs rust/Cargo.toml
git commit -m "feat: add command execution helpers (run, sudo, exists)"
```

---

### Task 4: Download module

**Files:**
- Create: `rust/src/common/download.rs`
- Modify: `rust/src/common/mod.rs`

- [ ] **Step 1: Create download.rs**

```rust
use anyhow::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Download a URL to a file, showing a progress bar.
pub fn download_file(url: &str, dest: &Path) -> Result<()> {
    let response = reqwest::blocking::Client::new()
        .get(url)
        .send()
        .with_context(|| format!("Failed to download: {url}"))?;

    if !response.status().is_success() {
        bail!("Download failed: HTTP {}", response.status());
    }

    let total_size = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("=> "),
    );
    pb.set_message(format!(
        "Downloading {}",
        dest.file_name().unwrap_or_default().to_string_lossy()
    ));

    let bytes = response.bytes()?;
    let mut file = fs::File::create(dest)
        .with_context(|| format!("Failed to create: {}", dest.display()))?;
    file.write_all(&bytes)?;

    pb.finish_with_message("Download complete");
    Ok(())
}

/// Fetch a URL and return the body as a string.
pub fn fetch_text(url: &str) -> Result<String> {
    let body = reqwest::blocking::get(url)
        .with_context(|| format!("Failed to fetch: {url}"))?
        .text()?;
    Ok(body)
}

/// Fetch a URL and parse the JSON response.
pub fn fetch_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T> {
    let body = reqwest::blocking::get(url)
        .with_context(|| format!("Failed to fetch: {url}"))?
        .json::<T>()?;
    Ok(body)
}

/// Compute SHA256 of a file and compare to expected hash.
pub fn verify_sha256(file: &Path, expected: &str) -> Result<()> {
    let bytes = fs::read(file)
        .with_context(|| format!("Failed to read: {}", file.display()))?;
    let hash = format!("{:x}", Sha256::digest(&bytes));

    if hash != expected.to_lowercase() {
        bail!(
            "Checksum mismatch for {}:\n  expected: {}\n  got:      {}",
            file.display(),
            expected.to_lowercase(),
            hash
        );
    }
    Ok(())
}
```

- [ ] **Step 2: Update common/mod.rs**

```rust
pub mod command;
pub mod download;
pub mod platform;
```

- [ ] **Step 3: Add a unit test for verify_sha256**

Add to the bottom of `download.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn sha256_matches_known_value() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        let mut f = fs::File::create(&file).unwrap();
        f.write_all(b"hello world").unwrap();
        // Known SHA256 of "hello world"
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        verify_sha256(&file, expected).unwrap();
    }

    #[test]
    fn sha256_rejects_wrong_hash() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        let mut f = fs::File::create(&file).unwrap();
        f.write_all(b"hello world").unwrap();
        assert!(verify_sha256(&file, "0000").is_err());
    }
}
```

- [ ] **Step 4: Add tempfile dev-dependency**

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 5: Run tests**

Run: `cd rust && cargo test`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add rust/src/common/download.rs rust/src/common/mod.rs rust/Cargo.toml
git commit -m "feat: add download module with progress bar and SHA256 verification"
```

---

### Task 5: Package manager module

**Files:**
- Create: `rust/src/common/package_manager.rs`
- Modify: `rust/src/common/mod.rs`

- [ ] **Step 1: Create package_manager.rs**

```rust
use super::command;
use super::platform::{Os, Platform};
use anyhow::{bail, Result};

/// Install a package via the platform's package manager.
/// On macOS: brew install <pkg>
/// On Linux: sudo apt install -y <pkg>
pub fn install(platform: &Platform, package: &str) -> Result<()> {
    match platform.os {
        Os::MacOs => {
            println!("  brew install {package}");
            command::run_visible("brew", &["install", package])
        }
        Os::Linux | Os::Wsl => {
            println!("  apt install {package}");
            command::run_sudo("apt-get", &["install", "-y", package])
        }
    }
}

/// Install a brew cask (macOS only).
pub fn brew_install_cask(package: &str) -> Result<()> {
    println!("  brew install --cask {package}");
    command::run_visible("brew", &["install", "--cask", package])
}

/// Add a GPG key from a URL to apt's trusted keys.
pub fn apt_add_gpg_key(url: &str, keyring_path: &str) -> Result<()> {
    let key_data = command::run(
        "curl",
        &["-fsSL", url],
    )?;

    // Pipe through gpg --dearmor and write to keyring
    let gpg_output = std::process::Command::new("gpg")
        .args(["--dearmor"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    // Use sudo tee to write to the keyring path
    let mut child = std::process::Command::new("sudo")
        .args(["tee", keyring_path])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()?;

    // Actually, simpler approach: download then sudo mv
    let temp_dir = std::env::temp_dir();
    let temp_key = temp_dir.join("bashc-gpg-key.gpg");

    // Download key, dearmor, write to temp
    command::run(
        "sh",
        &[
            "-c",
            &format!(
                "curl -fsSL '{url}' | gpg --dearmor -o '{}'",
                temp_key.display()
            ),
        ],
    )?;

    // Move to final location with sudo
    command::run_sudo(
        "mv",
        &[
            &temp_key.to_string_lossy(),
            keyring_path,
        ],
    )?;

    // Drop unused handles
    drop(gpg_output);
    drop(child);

    Ok(())
}

/// Add an apt repository source.
pub fn apt_add_repo(repo_line: &str, list_file: &str) -> Result<()> {
    command::run(
        "sh",
        &[
            "-c",
            &format!("echo '{}' | sudo tee {}", repo_line, list_file),
        ],
    )?;
    command::run_sudo("apt-get", &["update"])?;
    Ok(())
}

/// Ensure Homebrew is installed (macOS only).
pub fn ensure_brew() -> Result<()> {
    if command::exists("brew") {
        return Ok(());
    }

    println!("Homebrew not found. Installing...");
    command::run_visible(
        "/bin/bash",
        &[
            "-c",
            "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)",
        ],
    )?;

    // Evaluate shellenv so brew is available in this session
    if std::path::Path::new("/opt/homebrew/bin/brew").exists() {
        command::run_visible("sh", &["-c", "eval $(/opt/homebrew/bin/brew shellenv)"])?;
    }

    if !command::exists("brew") {
        bail!("Failed to install Homebrew");
    }

    Ok(())
}

/// Check if a command needs sudo on the current platform.
pub fn needs_sudo_for_apt(platform: &Platform) -> bool {
    platform.is_linux() && !command::is_root()
}
```

- [ ] **Step 2: Update common/mod.rs**

```rust
pub mod command;
pub mod download;
pub mod package_manager;
pub mod platform;
```

- [ ] **Step 3: Build to verify**

Run: `cd rust && cargo build`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git add rust/src/common/package_manager.rs rust/src/common/mod.rs
git commit -m "feat: add package manager helpers (brew, apt, GPG keys)"
```

---

### Task 6: Version comparison module

**Files:**
- Create: `rust/src/common/version.rs`
- Modify: `rust/src/common/mod.rs`

- [ ] **Step 1: Create version.rs**

```rust
use anyhow::Result;
use semver::Version;

/// Parse a version string, stripping any leading "v" or "go" prefix.
pub fn parse(version_str: &str) -> Result<Version> {
    let cleaned = version_str
        .trim()
        .trim_start_matches('v')
        .trim_start_matches("go");
    Ok(Version::parse(cleaned)?)
}

/// Returns true if `new_version` is greater than `current_version`.
pub fn is_newer(current: &str, new: &str) -> Result<bool> {
    let current = parse(current)?;
    let new = parse(new)?;
    Ok(new > current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_strips_v_prefix() {
        let v = parse("v1.22.3").unwrap();
        assert_eq!(v, Version::new(1, 22, 3));
    }

    #[test]
    fn parse_strips_go_prefix() {
        let v = parse("go1.22.3").unwrap();
        assert_eq!(v, Version::new(1, 22, 3));
    }

    #[test]
    fn is_newer_works() {
        assert!(is_newer("1.21.0", "1.22.0").unwrap());
        assert!(!is_newer("1.22.0", "1.21.0").unwrap());
        assert!(!is_newer("1.22.0", "1.22.0").unwrap());
    }

    #[test]
    fn is_newer_with_prefixes() {
        assert!(is_newer("v1.21.0", "v1.22.0").unwrap());
        assert!(is_newer("go1.21.0", "go1.22.0").unwrap());
    }
}
```

- [ ] **Step 2: Update common/mod.rs**

```rust
pub mod command;
pub mod download;
pub mod package_manager;
pub mod platform;
pub mod version;
```

- [ ] **Step 3: Run tests**

Run: `cd rust && cargo test`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add rust/src/common/version.rs rust/src/common/mod.rs
git commit -m "feat: add semver comparison with v/go prefix stripping"
```

---

## Chunk 2: Installer trait and first tools

### Task 7: Installer trait and orchestrator

**Files:**
- Create: `rust/src/install/mod.rs`
- Modify: `rust/src/main.rs`

- [ ] **Step 1: Create install/mod.rs with the trait and registry**

```rust
pub mod go;

use crate::common::platform::Platform;
use anyhow::Result;
use std::fmt;

pub trait Installer: Send + Sync {
    fn name(&self) -> &str;
    fn needs_sudo(&self, platform: &Platform) -> bool;
    fn is_installed(&self) -> bool;
    fn install(&self, platform: &Platform) -> Result<()>;
}

#[derive(Debug)]
pub struct InstallResult {
    pub name: String,
    pub outcome: Outcome,
}

#[derive(Debug)]
pub enum Outcome {
    Installed,
    Skipped,
    Failed(String),
}

impl fmt::Display for InstallResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.outcome {
            Outcome::Installed => write!(f, "  {} — installed", self.name),
            Outcome::Skipped => write!(f, "  {} — already installed, skipped", self.name),
            Outcome::Failed(e) => write!(f, "  {} — {}", self.name, e),
        }
    }
}

/// Returns all registered installers.
pub fn all_installers() -> Vec<Box<dyn Installer>> {
    vec![
        Box::new(go::Go),
        // More will be added here as tools are ported
    ]
}

/// Find a single installer by name.
pub fn find_installer(name: &str) -> Option<Box<dyn Installer>> {
    all_installers().into_iter().find(|i| i.name() == name)
}

/// Run a single installer with pre-flight checks.
pub fn run_one(installer: &dyn Installer, platform: &Platform) -> InstallResult {
    let name = installer.name().to_string();

    if installer.is_installed() {
        return InstallResult { name, outcome: Outcome::Skipped };
    }

    if installer.needs_sudo(platform) && !crate::common::command::is_root() {
        return InstallResult {
            name,
            outcome: Outcome::Failed(format!(
                "requires sudo, re-run with: sudo bashc install {}",
                installer.name()
            )),
        };
    }

    match installer.install(platform) {
        Ok(()) => InstallResult { name, outcome: Outcome::Installed },
        Err(e) => InstallResult { name, outcome: Outcome::Failed(format!("{e:#}")) },
    }
}

/// Run all installers. Checks sudo upfront, runs in parallel where possible.
pub fn run_all(platform: &Platform) -> Vec<InstallResult> {
    let installers = all_installers();

    // Pre-flight: check if any need sudo and we're not root
    let needs_sudo: Vec<&str> = installers
        .iter()
        .filter(|i| i.needs_sudo(platform) && !crate::common::command::is_root())
        .map(|i| i.name())
        .collect();

    if !needs_sudo.is_empty() {
        eprintln!(
            "Error: The following tools require sudo on this platform: {}",
            needs_sudo.join(", ")
        );
        eprintln!("Re-run with: sudo bashc install all");
        return needs_sudo
            .into_iter()
            .map(|name| InstallResult {
                name: name.to_string(),
                outcome: Outcome::Failed("requires sudo".to_string()),
            })
            .collect();
    }

    // Run all installers (sequential for now — parallelism added in a later task)
    installers
        .iter()
        .map(|i| run_one(i.as_ref(), platform))
        .collect()
}

/// Print a summary of install results.
pub fn print_summary(results: &[InstallResult]) {
    let installed: Vec<_> = results.iter().filter(|r| matches!(r.outcome, Outcome::Installed)).collect();
    let skipped: Vec<_> = results.iter().filter(|r| matches!(r.outcome, Outcome::Skipped)).collect();
    let failed: Vec<_> = results.iter().filter(|r| matches!(r.outcome, Outcome::Failed(_))).collect();

    let total = results.len();
    let ok = installed.len() + skipped.len();
    println!("\n{ok}/{total} tools OK.\n");

    if !installed.is_empty() {
        println!("Installed:");
        for r in &installed { println!("{r}"); }
        println!();
    }
    if !skipped.is_empty() {
        println!("Skipped (already installed):");
        for r in &skipped { println!("{r}"); }
        println!();
    }
    if !failed.is_empty() {
        println!("Failed:");
        for r in &failed { println!("{r}"); }
        println!("\nFailed tools can be retried individually: bashc install <tool>");
    }
}
```

- [ ] **Step 2: Update main.rs to use the install module**

```rust
mod common;
mod install;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bashc", about = "Shell customization toolkit")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install development tools
    Install {
        /// Tool to install, or "all" for everything
        tool: String,
        /// Show interactive selection menu
        #[arg(long)]
        interactive: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let platform = common::platform::Platform::detect()?;

    match cli.command {
        Commands::Install { tool, interactive } => {
            if interactive {
                println!("Interactive mode not yet implemented");
                return Ok(());
            }

            if tool == "all" {
                let results = install::run_all(&platform);
                install::print_summary(&results);
            } else if let Some(installer) = install::find_installer(&tool) {
                let result = install::run_one(installer.as_ref(), &platform);
                println!("{result}");
            } else {
                let names: Vec<_> = install::all_installers()
                    .iter()
                    .map(|i| i.name().to_string())
                    .collect();
                eprintln!("Unknown tool: {tool}");
                eprintln!("Available: {}", names.join(", "));
                std::process::exit(1);
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 3: Build to verify**

Run: `cd rust && cargo build`
Expected: Compiles (go module will be a stub in next task)

- [ ] **Step 4: Commit**

```bash
git add rust/src/install/mod.rs rust/src/main.rs
git commit -m "feat: add Installer trait, orchestrator, and summary reporting"
```

---

### Task 8: Go installer

**Files:**
- Create: `rust/src/install/go.rs`

- [ ] **Step 1: Create go.rs**

```rust
use crate::common::{command, download, package_manager, platform::Platform};
use anyhow::{Context, Result};
use serde::Deserialize;

pub struct Go;

#[derive(Deserialize)]
struct GoRelease {
    version: String,
    files: Vec<GoFile>,
}

#[derive(Deserialize)]
struct GoFile {
    filename: String,
    os: String,
    arch: String,
    sha256: String,
    kind: String,
}

impl super::Installer for Go {
    fn name(&self) -> &str { "go" }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        // Linux needs sudo to write to /usr/local
        platform.is_linux()
    }

    fn is_installed(&self) -> bool {
        command::exists("go")
    }

    fn install(&self, platform: &Platform) -> Result<()> {
        if platform.is_mac() {
            package_manager::ensure_brew()?;
            return package_manager::install(platform, "go");
        }

        install_go_linux(platform)
    }
}

fn install_go_linux(platform: &Platform) -> Result<()> {
    println!("Fetching latest Go version...");

    let releases: Vec<GoRelease> =
        download::fetch_json("https://go.dev/dl/?mode=json")?;

    let release = releases
        .first()
        .context("No Go releases found")?;

    let file = release
        .files
        .iter()
        .find(|f| {
            f.os == platform.go_os()
                && f.arch == platform.go_arch()
                && f.kind == "archive"
        })
        .with_context(|| {
            format!(
                "No Go archive found for {}/{}",
                platform.go_os(),
                platform.go_arch()
            )
        })?;

    println!("Installing {} ...", release.version);

    let temp_dir = std::env::temp_dir();
    let archive_path = temp_dir.join(&file.filename);
    let url = format!("https://go.dev/dl/{}", file.filename);

    download::download_file(&url, &archive_path)?;

    println!("Verifying checksum...");
    download::verify_sha256(&archive_path, &file.sha256)?;

    // Remove old installation
    if std::path::Path::new("/usr/local/go").exists() {
        command::run_sudo("rm", &["-rf", "/usr/local/go"])?;
    }

    // Extract
    command::run_sudo(
        "tar",
        &["-C", "/usr/local", "-xzf", &archive_path.to_string_lossy()],
    )?;

    // Clean up
    std::fs::remove_file(&archive_path).ok();

    println!("Go {} installed to /usr/local/go", release.version);
    println!("Ensure /usr/local/go/bin is in your PATH");
    Ok(())
}
```

- [ ] **Step 2: Verify build and run**

Run: `cd rust && cargo build`
Expected: Compiles

Run: `cargo run -- install go`
Expected: Either installs Go or says "already installed, skipped"

- [ ] **Step 3: Commit**

```bash
git add rust/src/install/go.rs
git commit -m "feat: add Go installer with version API and checksum verification"
```

---

### Task 9: kubectl installer

**Files:**
- Create: `rust/src/install/kubectl.rs`
- Modify: `rust/src/install/mod.rs` (add to registry)

- [ ] **Step 1: Create kubectl.rs**

```rust
use crate::common::{command, download, package_manager, platform::Platform};
use anyhow::Result;

pub struct Kubectl;

impl super::Installer for Kubectl {
    fn name(&self) -> &str { "kubectl" }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_linux()
    }

    fn is_installed(&self) -> bool {
        command::exists("kubectl")
    }

    fn install(&self, platform: &Platform) -> Result<()> {
        if platform.is_mac() {
            package_manager::ensure_brew()?;
            package_manager::install(platform, "kubernetes-cli")?;
            package_manager::install(platform, "kubectx")?;
            return Ok(());
        }

        install_kubectl_linux(platform)
    }
}

fn install_kubectl_linux(platform: &Platform) -> Result<()> {
    println!("Fetching latest kubectl version...");
    let version = download::fetch_text(
        "https://dl.k8s.io/release/stable.txt",
    )?;
    let version = version.trim();

    let arch = platform.go_arch();
    let url = format!(
        "https://dl.k8s.io/release/{version}/bin/linux/{arch}/kubectl"
    );
    let checksum_url = format!("{url}.sha256");

    let temp_dir = std::env::temp_dir();
    let binary_path = temp_dir.join("kubectl");

    download::download_file(&url, &binary_path)?;

    println!("Verifying checksum...");
    let expected_hash = download::fetch_text(&checksum_url)?;
    download::verify_sha256(&binary_path, expected_hash.trim())?;

    command::run_sudo("install", &[
        "-o", "root", "-g", "root", "-m", "0755",
        &binary_path.to_string_lossy(),
        "/usr/local/bin/kubectl",
    ])?;

    std::fs::remove_file(&binary_path).ok();

    println!("kubectl {version} installed to /usr/local/bin/kubectl");
    Ok(())
}
```

- [ ] **Step 2: Register in install/mod.rs**

Add `pub mod kubectl;` at the top and add to `all_installers()`:

```rust
pub mod go;
pub mod kubectl;
```

```rust
pub fn all_installers() -> Vec<Box<dyn Installer>> {
    vec![
        Box::new(go::Go),
        Box::new(kubectl::Kubectl),
    ]
}
```

- [ ] **Step 3: Build and verify**

Run: `cd rust && cargo build`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git add rust/src/install/kubectl.rs rust/src/install/mod.rs
git commit -m "feat: add kubectl installer with arch detection and checksum"
```

---

### Task 10: Rust (rustup) installer

**Files:**
- Create: `rust/src/install/rust_lang.rs`
- Modify: `rust/src/install/mod.rs`

- [ ] **Step 1: Create rust_lang.rs**

```rust
use crate::common::command;
use crate::common::platform::Platform;
use anyhow::Result;

pub struct RustLang;

impl super::Installer for RustLang {
    fn name(&self) -> &str { "rust" }

    fn needs_sudo(&self, _platform: &Platform) -> bool {
        false // rustup installs to ~/.cargo
    }

    fn is_installed(&self) -> bool {
        command::exists("rustc")
    }

    fn install(&self, _platform: &Platform) -> Result<()> {
        println!("Installing Rust via rustup...");
        command::run_visible(
            "sh",
            &["-c", "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"],
        )?;
        println!("Rust installed. Restart your shell or run: source ~/.cargo/env");
        Ok(())
    }
}
```

- [ ] **Step 2: Register in mod.rs**

Add `pub mod rust_lang;` and `Box::new(rust_lang::RustLang)` to `all_installers()`.

- [ ] **Step 3: Build**

Run: `cd rust && cargo build`

- [ ] **Step 4: Commit**

```bash
git add rust/src/install/rust_lang.rs rust/src/install/mod.rs
git commit -m "feat: add Rust installer via rustup"
```

---

## Chunk 3: Remaining tool installers

### Task 11: brew, docker, azure, dotnet installers

**Files:**
- Create: `rust/src/install/brew.rs`
- Create: `rust/src/install/docker.rs`
- Create: `rust/src/install/azure.rs`
- Create: `rust/src/install/dotnet.rs`
- Modify: `rust/src/install/mod.rs`

Each of these follows the same pattern: check platform, call brew or set up apt repo + install. I will not write out every line here — the pattern from go.rs/kubectl.rs is the template. Key notes per tool:

- [ ] **Step 1: Create brew.rs** — macOS only, calls `ensure_brew()`. Rejects WSL. `needs_sudo` returns false.

- [ ] **Step 2: Create docker.rs** — macOS: `brew install docker`. Linux: add Docker GPG key via `apt_add_gpg_key`, add Docker apt repo, `apt install docker-ce docker-ce-cli containerd.io docker-compose-plugin`. `needs_sudo` returns true on Linux.

- [ ] **Step 3: Create azure.rs** — macOS: `brew install azure-cli`. Linux: add Microsoft GPG key, add Azure CLI apt repo, `apt install azure-cli`. `needs_sudo` returns true on Linux. This replaces the dangerous `curl | sudo bash`.

- [ ] **Step 4: Create dotnet.rs** — macOS: `brew install dotnet`. Linux: detect distro via `/etc/os-release`, add Microsoft apt repo for the detected distro+version, `apt install dotnet-sdk-8.0`. `needs_sudo` returns true on Linux. Fails with clear error on unsupported distros.

- [ ] **Step 5: Register all four in mod.rs**

- [ ] **Step 6: Build**

Run: `cd rust && cargo build`

- [ ] **Step 7: Commit**

```bash
git add rust/src/install/brew.rs rust/src/install/docker.rs rust/src/install/azure.rs rust/src/install/dotnet.rs rust/src/install/mod.rs
git commit -m "feat: add brew, docker, azure, dotnet installers"
```

---

### Task 12: neovim, obsidian, java, github_cli installers

**Files:**
- Create: `rust/src/install/neovim.rs`
- Create: `rust/src/install/obsidian.rs`
- Create: `rust/src/install/java.rs`
- Create: `rust/src/install/github_cli.rs`
- Modify: `rust/src/install/mod.rs`

Key notes per tool:

- [ ] **Step 1: Create neovim.rs** — macOS: `brew install neovim`. Linux x86_64: download nvim.appimage from GitHub releases, install to `~/.mybin/nvim`. Linux aarch64: `apt install neovim` (appimage is x86-only). `needs_sudo` returns false on macOS, true on aarch64 Linux only.

- [ ] **Step 2: Create obsidian.rs** — macOS: `brew install --cask obsidian`. Linux: use GitHub Releases API (`https://api.github.com/repos/obsidianmd/obsidian-releases/releases/latest`) to get the latest .deb URL, download it, `apt install ./obsidian.deb`. `needs_sudo` returns true on Linux.

- [ ] **Step 3: Create java.rs** — macOS: `brew install openjdk`. Linux: `apt install default-jre default-jdk`. `needs_sudo` returns true on Linux.

- [ ] **Step 4: Create github_cli.rs** — macOS: `brew install gh`. Linux: add GitHub GPG key from `https://cli.github.com/packages/githubcli-archive-keyring.gpg`, add apt repo, `apt install gh`. `needs_sudo` returns true on Linux.

- [ ] **Step 5: Register all four in mod.rs**

- [ ] **Step 6: Build**

Run: `cd rust && cargo build`

- [ ] **Step 7: Commit**

```bash
git add rust/src/install/neovim.rs rust/src/install/obsidian.rs rust/src/install/java.rs rust/src/install/github_cli.rs rust/src/install/mod.rs
git commit -m "feat: add neovim, obsidian, java, github CLI installers"
```

---

### Task 13: terraform, postgres, javascript installers

**Files:**
- Create: `rust/src/install/terraform.rs`
- Create: `rust/src/install/postgres.rs`
- Create: `rust/src/install/javascript.rs`
- Modify: `rust/src/install/mod.rs`

- [ ] **Step 1: Create terraform.rs** — macOS: `brew install terraform`. Linux: add HashiCorp GPG key, add apt repo (`https://apt.releases.hashicorp.com`), `apt install terraform`. `needs_sudo` returns true on Linux.

- [ ] **Step 2: Create postgres.rs** — macOS: `brew install postgresql`. Linux: `apt install postgresql postgresql-contrib`. `needs_sudo` returns true on Linux.

- [ ] **Step 3: Create javascript.rs** — Installs nvm, pnpm, bun, and yarn. Each as a function:
  - `install_nvm()`: `curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash`
  - `install_pnpm()`: `curl -fsSL https://get.pnpm.io/install.sh | sh -`
  - `install_bun()`: `curl -fsSL https://bun.sh/install | bash`
  - `install_yarn()`: macOS: `brew install yarn`. Linux: add Yarn GPG key, apt repo, `apt install yarn`.
  - The `install()` method runs all four in sequence. `needs_sudo` returns true on Linux (for yarn apt repo only).

- [ ] **Step 4: Register all three in mod.rs**

- [ ] **Step 5: Build and run `bashc install all` to see the full summary**

Run: `cd rust && cargo build && cargo run -- install all`
Expected: Summary listing all 14 tools

- [ ] **Step 6: Commit**

```bash
git add rust/src/install/terraform.rs rust/src/install/postgres.rs rust/src/install/javascript.rs rust/src/install/mod.rs
git commit -m "feat: add terraform, postgres, javascript installers — all tools complete"
```

---

## Chunk 4: Parallel execution, interactive menu, CI/CD, and bootstrap

### Task 14: Parallel execution for `install all`

**Files:**
- Modify: `rust/src/install/mod.rs`

- [ ] **Step 1: Add dependency ordering and parallel execution**

Replace the sequential `run_all` with a version that:
1. Runs `brew` first on macOS (dependency for most macOS tools)
2. Runs all non-JS tools in parallel using `tokio::task::spawn_blocking`
3. Runs `nvm` (from javascript installer), then pnpm/bun/yarn
4. Collects all results

Add a `phase` method to the Installer trait:

```rust
/// Installation phase for ordering. Lower phases run first.
/// Phase 0: prerequisites (brew)
/// Phase 1: independent tools (parallel)
/// Phase 2: JS tools (nvm first, then rest)
fn phase(&self) -> u8 { 1 } // default: independent
```

- [ ] **Step 2: Implement parallel run_all**

Use `tokio::task::spawn_blocking` since installers use blocking I/O (subprocess calls). Group by phase, run each phase's tools concurrently, collect results.

- [ ] **Step 3: Test with `cargo run -- install all`**

Expected: Tools in the same phase install concurrently, phases run sequentially.

- [ ] **Step 4: Commit**

```bash
git add rust/src/install/mod.rs
git commit -m "feat: parallel tool installation for install all"
```

---

### Task 15: Interactive menu

**Files:**
- Modify: `rust/src/install/mod.rs`
- Modify: `rust/src/main.rs`

- [ ] **Step 1: Add interactive selection using dialoguer**

```rust
pub fn run_interactive(platform: &Platform) -> Result<Vec<InstallResult>> {
    let installers = all_installers();
    let names: Vec<&str> = installers.iter().map(|i| i.name()).collect();

    let selections = dialoguer::MultiSelect::new()
        .with_prompt("Select tools to install")
        .items(&names)
        .interact()?;

    let selected: Vec<_> = selections
        .into_iter()
        .map(|i| installers[i].as_ref())
        .collect();

    // Run selected installers using run_one
    Ok(selected.iter().map(|i| run_one(*i, platform)).collect())
}
```

- [ ] **Step 2: Wire into main.rs**

In the `Install` match arm, when `interactive` is true, call `install::run_interactive(&platform)?` and print summary.

- [ ] **Step 3: Test**

Run: `cargo run -- install --interactive`
Expected: Shows checkboxes for all tools

- [ ] **Step 4: Commit**

```bash
git add rust/src/install/mod.rs rust/src/main.rs
git commit -m "feat: add interactive tool selection menu"
```

---

### Task 16: GitHub Actions release workflow

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Create the release workflow**

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

permissions:
  contents: write

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install musl tools
        if: contains(matrix.target, 'musl')
        run: sudo apt-get install -y musl-tools

      - name: Build
        working-directory: rust
        run: cargo build --release --target ${{ matrix.target }}

      - name: Rename binary
        run: |
          cp rust/target/${{ matrix.target }}/release/bashc bashc-${{ matrix.target }}

      - name: Generate checksum
        run: shasum -a 256 bashc-${{ matrix.target }} > bashc-${{ matrix.target }}.sha256

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: bashc-${{ matrix.target }}
          path: |
            bashc-${{ matrix.target }}
            bashc-${{ matrix.target }}.sha256

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          merge-multiple: true

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: |
            bashc-*
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "feat: add GitHub Actions release workflow for cross-compilation"
```

---

### Task 17: Bootstrap init.sh

**Files:**
- Create: `init.sh`

- [ ] **Step 1: Write the POSIX bootstrap script**

```sh
#!/bin/sh
set -e

REPO="SindreFonnes/bashCustomization"

# Detect OS
case "$(uname -s)" in
    Darwin) OS="apple-darwin" ;;
    Linux)  OS="unknown-linux-gnu" ;;
    *)      echo "Error: Unsupported OS: $(uname -s)"; exit 1 ;;
esac

# Detect architecture
case "$(uname -m)" in
    x86_64)  ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *)       echo "Error: Unsupported architecture: $(uname -m)"; exit 1 ;;
esac

TARGET="${ARCH}-${OS}"
BINARY_NAME="bashc-${TARGET}"

echo "Detected platform: ${TARGET}"

# Get latest release URL
RELEASE_URL=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep "browser_download_url.*${BINARY_NAME}\"" \
    | head -1 \
    | cut -d '"' -f 4)

if [ -z "$RELEASE_URL" ]; then
    echo "Error: No release found for ${TARGET}"
    echo "Available at: https://github.com/${REPO}/releases"
    exit 1
fi

CHECKSUM_URL="${RELEASE_URL}.sha256"

echo "Downloading bashc for ${TARGET}..."
curl -fsSL -o /tmp/bashc "$RELEASE_URL"
curl -fsSL -o /tmp/bashc.sha256 "$CHECKSUM_URL"

# Verify checksum
echo "Verifying checksum..."
EXPECTED=$(cat /tmp/bashc.sha256 | awk '{print $1}')
if command -v sha256sum > /dev/null 2>&1; then
    ACTUAL=$(sha256sum /tmp/bashc | awk '{print $1}')
elif command -v shasum > /dev/null 2>&1; then
    ACTUAL=$(shasum -a 256 /tmp/bashc | awk '{print $1}')
else
    echo "Warning: No sha256 tool found, skipping verification"
    ACTUAL="$EXPECTED"
fi

if [ "$EXPECTED" != "$ACTUAL" ]; then
    echo "Error: Checksum mismatch"
    echo "  Expected: $EXPECTED"
    echo "  Got:      $ACTUAL"
    exit 1
fi

chmod +x /tmp/bashc
echo "bashc downloaded and verified."
echo ""

# Run install all (user can also pass specific args)
if [ $# -eq 0 ]; then
    echo "Running: bashc install all"
    /tmp/bashc install all
else
    /tmp/bashc "$@"
fi
```

- [ ] **Step 2: Make executable**

```bash
chmod +x init.sh
```

- [ ] **Step 3: Commit**

```bash
git add init.sh
git commit -m "feat: add bootstrap init.sh for fresh machine setup"
```

---

### Task 18: Update shell integration

**Files:**
- Modify: `installScripts/installMain.sh`

- [ ] **Step 1: Update `run_my_install` to use bashc binary if available**

```bash
run_my_install () {
    # Prefer the Rust binary if available
    if command -v bashc &> /dev/null; then
        bashc install "$1"
        return $?
    fi
    # Fallback to legacy shell scripts
    "$MYINSTALL_SCRIPT_LOCATION" $1 $2;
}
```

- [ ] **Step 2: Commit**

```bash
git add installScripts/installMain.sh
git commit -m "feat: update run_my_install to prefer bashc binary"
```

---

### Task 19: Final integration test

- [ ] **Step 1: Build release binary**

Run: `cd rust && cargo build --release`

- [ ] **Step 2: Test single install**

Run: `./target/release/bashc install go`
Expected: Installs or skips Go with proper output

- [ ] **Step 3: Test install all**

Run: `./target/release/bashc install all`
Expected: Summary showing status of all 14 tools

- [ ] **Step 4: Test interactive**

Run: `./target/release/bashc install --interactive`
Expected: Shows checkboxes, installs selected tools

- [ ] **Step 5: Test unknown tool**

Run: `./target/release/bashc install foobar`
Expected: Error with list of available tools

- [ ] **Step 6: Final commit and tag**

```bash
git add -A
git commit -m "feat: bashc phase 1 complete — all install tools ported to Rust"
git tag v0.1.0
git push && git push --tags
```

This triggers the GitHub Actions release workflow, producing binaries for all 4 platforms.
