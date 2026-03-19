# `bashc` Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `bashc` Rust binary with all install subcommands, CI/CD for precompiled releases, and a bootstrap init script.

**Architecture:** Single Rust crate with clap subcommands. Each tool installer implements a common interface (trait or function set — implementer decides). `bashc install all` runs installers in parallel where possible, collects errors, and reports a summary. Sudo requirements are checked upfront before any work begins. Distributed via GitHub Releases; a small POSIX init.sh bootstraps fresh machines.

**Tech Stack:** Rust (2024 edition), clap, reqwest, sha2, semver, serde/serde_json, tokio, indicatif, dialoguer. Use latest crate versions — do not pin.

**Spec:** `docs/specs/2026-03-18-bashc-binary-design.md`

**Package manager preference:** On macOS, Debian-based, and Fedora-based systems, Homebrew (including Linuxbrew) is the preferred installation method. Only fall back to native package managers (apt, dnf) when brew is unavailable or the package has a known brew-specific limitation. On other distros (Arch, NixOS, Alpine, etc.), use the native package manager directly. See the spec for full rationale.

---

## Testing approach

**What to test:** Pure functions and logic that don't touch the filesystem or run subprocesses. These are the areas where bugs are subtle and tests are cheap.

- `common/version.rs` — prefix stripping, comparison logic. Easy to test, important to get right.
- `common/download.rs` — `verify_sha256` (hash a known byte string, check match/mismatch). Skip testing actual HTTP downloads.
- `common/platform.rs` — `go_os()`/`go_arch()` return correct strings for known platform values. `Platform::detect()` returns a valid result on the current machine.
- Installer registry — `find_installer` returns the right tool, unknown names return None.

**What NOT to test:** Anything that runs shell commands, installs packages, downloads files, or modifies the system. These are integration concerns — verify them manually during Task 19 (final integration test). Trying to sandbox-test `brew install` or `apt-get` would add complexity with little value.

**Guideline for the implementer:** If a function is pure (data in, data out, no side effects), consider writing a test if the logic is non-trivial. If it shells out or touches the filesystem beyond temp files, skip it.

---

## Return type guidance

The function signatures below use `Result<T>` as a suggestion, not a prescription. The return types should reflect what actually makes sense in context — evaluate whether `Result` is appropriate and what the inner type should be when implementing each function. Some functions may need to return data the caller uses, others may only need to signal success/failure, and some might not need `Result` at all. Let the implementation context drive these decisions.

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

**Required installers** (19 tools): go, kubectl, rust, docker, azure, dotnet, neovim, obsidian, brew, java, github CLI, terraform, postgres, javascript (nvm/pnpm/bun/yarn), ripgrep, bat, fd, eza, shellcheck.

---

## Chunk 1: Scaffold and shared libraries

### Task 1: Initialize the Rust crate

- [ ] **Step 1:** Create `Cargo.toml` with 2024 edition. Add dependencies: `clap` (with derive feature), `reqwest` (with blocking and json features), `sha2`, `semver`, `serde` (with derive), `serde_json`, `tokio` (with full features), `indicatif`, `dialoguer`, `anyhow`, `libc`. Use latest versions for all.

- [ ] **Step 2:** Create a minimal `main.rs` that sets up a clap CLI with a single `install` subcommand accepting a tool name string and an `--interactive` flag. Use tokio as the async runtime. For now, just print the detected platform and the requested tool name.

- [ ] **Step 3:** Verify it compiles with `cargo build` and runs with `cargo run -- install go`.

- [ ] **Step 4:** Commit.

---

### Task 2: Platform detection module

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

**Requirements:**
- `install(platform, package) -> Result<()>` — install a package, preferring brew on macOS/Debian/Fedora, falling back to apt/dnf if brew is unavailable, using native package manager directly on other distros
- `brew_install(package) -> Result<()>` — install a package via brew
- `brew_install_cask(package) -> Result<()>` — install a brew cask (macOS only — casks not supported on Linuxbrew)
- `ensure_brew() -> Result<()>` — check if brew is installed, install it if not. On macOS: handle both `/opt/homebrew` (ARM) and `/usr/local` (Intel) paths. On Debian/Fedora Linux: install Linuxbrew to `/home/linuxbrew/.linuxbrew`. On other distros: no-op or skip.
- `has_brew() -> bool` — check if brew is available on PATH
- `apt_add_gpg_key(url, keyring_path) -> Result<()>` — download a GPG key and install it for apt
- `apt_add_repo(repo_line, list_file) -> Result<()>` — add an apt repository source file and run apt update
- `needs_sudo_for_apt(platform) -> bool` — returns true if on Linux and not root

- [ ] **Step 1:** Implement the module.
- [ ] **Step 2:** Verify it compiles.
- [ ] **Step 3:** Commit.

---

### Task 6: Version comparison module

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

**Problem:** Install the Go language runtime with proper arch detection and checksum verification.

**Requirements:**
- Preferred: `brew install go` (macOS, Debian-based, Fedora-based)
- Fallback (no brew): fetch the latest release from `https://go.dev/dl/?mode=json` (returns JSON array of releases with version, files, sha256 per file). Find the archive matching the current OS+arch. Download it, verify SHA256, extract to `/usr/local/go`. Needs sudo on Linux.
- The JSON response has a `files` array where each entry has `os`, `arch`, `sha256`, `filename`, `kind` fields. Filter by `kind == "archive"`.
- This fixes the hardcoded amd64 bug in the old shell script.

- [ ] **Step 1:** Implement the installer.
- [ ] **Step 2:** Register in the installer registry.
- [ ] **Step 3:** Build and test with `cargo run -- install go`.
- [ ] **Step 4:** Commit.

---

### Task 9: kubectl installer

**Problem:** Install kubectl with proper arch detection and checksum verification.

**Requirements:**
- Preferred: `brew install kubernetes-cli` and `brew install kubectx` (macOS, Debian-based, Fedora-based)
- Fallback (no brew): fetch latest version from `https://dl.k8s.io/release/stable.txt`. Download binary from `https://dl.k8s.io/release/<version>/bin/linux/<arch>/kubectl`. Verify SHA256 from `<url>.sha256`. Install to `/usr/local/bin/kubectl`. Needs sudo.

- [ ] **Step 1:** Implement and register.
- [ ] **Step 2:** Build.
- [ ] **Step 3:** Commit.

---

### Task 10: Rust (rustup) installer

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

**brew:**
- macOS, Debian-based, and Fedora-based systems. Calls `ensure_brew()` to install Homebrew (macOS) or Linuxbrew (Linux). Since brew is the preferred package manager on these distro families, this should run first. On other distros (Arch, NixOS, Alpine, etc.), skip brew and use the native package manager. Does not need sudo.

**docker:**
- Preferred: `brew install docker` (macOS, Debian-based, Fedora-based)
- Fallback (no brew, Linux): add Docker GPG key (from `https://download.docker.com/linux/ubuntu/gpg`), add Docker apt repo, install `docker-ce docker-ce-cli containerd.io docker-compose-plugin`. Needs sudo. **Brew limitation:** Docker Desktop via brew cask is macOS-only; on Linux, the apt path installs the daemon properly with systemd integration, so this is one case where apt is actually better on Linux if Docker Engine (not Desktop) is the goal. Use judgement: brew for Docker Desktop on macOS, apt for Docker Engine on Linux.

**azure:**
- Preferred: `brew install azure-cli` (macOS, Debian-based, Fedora-based)
- Fallback (no brew, Linux): add Microsoft GPG key, add Azure CLI apt repo, install `azure-cli`. This replaces the dangerous `curl | sudo bash` in the old script. Needs sudo.

**dotnet:**
- Preferred: `brew install dotnet` (macOS, Debian-based, Fedora-based)
- Fallback (no brew, Linux): read `/etc/os-release` to detect distro and version dynamically. Add Microsoft apt repo for the detected distro. Install `dotnet-sdk-8.0`. Fail with a clear error on unsupported distros (not just Ubuntu 22.04). Needs sudo.

- [ ] **Step 1:** Implement all four installers.
- [ ] **Step 2:** Register in mod.rs.
- [ ] **Step 3:** Build.
- [ ] **Step 4:** Commit.

---

### Task 12: neovim, obsidian, java, github_cli installers

**neovim:**
- Preferred: `brew install neovim` (macOS, Debian-based, Fedora-based)
- Fallback (no brew, Linux x86_64): download `nvim.appimage` from GitHub releases, install to `~/.mybin/nvim`, make executable. Does not need sudo.
- Fallback (no brew, Linux aarch64): appimage is x86-only, so use `apt install neovim` instead. Needs sudo.

**obsidian:**
- macOS: `brew install --cask obsidian`
- Linux: use the GitHub Releases API (`https://api.github.com/repos/obsidianmd/obsidian-releases/releases/latest`) to find the latest `.deb` asset URL. Download it, install with `apt install ./<file>.deb`. Replaces the fragile HTML scraping. Needs sudo on Linux. **Brew limitation:** Obsidian is a cask (GUI app) — on Linux, casks are not supported by Linuxbrew, so the .deb path is required.

**java:**
- Preferred: `brew install openjdk` (macOS, Debian-based, Fedora-based)
- Fallback (no brew, Linux): `apt install default-jre default-jdk`. Needs sudo.

**github_cli:**
- Preferred: `brew install gh` (macOS, Debian-based, Fedora-based)
- Fallback (no brew, Linux): add GitHub GPG key from `https://cli.github.com/packages/githubcli-archive-keyring.gpg`, add apt repo, `apt install gh`. Needs sudo.

- [ ] **Step 1:** Implement all four installers.
- [ ] **Step 2:** Register in mod.rs.
- [ ] **Step 3:** Build.
- [ ] **Step 4:** Commit.

---

### Task 13: terraform, postgres, javascript installers

**terraform:**
- Preferred: `brew install terraform` (macOS, Debian-based, Fedora-based)
- Fallback (no brew, Linux): add HashiCorp GPG key, add apt repo (`https://apt.releases.hashicorp.com`), `apt install terraform`. Needs sudo.

**postgres:**
- Preferred: `brew install postgresql` (macOS, Debian-based, Fedora-based)
- Fallback (no brew, Linux): `apt install postgresql postgresql-contrib`. Needs sudo. **Brew limitation:** Linuxbrew-installed postgres may have issues with systemd service management. If the user needs postgres as a system service on Linux, apt is safer. For development use, brew is fine.

**javascript:**
- This installer covers 4 sub-tools: nvm, pnpm, bun, yarn
- nvm: download and run the install script from `https://raw.githubusercontent.com/nvm-sh/nvm/<latest>/install.sh` (no brew equivalent)
- pnpm: `curl -fsSL https://get.pnpm.io/install.sh | sh -`
- bun: `curl -fsSL https://bun.sh/install | bash`
- yarn: preferred `brew install yarn`, fallback (no brew, Linux) add Yarn GPG key + apt repo + `apt install yarn`
- The install method should run all four in sequence (nvm first since it provides node, then the rest)
- Needs sudo on Linux only if brew is unavailable (for yarn apt repo)

- [ ] **Step 1:** Implement all three installers.
- [ ] **Step 2:** Register in mod.rs.
- [ ] **Step 3:** Build.
- [ ] **Step 4:** Commit.

---

### Task 13b: ripgrep, bat, fd, eza, shellcheck installers

These 5 tools were recently added to the shell install scripts. They all follow the brew-first preference on macOS/Debian/Fedora: try brew first, fall back to native package manager only if brew is unavailable. On other distros, use the native package manager directly.

**ripgrep:**
- Preferred: `brew install ripgrep`
- Fallback: `apt install ripgrep`. Needs sudo.
- Check: `rg` on PATH.

**bat:**
- Preferred: `brew install bat` (also avoids Debian/Ubuntu `batcat` naming issue)
- Fallback: `apt install bat`. On Debian/Ubuntu the binary is installed as `batcat` — create a symlink `~/.local/bin/bat -> batcat` if needed. Needs sudo.
- Check: `bat` or `batcat` on PATH.

**fd:**
- Preferred: `brew install fd` (also avoids Debian/Ubuntu `fdfind` naming issue)
- Fallback: `apt install fd-find`. On Debian/Ubuntu the binary is installed as `fdfind` — create a symlink `~/.local/bin/fd -> fdfind` if needed. Needs sudo.
- Check: `fd` or `fdfind` on PATH.

**eza:**
- Preferred: `brew install eza` (also avoids needing third-party apt repo)
- Fallback: requires adding GPG key from `https://raw.githubusercontent.com/eza-community/eza/main/deb.asc` to `/etc/apt/keyrings/gierens.gpg`, adding repo `deb [signed-by=/etc/apt/keyrings/gierens.gpg] http://deb.gierens.de stable main`, then `apt install eza`. Needs sudo.
- Check: `eza` on PATH.

**shellcheck:**
- Preferred: `brew install shellcheck`
- Fallback: `apt install shellcheck`. Needs sudo.
- Check: `shellcheck` on PATH.

- [ ] **Step 1:** Implement all five installers.
- [ ] **Step 2:** Register in mod.rs.
- [ ] **Step 3:** Build and test `cargo run -- install all` — should list all 19 tools in the summary.
- [ ] **Step 4:** Commit.

---

## Chunk 4: Parallel execution, interactive menu, CI/CD, and bootstrap

### Task 14: Parallel execution for `install all`

**Problem:** `install all` currently runs sequentially. Tools should install in parallel where possible.

**Requirements:**
- Define installation phases (e.g., via a method on the installer interface or a separate ordering function):
  - Phase 0: prerequisites (`brew` on macOS, Debian-based, and Fedora-based systems; skip on Arch/NixOS/Alpine)
  - Phase 1: all independent tools (go, rust, docker, azure, dotnet, neovim, obsidian, java, github, terraform, postgres, kubectl, ripgrep, bat, fd, eza, shellcheck) — run in parallel
  - Phase 2: JS tools — nvm first, then pnpm/bun/yarn in parallel
- Use `tokio::task::spawn_blocking` since installers call subprocesses (blocking I/O)
- Collect results from all phases into a single summary

- [ ] **Step 1:** Add phase ordering to the installer interface.
- [ ] **Step 2:** Implement parallel `run_all` grouped by phase.
- [ ] **Step 3:** Test with `cargo run -- install all`.
- [ ] **Step 4:** Commit.

---

### Task 15: Interactive menu

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
- [ ] **Step 3:** Test `bashc install all` — summary showing status of all 19 tools.
- [ ] **Step 4:** Test `bashc install --interactive` — shows checkboxes, installs selected.
- [ ] **Step 5:** Test `bashc install foobar` — error with list of available tools.
- [ ] **Step 6:** Commit, tag `v0.1.0`, push with tags. This triggers the GitHub Actions release workflow.
