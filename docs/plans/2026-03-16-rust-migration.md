# Rust Migration Plan for bashCustomization

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate the shell customization framework to Rust binaries distributed as precompiled releases, so a fresh machine can be fully set up by cloning the repo and running a single init command.

**Architecture:** A Rust workspace (`bashc`) containing multiple binary crates. Binaries are cross-compiled via GitHub Actions and distributed as GitHub Releases. A thin POSIX bootstrap script (`init.sh`) detects the platform, downloads the correct precompiled binary, and hands off to it. Shell scripts remain only for things that must mutate the current shell process (aliases, exports, sourcing). The Rust binary must fail with a clear error on unsupported platforms rather than silently misbehaving.

**Distribution model:**
```
Fresh machine → clone repo → run init.sh (~30 lines POSIX shell)
  → init.sh detects OS+arch
  → downloads bashc-install from GitHub Releases
  → bashc-install takes over: installs all tools, configures git, sets up shell
```

**Supported platforms:**
- `x86_64-apple-darwin` (Intel Mac)
- `aarch64-apple-darwin` (Apple Silicon)
- `x86_64-unknown-linux-gnu` (Linux x86_64)
- `x86_64-unknown-linux-musl` (Alpine/WSL static binary)

**Tech Stack:** Rust, clap (CLI), reqwest (HTTP), sha2 (checksums), semver (version comparison), serde/serde_json (config parsing), GitHub Actions (CI/CD cross-compilation)

---

## Phase 0: Pre-migration cleanup (shell-only, no Rust)

This phase prepares the codebase for migration by deduplicating logic, fixing interfaces, and ensuring clean module boundaries. **This phase is done as part of the current cleanup branch.**

### Task 0.1: Deduplicate OS detection

Currently duplicated across: `general_functions.sh`, `variables.sh` (`handle_wsl`), `commonMyinstallFunctions.sh` (`is_mac_os`/`is_wsl_os`/`is_linux_os`), `shellFunctions.sh` (`output_to_clipboad` uses `uname`), `standard_settings.sh` (checks `$SHELL` directly).

**Target state:** All modules use the `IS_MAC`/`IS_WSL` globals set by `determine_running_os` in `general_functions.sh`. Install scripts that run as subprocesses can source `general_functions.sh` or use their own `commonMyinstallFunctions.sh` helpers, but the logic should match.

- [x] Refactor `variables.sh` `handle_wsl()` to use `IS_WSL` instead of re-reading `/proc/version`
- [x] Fix `standard_settings.sh` to check `$PROFILE_SHELL` instead of `$SHELL`

### Task 0.2: Deduplicate `is_greater_than_current_version`

Identical function exists in `programExtensions/git/functions/gitAddCommitPushTag.sh` and `installScripts/go/installGo.sh`.

**Target state:** Single definition in a shared location. For now, keep both since they run in different contexts (sourced vs subprocess), but mark both with a comment pointing to the future Rust `semver` replacement.

- [x] Add comment to both copies: `# TODO(rust-migration): Replace with bashc semver compare`

### Task 0.3: Fix double-loading in main.sh

- [x] Remove the first `load_shell_extentionfiles "false"` call, keep only the `"first_load"` call after the update check

### Task 0.4: Remove redundant self-source in load_shell_extentionfiles

- [x] Remove `source $bashC/general_functions.sh` from inside `load_shell_extentionfiles` (it's already sourced by `main.sh` before the function is called)

---

## Phase 1: `bashc-install` — Install orchestration binary

**What replaces:** All 19 install scripts in `installScripts/`, `installScript.sh` menu, `installStuff.sh` base packages, `commonMyinstallFunctions.sh` utility functions.

**Why first:** Install scripts are standalone executables (not sourced), run infrequently, have the most complex and fragile logic (HTML scraping for versions, manual checksum verification, platform-specific branching), and produce no shell state mutations — they install software and exit.

**Crate location:** `rust/bashc-install/`

**Distribution:** Precompiled binaries via GitHub Releases. No Rust toolchain needed on the target machine. A thin POSIX bootstrap script downloads the correct binary for the platform.

### Task 1.1: Scaffold the Rust workspace

**Files:**
- Create: `rust/Cargo.toml` (workspace root)
- Create: `rust/bashc-install/Cargo.toml`
- Create: `rust/bashc-install/src/main.rs`
- Create: `rust/bashc-common/Cargo.toml` (shared library)
- Create: `rust/bashc-common/src/lib.rs`

- [ ] Initialize workspace with `bashc-install` binary and `bashc-common` library crate
- [ ] Add dependencies: `clap`, `reqwest`, `sha2`, `semver`, `serde`, `serde_json`, `tokio`
- [ ] Implement `bashc-common::platform` module: OS detection (mac/wsl/linux), arch detection (amd64/arm64), with explicit error on unsupported platforms
- [ ] Implement `bashc-common::version` module: semver comparison (replaces `is_greater_than_current_version`)
- [ ] Implement `bashc-common::download` module: download with progress, checksum verification
- [ ] Implement `bashc-common::package_manager` module: brew-first dispatch on macOS/Debian/Fedora (Linuxbrew on Linux), native package manager on other distros (Arch/NixOS/Alpine)

### Task 1.2: Port Go install script

The Go install script is the most mature (has checksum verification, version fetching) and a good template.

**Files:**
- Create: `rust/bashc-install/src/tools/go.rs`

- [ ] Implement Go installer: fetch latest version from go.dev API, detect OS+arch, download, verify sha256, extract, update PATH
- [ ] Wire into `bashc-install go` subcommand
- [ ] Test on macOS, verify arm64 detection works (fixes the hardcoded amd64 bug)

### Task 1.3: Port remaining install scripts

Port all install scripts as Rust subcommands. Priority order based on complexity:

- [ ] `kubectl` — already has checksum verification, benefits from arch detection
- [ ] `rust` — downloads and runs rustup-init
- [ ] `docker` — platform-specific (brew on macOS, apt repo on Linux)
- [ ] `azure` — replace dangerous `curl | sudo bash` with proper apt repo setup
- [ ] `dotnet` — replace hardcoded Ubuntu 22.04 with dynamic distro detection
- [ ] `neovim` — appimage is x86-only, needs arch-aware alternative (brew on macOS, appimage or apt on Linux)
- [ ] `obsidian` — replace HTML scraping with GitHub Releases API
- [ ] `brew` — macOS only, installs Homebrew
- [ ] `java` — brew/apt dispatch
- [ ] `github` — brew/apt dispatch with GPG key setup
- [ ] `terraform` — brew/apt dispatch with HashiCorp repo
- [ ] `postgres` — brew/apt dispatch
- [ ] `javascript` — nvm, pnpm, bun, yarn as sub-installers
- [ ] `ripgrep` — brew/apt dispatch (simple)
- [ ] `bat` — brew on all platforms, apt on Linux (note: Debian/Ubuntu installs as `batcat`, needs symlink handling)
- [ ] `fd` — brew on all platforms, apt on Linux (note: Debian/Ubuntu installs as `fdfind`, needs symlink handling)
- [ ] `eza` — brew on all platforms, apt on Linux (needs GPG key + third-party repo from deb.gierens.de)
- [ ] `shellcheck` — brew/apt dispatch (simple)

### Task 1.3.1: Port `installStuff.sh` base packages

The refactored `installStuff.sh` installs brew first, then base packages (git, gnupg via brew; build-essential, git, safe-rm, keychain, nala, gnupg, pkg-config, libssl-dev, zip, unzip, tar, gzip, net-tools, libfuse2, libnss3-tools via apt on Linux), then delegates to the 5 new dedicated install scripts. In the Rust binary, this becomes a `bashc install base` command that installs platform-appropriate base packages before individual tools.

- [ ] Implement `bashc install base` — installs foundational packages (brew packages on macOS, apt packages on Linux)
- [ ] Ensure `bashc install all` runs `base` as the first phase before individual tool installers

### Task 1.4: Interactive menu and `all` command

- [ ] Implement `bashc-install --interactive` with a proper TUI menu (replaces the fragile positional-parameter case statement in `installScript.sh`)
- [ ] Implement `bashc-install all` to run all installers sequentially
- [ ] Update `installMain.sh` to point `run_my_install` at the Rust binary

### Task 1.5: GitHub Actions CI/CD for cross-compilation

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] Set up GitHub Actions workflow triggered on version tags (e.g., `v*`)
- [ ] Cross-compile for all supported targets: `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`
- [ ] Upload binaries as GitHub Release assets with platform-specific names (e.g., `bashc-install-aarch64-apple-darwin`)
- [ ] Include SHA256 checksums file in the release

### Task 1.6: Bootstrap init script

**Files:**
- Create: `init.sh` (replaces current `init_repo.sh`)

- [ ] Write thin POSIX-compatible bootstrap script (~30 lines) that:
  - Detects OS (macOS/Linux) and architecture (x86_64/aarch64)
  - Errors clearly on unsupported platforms
  - Downloads the correct `bashc-install` binary from the latest GitHub Release
  - Verifies the download checksum
  - Makes the binary executable and runs it
- [ ] `bashc-install` then handles: git configuration, tool installation, shell setup
- [ ] Deprecate/remove `init_repo.sh` once `init.sh` is proven

---

## Phase 2: `bashc-scripts` — General script dispatch

**What replaces:** `gScriptRun.sh` case statement and standalone general scripts that are pure executables.

**Crate location:** `rust/bashc-scripts/`

### Task 2.1: Port git configuration

- [ ] Implement `bashc-scripts git-config` — interactive git configuration (replaces `configureGit.sh`, which was already fixed to use `git config --global` commands and the non-existent template path bug is resolved)

### Task 2.2: Port GPG signing setup

- [ ] Port `setupGpgSigning.sh` logic — already well-structured, good candidate

### Task 2.3: Port SSL cert generation

- [ ] Implement `bashc-scripts ssl-cert` — wraps openssl with proper defaults (passphrase, chmod 600)

### Task 2.4: Port launch-steam script

- [ ] Implement `bashc-scripts launch-steam` — sets SDL controller config env vars and launches Steam (replaces `launchSteam.sh`)
- [ ] Note: this is a niche script (gaming/controller workaround), low priority

### Task 2.5: Replace script dispatcher

- [ ] Implement `bashc-scripts --interactive` menu (replaces `gScriptRun.sh`)
- [ ] Update `gScriptMain.sh` to point at Rust binary

---

## Phase 3: `bashc` unified CLI with shared library

**What replaces:** Shared logic across modules.

**Crate location:** `rust/bashc/` (thin CLI wrapping `bashc-common`)

### Task 3.1: Platform detection command

- [ ] Implement `bashc platform` — outputs `IS_MAC=true; IS_WSL=false; PROFILE_SHELL=zsh; ARCH=arm64` for `eval`
- [ ] Optionally replace `determine_running_os` and `determine_running_shell` with `eval "$(bashc platform)"`

### Task 3.2: Version comparison command

- [ ] Implement `bashc version compare 1.2.3 1.3.0` — replaces `is_greater_than_current_version` shell function
- [ ] Implement `bashc version bump minor 1.2.3` — returns `1.3.0`

### Task 3.3: Clipboard command

- [ ] Implement `bashc clipboard` — cross-platform clipboard using `arboard` crate (replaces `output_to_clipboad`)
- [ ] Update `shellFunctions.sh` to call `bashc clipboard` instead of platform detection logic

---

## Phase 4: `bashc-init` — Shell config generation (long-term)

**What replaces:** The sourcing chain itself. Instead of sourcing 20+ files, `main.sh` becomes `eval "$(bashc-init)"`.

**This is the highest-risk phase** and should only be attempted after Phases 1-3 are stable.

### Task 4.1: Config file format

- [ ] Define a TOML/JSON config format that describes: which extensions to load, which aliases to define, PATH entries, environment variables
- [ ] Implement config parser in `bashc-common`

### Task 4.2: Shell output generation

- [ ] Implement `bashc-init` that reads config and outputs shell code (aliases, exports, function definitions)
- [ ] Support `--shell zsh` and `--shell bash` flags for shell-specific output
- [ ] Support `--dry-run` for debugging

### Task 4.3: Migration

- [ ] Replace `main.sh` sourcing chain with `eval "$(bashc-init)"`
- [ ] Keep shell-native functions (pushd/popd wrappers, restart_shell, etc.) in a minimal sourced file

---

## What permanently remains as shell

These cannot be replaced by external binaries because they mutate the current shell process:

- `main.sh` — entry point that gets sourced by `.bashrc`/`.zshrc`
- Alias definitions — must be `source`d to take effect
- `restart_shell` — uses `exec` to replace shell process
- `pushd_wrapper`/`popd_wrapper` — manipulate shell directory stack
- `load_shell_extentionfiles` — the sourcing mechanism itself (until Phase 4)
- `ensure_ssh_agent` — platform-aware SSH agent setup (macOS native keychain, systemd on Linux, keychain on WSL)
- `update_packages` — simple brew/apt wrapper, not worth porting
- `standard_settings.sh` — Oh-My-Zsh integration is deeply zsh-specific
- `local/` customization layer — user-specific, must remain flexible

## Key principle: "Rust computes, shell applies"

Rust handles: downloading, version comparison, checksumming, JIRA parsing, tag management, config generation, platform detection.

Shell handles: aliases, exports, PATH, directory stack, process replacement, sourcing.

The interface is stdout: Rust prints what to do, shell `eval`s or acts on it.
