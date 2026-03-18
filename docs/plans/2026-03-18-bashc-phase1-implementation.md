# `bashc` Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `bashc` Rust binary with all install subcommands, CI/CD for precompiled releases, and a bootstrap init script.

**Architecture:** Single Rust crate with clap subcommands. Each tool installer implements a common interface (trait or function set — implementer decides). `bashc install all` runs installers in parallel where possible, collects errors, and reports a summary. Sudo requirements are checked upfront before any work begins. Distributed via GitHub Releases; a small POSIX init.sh bootstraps fresh machines.

**Tech Stack:** Rust (2024 edition), clap, reqwest, sha2, semver, serde/serde_json, tokio, indicatif, dialoguer. Use latest crate versions — do not pin.

**Spec:** `docs/specs/2026-03-18-bashc-binary-design.md`

---

## File structure

The required modules are listed below. Simple installers (e.g., `brew install X` on macOS, `apt install X` on Linux) can live in a single file. More complex installers (e.g., Go with version API + checksum, or javascript with 4 sub-tools) may warrant a dedicated module directory. The implementing agent should use their judgement — prefer the simplest structure that keeps things readable.

```
rust/
  Cargo.toml
  src/
    main.rs                    # clap CLI, tokio runtime entry
    install/
      mod.rs                   # installer interface, registry, parallel orchestrator, sudo pre-flight
      ...                      # one file per simple installer, or a directory for complex ones
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

**Required installers** (14 tools): go, kubectl, rust, docker, azure, dotnet, neovim, obsidian, brew, java, github CLI, terraform, postgres, javascript (nvm/pnpm/bun/yarn).

---

## Chunk 1: Scaffold and shared libraries

### Task 1: Initialize the Rust crate

**Files:**
- Create: `rust/Cargo.toml`
- Create: `rust/src/main.rs`

- [ ] **Step 1:** Create `Cargo.toml` with 2024 edition. Add dependencies: `clap` (with derive feature), `reqwest` (with blocking and json features), `sha2`, `semver`, `serde` (with derive), `serde_json`, `tokio` (with full features), `indicatif`, `dialoguer`, `anyhow`, `libc`. Use latest versions for all — do not pin.

- [ ] **Step 2:** Create a minimal `main.rs` that sets up a clap CLI with a single `install` subcommand accepting a tool name string and an `--interactive` flag. Use tokio as the async runtime. For now, just print the detected platform and the requested tool name.

- [ ] **Step 3:** Verify it compiles with `cargo build` and runs with `cargo run -- install go`.

- [ ] **Step 4:** Commit.

---

### Task 2: Platform detection module

**Files:**
- Create: `rust/src/common/mod.rs`
- Create: `rust/src/common/platform.rs`
- Modify: `rust/src/main.rs`

**Requirements:**
- Define an `Os` enum with variants: `MacOs`, `Linux`, `Wsl`
- Define an `Arch` enum with variants: `X86_64`, `Aarch64`
- Define a `Platform` struct holding both
- Implement `Platform::detect()` that returns `Result<Platform>`:
  - Use `cfg!(target_os = ...)` for OS detection
  - On Linux, read `/proc/version` to detect WSL
  - Use `std::env::consts::ARCH` for architecture
  - Return a clear error on unsupported OS or arch, listing what was detected and what is supported
- Add helper methods: `is_mac()`, `is_linux()` (true for both Linux and WSL), `is_wsl()`
- Add methods that return Go-style strings (`go_os()` -> "darwin"/"linux", `go_arch()` -> "amd64"/"arm64") since several download URLs use this format
- Write tests: detection returns a valid platform on the current machine, Go-style strings are correct for known platform combinations

- [ ] **Step 1:** Create the module with types, detection, and helpers.
- [ ] **Step 2:** Wire into main.rs — print detected platform on startup.
- [ ] **Step 3:** Run `cargo test` — all tests pass.
- [ ] **Step 4:** Commit.

---

### Task 3: Command execution module

**Files:**
- Create: `rust/src/common/command.rs`
- Modify: `rust/src/common/mod.rs`

**Requirements:**
- `run(program, args) -> Result<String>` — run a command, capture stdout, fail on non-zero exit
- `run_visible(program, args) -> Result<()>` — run a command inheriting stdin/stdout/stderr (user sees output), fail on non-zero exit
- `exists(program) -> bool` — check if a command is on PATH
- `run_sudo(program, args) -> Result<()>` — run a command prefixed with `sudo`, inheriting stdio
- `is_root() -> bool` — check if the current process is running as root (use `libc::geteuid`)

- [ ] **Step 1:** Implement the module.
- [ ] **Step 2:** Verify it compiles.
- [ ] **Step 3:** Commit.

---

### Task 4: Download module

**Files:**
- Create: `rust/src/common/download.rs`
- Modify: `rust/src/common/mod.rs`

**Requirements:**
- `download_file(url, dest_path) -> Result<()>` — download a URL to a file, showing a progress bar via `indicatif`
- `fetch_text(url) -> Result<String>` — fetch a URL and return the body as text
- `fetch_json<T: DeserializeOwned>(url) -> Result<T>` — fetch a URL and deserialize the JSON response
- `verify_sha256(file_path, expected_hex) -> Result<()>` — compute SHA256 of a file and compare to expected hash, fail with a clear mismatch message
- Write tests: `verify_sha256` accepts correct hash for known content, rejects wrong hash. Use `tempfile` crate as a dev-dependency.

- [ ] **Step 1:** Implement the module.
- [ ] **Step 2:** Run `cargo test` — all tests pass.
- [ ] **Step 3:** Commit.

---

### Task 5: Package manager module

**Files:**
- Create: `rust/src/common/package_manager.rs`
- Modify: `rust/src/common/mod.rs`

**Requirements:**
- `install(platform, package) -> Result<()>` — install a package via brew (macOS) or apt (Linux)
- `brew_install_cask(package) -> Result<()>` — install a brew cask (macOS only)
- `ensure_brew() -> Result<()>` — check if brew is installed, install it if not (macOS only). Handle both `/opt/homebrew` (ARM) and `/usr/local` (Intel) paths.
- `apt_add_gpg_key(url, keyring_path) -> Result<()>` — download a GPG key and install it for apt
- `apt_add_repo(repo_line, list_file) -> Result<()>` — add an apt repository source file and run apt update
- `needs_sudo_for_apt(platform) -> bool` — returns true if on Linux and not root

- [ ] **Step 1:** Implement the module.
- [ ] **Step 2:** Verify it compiles.
- [ ] **Step 3:** Commit.

---

### Task 6: Version comparison module

**Files:**
- Create: `rust/src/common/version.rs`
- Modify: `rust/src/common/mod.rs`

**Requirements:**
- `parse(version_str) -> Result<Version>` — parse a version string, stripping common prefixes (`v`, `go`). Use the `semver` crate.
- `is_newer(current, new) -> Result<bool>` — returns true if `new` is greater than `current`
- Write tests: strips `v` prefix, strips `go` prefix, correctly compares versions, handles equal versions

- [ ] **Step 1:** Implement the module with tests.
- [ ] **Step 2:** Run `cargo test` — all tests pass.
- [ ] **Step 3:** Commit.

---

## Chunk 2: Installer interface and first tools

### Task 7: Installer interface and orchestrator

**Files:**
- Create: `rust/src/install/mod.rs`
- Modify: `rust/src/main.rs`

**Requirements:**

Define a common installer interface (trait or function set). Each installer must provide:
- `name() -> &str` — tool name used as the CLI argument
- `needs_sudo(platform) -> bool` — whether this tool needs root on the given platform
- `is_installed() -> bool` — check if already installed
- `install(platform) -> Result<()>` — perform the installation

Build an orchestrator that provides:
- A registry of all installers
- `find_installer(name) -> Option<...>` — look up by name
- `run_one(installer, platform) -> InstallResult` — run a single installer with pre-flight checks (skip if installed, fail if needs sudo and not root)
- `run_all(platform) -> Vec<InstallResult>` — run all installers. Check sudo upfront for all tools before starting any work. If any need sudo and we're not root, report them all and exit. Sequential for now (parallelism in Task 14).
- `print_summary(results)` — print a summary grouping results into installed/skipped/failed categories

Wire into `main.rs`: `bashc install <tool>` runs one, `bashc install all` runs all, unknown tool prints available list.

- [ ] **Step 1:** Create the install module with interface, orchestrator, and summary printer.
- [ ] **Step 2:** Update main.rs to use the install module.
- [ ] **Step 3:** Build to verify (will need at least one stub installer — create Go as a stub that just prints its name).
- [ ] **Step 4:** Commit.

---

### Task 8: Go installer

**Files:**
- Create: `rust/src/install/go.rs`

**Problem:** Install the Go language runtime with proper arch detection and checksum verification.

**Requirements:**
- On macOS: ensure brew, then `brew install go`
- On Linux: fetch the latest release from `https://go.dev/dl/?mode=json` (returns JSON array of releases with version, files, sha256 per file). Find the archive matching the current OS+arch. Download it, verify SHA256, extract to `/usr/local/go`. Needs sudo on Linux.
- The JSON response has a `files` array where each entry has `os`, `arch`, `sha256`, `filename`, `kind` fields. Filter by `kind == "archive"`.
- This fixes the hardcoded amd64 bug in the old shell script.

- [ ] **Step 1:** Implement the installer.
- [ ] **Step 2:** Register in the installer registry.
- [ ] **Step 3:** Build and test with `cargo run -- install go`.
- [ ] **Step 4:** Commit.

---

### Task 9: kubectl installer

**Files:**
- Create: `rust/src/install/kubectl.rs`
- Modify: `rust/src/install/mod.rs` (register)

**Problem:** Install kubectl with proper arch detection and checksum verification.

**Requirements:**
- On macOS: `brew install kubernetes-cli` and `brew install kubectx`
- On Linux: fetch latest version from `https://dl.k8s.io/release/stable.txt`. Download binary from `https://dl.k8s.io/release/<version>/bin/linux/<arch>/kubectl`. Verify SHA256 from `<url>.sha256`. Install to `/usr/local/bin/kubectl`. Needs sudo.

- [ ] **Step 1:** Implement and register.
- [ ] **Step 2:** Build.
- [ ] **Step 3:** Commit.

---

### Task 10: Rust (rustup) installer

**Files:**
- Create: `rust/src/install/rust_lang.rs`
- Modify: `rust/src/install/mod.rs` (register)

**Problem:** Install Rust via rustup.

**Requirements:**
- Download and run rustup-init with `-y` flag (unattended). Use the official install command: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y`
- Does not need sudo (installs to `~/.cargo`).
- Check if `rustc` exists to determine if already installed.

- [ ] **Step 1:** Implement and register.
- [ ] **Step 2:** Build.
- [ ] **Step 3:** Commit.

---

## Chunk 3: Remaining tool installers

### Task 11: brew, docker, azure, dotnet installers

**Files:**
- Create: `rust/src/install/brew.rs`
- Create: `rust/src/install/docker.rs`
- Create: `rust/src/install/azure.rs`
- Create: `rust/src/install/dotnet.rs`
- Modify: `rust/src/install/mod.rs` (register all)

**brew:**
- macOS only. Calls `ensure_brew()`. Rejects WSL explicitly. Does not need sudo.

**docker:**
- macOS: `brew install docker`
- Linux: add Docker GPG key (from `https://download.docker.com/linux/ubuntu/gpg`), add Docker apt repo, install `docker-ce docker-ce-cli containerd.io docker-compose-plugin`. Needs sudo on Linux.

**azure:**
- macOS: `brew install azure-cli`
- Linux: add Microsoft GPG key, add Azure CLI apt repo, install `azure-cli`. This replaces the dangerous `curl | sudo bash` in the old script. Needs sudo on Linux.

**dotnet:**
- macOS: `brew install dotnet`
- Linux: read `/etc/os-release` to detect distro and version dynamically. Add Microsoft apt repo for the detected distro. Install `dotnet-sdk-8.0`. Fail with a clear error on unsupported distros (not just Ubuntu 22.04). Needs sudo on Linux.

- [ ] **Step 1:** Implement all four installers.
- [ ] **Step 2:** Register in mod.rs.
- [ ] **Step 3:** Build.
- [ ] **Step 4:** Commit.

---

### Task 12: neovim, obsidian, java, github_cli installers

**Files:**
- Create: `rust/src/install/neovim.rs`
- Create: `rust/src/install/obsidian.rs`
- Create: `rust/src/install/java.rs`
- Create: `rust/src/install/github_cli.rs`
- Modify: `rust/src/install/mod.rs` (register all)

**neovim:**
- macOS: `brew install neovim`
- Linux x86_64: download `nvim.appimage` from GitHub releases, install to `~/.mybin/nvim`, make executable. Does not need sudo.
- Linux aarch64: appimage is x86-only, so use `apt install neovim` instead. Needs sudo only on aarch64 Linux.

**obsidian:**
- macOS: `brew install --cask obsidian`
- Linux: use the GitHub Releases API (`https://api.github.com/repos/obsidianmd/obsidian-releases/releases/latest`) to find the latest `.deb` asset URL. Download it, install with `apt install ./<file>.deb`. Replaces the fragile HTML scraping. Needs sudo on Linux.

**java:**
- macOS: `brew install openjdk`
- Linux: `apt install default-jre default-jdk`. Needs sudo on Linux.

**github_cli:**
- macOS: `brew install gh`
- Linux: add GitHub GPG key from `https://cli.github.com/packages/githubcli-archive-keyring.gpg`, add apt repo, `apt install gh`. Needs sudo on Linux.

- [ ] **Step 1:** Implement all four installers.
- [ ] **Step 2:** Register in mod.rs.
- [ ] **Step 3:** Build.
- [ ] **Step 4:** Commit.

---

### Task 13: terraform, postgres, javascript installers

**Files:**
- Create: `rust/src/install/terraform.rs`
- Create: `rust/src/install/postgres.rs`
- Create: `rust/src/install/javascript.rs`
- Modify: `rust/src/install/mod.rs` (register all)

**terraform:**
- macOS: `brew install terraform`
- Linux: add HashiCorp GPG key, add apt repo (`https://apt.releases.hashicorp.com`), `apt install terraform`. Needs sudo on Linux.

**postgres:**
- macOS: `brew install postgresql`
- Linux: `apt install postgresql postgresql-contrib`. Needs sudo on Linux.

**javascript:**
- This installer covers 4 sub-tools: nvm, pnpm, bun, yarn
- nvm: download and run the install script from `https://raw.githubusercontent.com/nvm-sh/nvm/<latest>/install.sh`
- pnpm: `curl -fsSL https://get.pnpm.io/install.sh | sh -`
- bun: `curl -fsSL https://bun.sh/install | bash`
- yarn: macOS `brew install yarn`, Linux add Yarn GPG key + apt repo + `apt install yarn`
- The install method should run all four in sequence (nvm first since it provides node, then the rest)
- Needs sudo on Linux (for yarn apt repo)

- [ ] **Step 1:** Implement all three installers.
- [ ] **Step 2:** Register in mod.rs.
- [ ] **Step 3:** Build and test `cargo run -- install all` — should list all 14 tools in the summary.
- [ ] **Step 4:** Commit.

---

## Chunk 4: Parallel execution, interactive menu, CI/CD, and bootstrap

### Task 14: Parallel execution for `install all`

**Files:**
- Modify: `rust/src/install/mod.rs`

**Problem:** `install all` currently runs sequentially. Tools should install in parallel where possible.

**Requirements:**
- Define installation phases (e.g., via a method on the installer interface or a separate ordering function):
  - Phase 0: prerequisites (`brew` on macOS)
  - Phase 1: all independent tools (go, rust, docker, azure, dotnet, neovim, obsidian, java, github, terraform, postgres, kubectl) — run in parallel
  - Phase 2: JS tools — nvm first, then pnpm/bun/yarn in parallel
- Use `tokio::task::spawn_blocking` since installers call subprocesses (blocking I/O)
- Collect results from all phases into a single summary

- [ ] **Step 1:** Add phase ordering to the installer interface.
- [ ] **Step 2:** Implement parallel `run_all` grouped by phase.
- [ ] **Step 3:** Test with `cargo run -- install all`.
- [ ] **Step 4:** Commit.

---

### Task 15: Interactive menu

**Files:**
- Modify: `rust/src/install/mod.rs`
- Modify: `rust/src/main.rs`

**Problem:** `bashc install --interactive` should show a multi-select menu of all tools.

**Requirements:**
- Use `dialoguer::MultiSelect` to show checkboxes for all registered tools
- Run selected installers through the same orchestrator (with sudo pre-flight and summary)
- Wire into the `--interactive` flag in main.rs

- [ ] **Step 1:** Implement interactive selection.
- [ ] **Step 2:** Wire into main.rs.
- [ ] **Step 3:** Test with `cargo run -- install --interactive`.
- [ ] **Step 4:** Commit.

---

### Task 16: GitHub Actions release workflow

**Files:**
- Create: `.github/workflows/release.yml`

**Problem:** Precompiled binaries need to be built and published for every release.

**Requirements:**
- Trigger on version tags (`v*`)
- Build matrix for all 4 targets: `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`
- Install musl-tools on Ubuntu for the musl target
- Use `dtolnay/rust-toolchain@stable` with the appropriate target
- Name output binaries as `bashc-<target>` (e.g., `bashc-aarch64-apple-darwin`)
- Generate SHA256 checksum files per binary
- Upload all binaries and checksums as GitHub Release assets using `softprops/action-gh-release` or similar

- [ ] **Step 1:** Create the workflow file.
- [ ] **Step 2:** Commit.

---

### Task 17: Bootstrap init.sh

**Files:**
- Create: `init.sh`

**Problem:** A fresh machine needs to download and run the correct bashc binary with zero dependencies beyond `curl` and a POSIX shell.

**Requirements:**
- POSIX-compatible shell script (sh, not bash)
- Detect OS via `uname -s` (Darwin/Linux) and arch via `uname -m` (x86_64/aarch64/arm64)
- Error clearly on unsupported platforms
- Fetch the latest release URL from the GitHub API (`https://api.github.com/repos/<repo>/releases/latest`)
- Download the correct binary and its `.sha256` file
- Verify the checksum (handle both `sha256sum` and `shasum -a 256` since macOS uses the latter)
- Make executable and run — default to `bashc install all` if no arguments given, otherwise pass through all arguments
- This replaces `init_repo.sh`

- [ ] **Step 1:** Write the script.
- [ ] **Step 2:** Make executable (`chmod +x`).
- [ ] **Step 3:** Commit.

---

### Task 18: Update shell integration

**Files:**
- Modify: `installScripts/installMain.sh`

**Problem:** The existing `run_my_install` shell function should prefer the Rust binary when available.

**Requirements:**
- Check if `bashc` is on PATH. If so, run `bashc install <tool>` and return.
- Otherwise fall back to the existing shell script dispatch.

- [ ] **Step 1:** Update `run_my_install` function.
- [ ] **Step 2:** Commit.

---

### Task 19: Final integration test and release

- [ ] **Step 1:** Build release binary: `cd rust && cargo build --release`
- [ ] **Step 2:** Test `bashc install go` — should install or skip with proper output.
- [ ] **Step 3:** Test `bashc install all` — summary showing status of all 14 tools.
- [ ] **Step 4:** Test `bashc install --interactive` — shows checkboxes, installs selected.
- [ ] **Step 5:** Test `bashc install foobar` — error with list of available tools.
- [ ] **Step 6:** Commit, tag `v0.1.0`, push with tags. This triggers the GitHub Actions release workflow.
