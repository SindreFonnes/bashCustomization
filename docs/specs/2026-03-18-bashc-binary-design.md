# `bashc` — Unified CLI for shell customization

## Overview

A single Rust binary that replaces all shell-based install scripts, general scripts, and platform utilities in the bashCustomization project. Distributed as precompiled binaries via GitHub Releases so no Rust toolchain is needed on the target machine.

## Goals

- A fresh machine can be fully set up by cloning the repo and running a single init command
- Bulletproof and maintainable code that works for all supported environments
- Explicit errors on unsupported platforms — no silent misbehavior
- Replace all 19 fragile shell install scripts with proper error handling, arch detection, and checksum verification

## Supported platforms

| Target | Description |
|--------|-------------|
| `x86_64-apple-darwin` | Intel Mac |
| `aarch64-apple-darwin` | Apple Silicon |
| `x86_64-unknown-linux-gnu` | Linux x86_64 (glibc) |
| `x86_64-unknown-linux-musl` | Linux x86_64 static (Alpine/WSL) |

Any other platform produces a clear error showing what was detected and what is supported.

## CLI structure

```
bashc install go               # install a specific tool
bashc install all              # install everything (parallel where possible)
bashc install base             # install platform base packages (replaces installStuff.sh)
bashc install --interactive    # TUI menu for selecting tools
bashc scripts git-config       # (Phase 2) interactive git configuration
bashc scripts gpg-setup        # (Phase 2) GPG signing setup
bashc platform                 # (Phase 3) output platform vars for eval
bashc version compare 1.2 1.3  # (Phase 3) semver comparison
bashc init                     # (Phase 4) generate shell config for eval
```

## Package manager preference

On **macOS, Debian-based, and Fedora-based** systems, **Homebrew is the preferred installation method** if the package is available through it. This applies to Linuxbrew on Linux as well. Only fall back to the native package manager (apt, dnf) when:

- The package is not available in Homebrew
- There is a known limitation when installed via Homebrew (e.g., system services that need systemd integration, packages that require tight OS-level integration)
- Homebrew is not available and the user opted not to install it

**Does not apply to:** Arch-based (use pacman/yay), NixOS (use nix), Alpine (use apk), or other distros with their own strong package ecosystems. On these systems, use the native package manager directly.

**Rationale:** Homebrew provides consistent package names, avoids distro-specific quirks (e.g., Debian renaming `bat` to `batcat`, `fd` to `fdfind`), doesn't require sudo, and gives more up-to-date versions. Using a single package manager across macOS and the most common Linux distros simplifies the installer logic.

Each installer should: detect the distro family → if macOS/Debian/Fedora, check if brew is available → try `brew install` → fall back to native package manager if brew is unavailable or the package has a known brew limitation. On other distros, go straight to the native package manager.

## Sudo handling

Before starting any install, `bashc` checks whether the requested operation needs root privileges (e.g., apt operations on Linux). If it does and the process is not running as root, it exits immediately with a clear message:

```
Error: Installing docker requires sudo on Linux.
Re-run with: sudo bashc install docker
```

For `bashc install all`, it checks all requested tools upfront before doing anything and reports which ones need sudo:

```
Error: The following tools require sudo on this platform: docker, azure, terraform, github
Re-run with: sudo bashc install all
```

## Error handling for `install all`

Failures are collected, not fatal. Each tool installs independently. At the end a summary is printed:

```
Installed 12/14 tools successfully.

Skipped (already installed):
  go — v1.22.0

Failed:
  dotnet — Ubuntu 24.04 not yet supported
  docker — apt-get returned exit code 100

Failed tools can be retried individually: bashc install <tool>
```

## Parallel installation

`bashc install all` installs tools in parallel where possible:

1. **Pre-flight phase**: Check sudo requirements for all tools upfront. Detect already-installed tools and skip them.
2. **Base packages**: Install Homebrew (if macOS, debian based or fedora based system) and platform base packages (git, gnupg via brew; build-essential, git, safe-rm, keychain, nala, gnupg, pkg-config, libssl-dev, zip, unzip, tar, gzip, net-tools, libfuse2, libnss3-tools via apt on Linux). This replaces `installStuff.sh`.
3. **Parallel batch**: Install all independent tools concurrently — go, rust, docker, azure, dotnet, neovim, obsidian, java, github, terraform, postgres, kubectl, ripgrep, bat, fd, eza, shellcheck.
4. **Sequential JS batch**: Install nvm first (provides node), then pnpm, bun, and yarn in parallel.
5. **Report**: Print summary of successes, skips, and failures.

Individual tool installs (`bashc install go`) run serially — parallelism only applies to `all`.

## Installer trait

Each tool implements a common interface:

```rust
trait Installer {
    fn name(&self) -> &str;
    fn needs_sudo(&self, platform: &Platform) -> bool;
    fn is_installed(&self) -> bool;
    fn install(&self, platform: &Platform) -> Result<()>;
}
```

This gives uniform behavior across all tools: check if installed, check sudo, run install, report result.

## Crate layout

```
rust/
  Cargo.toml                   # single crate (not a workspace — one binary)
  src/
    main.rs                    # clap CLI entry point
    install/
      mod.rs                   # install subcommand, Installer trait, parallel orchestration
      go.rs                    # Go language runtime
      kubectl.rs               # Kubernetes CLI
      rust_lang.rs             # Rust via rustup
      docker.rs                # Docker Engine
      azure.rs                 # Azure CLI
      dotnet.rs                # .NET SDK
      neovim.rs                # Neovim editor
      obsidian.rs              # Obsidian notes app
      brew.rs                  # Homebrew (macOS)
      java.rs                  # OpenJDK
      github.rs                # GitHub CLI
      terraform.rs             # Terraform
      postgres.rs              # PostgreSQL
      javascript.rs            # nvm, pnpm, bun, yarn
      ripgrep.rs               # ripgrep (rg)
      bat.rs                   # bat (handles batcat symlink on Debian/Ubuntu)
      fd.rs                    # fd (handles fdfind symlink on Debian/Ubuntu)
      eza.rs                   # eza (third-party apt repo on Linux)
      shellcheck.rs            # ShellCheck
    common/
      mod.rs
      platform.rs              # OS/arch detection with explicit unsupported errors
      version.rs               # semver comparison
      download.rs              # HTTP download with progress + SHA256 checksum verification
      package_manager.rs       # brew-first on macOS/Debian/Fedora, native pkg manager on other distros
      command.rs               # subprocess execution with stdout/stderr capture
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing with subcommands |
| `reqwest` | HTTP client for downloads and API calls |
| `sha2` | SHA256 checksum verification |
| `semver` | Semantic version parsing and comparison |
| `serde` + `serde_json` | JSON parsing (GitHub API, go.dev API) |
| `tokio` | Async runtime for parallel downloads and installs |
| `indicatif` | Progress bars for downloads |
| `dialoguer` | Interactive TUI menu for `--interactive` |

## Tool-specific notes

### Tools with complex install logic (high-value ports)

- **go**: Fetch latest version from go.dev/dl/?mode=json API, detect OS+arch (fixes hardcoded amd64 bug), download, verify SHA256, extract to /usr/local/go
- **kubectl**: Download from dl.k8s.io, verify SHA256 checksum, install to /usr/local/bin. Detect arch properly.
- **neovim**: brew on macOS, appimage on x86_64 Linux, apt or build from source on aarch64 Linux (appimage is x86-only)
- **obsidian**: Use GitHub Releases API instead of HTML scraping. Download .deb on Linux, brew cask on macOS.
- **docker**: brew on macOS. On Linux: add Docker GPG key, add apt repo, install docker-ce packages.
- **azure**: brew on macOS. On Linux: add Microsoft GPG key, add apt repo, install azure-cli. Replaces dangerous `curl | sudo bash`.
- **dotnet**: brew on macOS. On Linux: detect distro and version dynamically instead of hardcoding Ubuntu 22.04.

### Tools with simple logic

- **rust**: Download and run rustup-init with `-y` flag
- **java**: brew install openjdk (macOS), apt install default-jdk (Linux)
- **github**: brew install gh (macOS), add GPG key + apt repo (Linux)
- **terraform**: brew install terraform (macOS), add HashiCorp GPG key + apt repo (Linux)
- **postgres**: brew install postgresql (macOS), apt install postgresql (Linux)
- **brew**: macOS only, run Homebrew's official install script
- **javascript**: nvm via install script, pnpm via install script, bun via install script, yarn via brew/apt
- **ripgrep**: brew install ripgrep (all), apt install ripgrep (Linux)
- **bat**: brew install bat (all). On Linux apt installs as `batcat` — needs `~/.local/bin/bat` symlink
- **fd**: brew install fd (all). On Linux apt package is `fd-find`, installs as `fdfind` — needs `~/.local/bin/fd` symlink
- **eza**: brew install eza (all). On Linux: add GPG key + third-party apt repo (deb.gierens.de)
- **shellcheck**: brew install shellcheck (all), apt install shellcheck (Linux)

## CI/CD: GitHub Actions release workflow

Triggered on version tags (`v*`):

1. Build for all 4 supported targets using `cross` or platform-native runners
2. Name binaries by target: `bashc-x86_64-apple-darwin`, `bashc-aarch64-apple-darwin`, etc.
3. Generate SHA256 checksums file (`checksums.txt`)
4. Upload binaries + checksums as GitHub Release assets

## Bootstrap: `init.sh`

A small POSIX-compatible shell script — the only shell code required for fresh machine setup. It:

1. Detects OS and architecture
2. Errors clearly on unsupported platforms
3. Downloads the correct `bashc` binary from the latest GitHub Release
4. Verifies the download against the checksums file
5. Makes the binary executable
6. Runs `bashc install all` (or whatever the user specifies)
7. Optionally runs `bashc scripts git-config` for initial git setup

This replaces the current `init_repo.sh`.

## What this does NOT replace

These must remain as shell scripts because they mutate the current shell process:

- `main.sh` — entry point sourced by `.bashrc`/`.zshrc`
- Alias definitions — must be `source`d
- `restart_shell` — uses `exec`
- `pushd_wrapper`/`popd_wrapper` — directory stack manipulation
- `ensure_ssh_agent` — platform-aware SSH agent setup
- `standard_settings.sh` — Oh-My-Zsh integration
- `local/` customization layer
