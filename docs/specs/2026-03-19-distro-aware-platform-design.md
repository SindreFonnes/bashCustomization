# Distro-Aware Platform Support

## Problem

The current platform model is binary: macOS or Linux (assumed Debian). Every
installer calls `apt_install()` when `is_linux()` is true. Running on Fedora,
Arch, Alpine, or NixOS would call `apt-get` and fail.

## Target Distros

| Distro family | Package manager | Brew as primary? | Status |
|---------------|-----------------|-------------------|--------|
| macOS | Homebrew | Yes (only option) | Implemented |
| Debian/Ubuntu | Homebrew → apt fallback | Yes | Implemented |
| Fedora/RHEL | Homebrew → dnf fallback | Yes | Detect now, implement later |
| Arch/Manjaro | pacman | No | Detect now, implement later |
| Alpine | apk | No | Detect now, implement later |
| NixOS | Declarative guidance (no imperative installs) | No | Detect now, implement later |

## Design

### 1. Distro Detection (`platform.rs`)

The `Os` enum currently has `MacOs`, `Linux`, and `Wsl`. It needs to carry
distro information for Linux and WSL variants.

**Requirements:**
- Add a distro type with variants for each supported family: Debian (covers Ubuntu, Pop!_OS, Mint, etc.), Fedora (covers RHEL, CentOS Stream, Rocky, Alma), Arch (covers Manjaro, EndeavourOS), Alpine, NixOS, and an unknown fallback that captures the detected ID string
- `Linux` and `Wsl` OS variants carry a distro value. macOS has no distro
- Detection reads `/etc/os-release` and matches on the `ID` and `ID_LIKE` fields. Derivatives should be matched via `ID_LIKE` (e.g., Ubuntu's `ID_LIKE=debian`, Rocky's `ID_LIKE="rhel centos fedora"`)
- macOS does not have `/etc/os-release` — handle its absence gracefully
- Existing helpers (`is_linux()`, `is_mac()`, `is_wsl()`) continue to work
- Add convenience methods for querying distro: `distro()` accessor, plus `is_debian()`, `is_fedora()`, etc.
- Update `Display` impl to include distro name in output
- Update existing tests and add tests for distro detection with known `os-release` content

### 2. Privilege Escalation (`common/privilege.rs`)

The current code hardcodes `sudo` via `command::run_sudo()`. Different distros
ship different escalation tools — or none at all:
- macOS always has `sudo`
- Base Debian may only have `su` (no `sudo`)
- Alpine typically has neither `sudo` nor `doas` on fresh installs
- Other distros vary

**Requirements:**
- Create a privilege escalation module that replaces `command::run_sudo()`
- Detect available escalation method at runtime by checking PATH for `sudo`, `doas`, and `su` (in that preference order)
- If already running as root, skip escalation and run commands directly
- If no escalation method is found, error with a clear message directing the user to install doas via `bashc install doas`
- All existing `command::run_sudo()` call sites migrate to the new privilege module
- Handle the `su` case correctly — `su -c` requires the full command as a single string argument, unlike `sudo`/`doas` which accept program + args directly

### 3. Doas Installer

Alpine ships with neither `sudo` nor `doas`. On a fresh install the user is
typically root. A dedicated doas installer enables bootstrapping privilege
escalation before other tools need it.

**Requirements:**
- Add a `doas` tool to the installer registry (tool #21)
- Can be run explicitly via `bashc install doas`
- Requires root to install — error clearly if not root
- On Alpine: install via `apk`. On Debian: install via `apt`. Other distros: "not yet supported" message. macOS: "not applicable, sudo is built-in"
- After installing the binary, configure it: create the doas config directory and a config file that permits the `wheel` group to escalate with credential caching
- Print guidance about adding the user to the `wheel` group
- During pre-flight checks in the orchestrator: if running as root and no escalation method is detected, auto-invoke the doas installer on Alpine before proceeding with other tools. This ensures subsequent tools can escalate privileges as needed

### 4. Package Manager Routing (`package_manager.rs`)

The `install()` function currently assumes Linux means apt. It needs to route
to the correct package manager based on detected distro.

**Requirements:**
- macOS → Homebrew (unchanged)
- Debian and Fedora → Homebrew first, fall back to native package manager (apt / dnf) if brew is unavailable or failed. This preserves the existing brew-first behavior
- Arch → pacman directly
- Alpine → apk directly
- NixOS → do not install imperatively. Instead, print declarative guidance showing what to add to system configuration or home-manager (e.g., the package name in `environment.systemPackages`). Return success (not an error) since the user has been told what to do
- Unknown distro → error with the detected distro name and a list of supported distros
- Unimplemented distro paths (Fedora, Arch, Alpine native installs) should return a clear error: "not yet supported on {distro}, would install: {package}". These are stubs to be filled in later
- The existing `brew_install`, `brew_install_cask`, and `apt_install` functions remain. Add stub functions for `dnf`, `pacman`, and `apk` that error with "not yet implemented"
- The `ensure_brew()` function should be aware that brew is only applicable on macOS, Debian, and Fedora — no-op or skip on other distros

### 5. Distro-Specific Operations (repos, GPG keys)

Six installers (azure, docker, dotnet, eza, github, terraform) directly call
`apt_add_gpg_key` and `apt_add_repo`. These operations are Debian-specific.

**Requirements:**
- These operations need to be guarded behind a distro check
- On Debian: existing apt-based repo/key logic works as-is
- On other distros: error with "third-party repo setup not yet supported on {distro}" for now
- When Fedora support is implemented later, equivalent `dnf` repo operations will be needed (different mechanism — `.repo` files in `/etc/yum.repos.d/` rather than apt sources)
- Consider making repo/key operations distro-aware at the package_manager level rather than in each installer, to avoid duplicating the distro checks across six tools

### 6. Installer Changes

Most installers only call `package_manager::install()` and will get distro
routing for free without any code changes.

**No changes needed** (already route through `package_manager::install()`):
bat, brew, fd, java, kubectl, postgres, ripgrep, rust_lang, shellcheck

**Need Debian guard on direct apt calls** (bail with clear message on other distros for now):
azure, docker, dotnet, eza, github, terraform

**Need distro-aware logic**:
- `base.rs` — the apt package list (build-essential, nala, libfuse2, etc.) is Debian-specific. Other distros have different base package names or don't need them. For now, only run on Debian; other distros get a "base packages not yet configured for {distro}" message
- `obsidian.rs` — uses `dpkg -i` directly on Linux to install the `.deb`. Only applicable to Debian-based distros; guard accordingly
- `neovim.rs` — the appimage path is distro-agnostic, but the apt fallback for aarch64 needs a Debian guard

### 7. `needs_sudo` Changes

Currently hardcoded as `platform.is_linux() && !has_brew()`.

**Requirements:**
- NixOS → never needs sudo for package installs (declarative, no system changes)
- Alpine → depends on whether running as root and whether doas is available
- Other distros → depends on whether brew is available and whether the native package manager requires root
- The logic should account for the privilege escalation method, not just assume sudo

### 8. Bootstrap Init Script (`init.sh`)

The existing `init.sh` handles downloading and running the bashc binary on a
fresh machine. It needs to be distro-aware for Alpine.

**Requirements:**
- `init.sh` already detects OS and arch. Extend it to detect the distro family by reading `/etc/os-release` (same `ID`/`ID_LIKE` approach as the Rust platform module)
- On Alpine, if running as root and neither `sudo`, `doas`, nor `su` is available: install and configure doas via `apk` before downloading/running bashc. This is the shell equivalent of the Rust doas installer — kept minimal since it only needs to cover Alpine's `apk add doas` + basic config
- This ensures that when `bashc install all` runs, privilege escalation is already available
- On non-Alpine distros, no changes to the existing init flow

## Scope

This change builds the detection and routing infrastructure. Only macOS and
Debian have working install paths. All other distros are properly detected
and get clear "not yet supported on {distro}" messages. Each distro's install
commands can be implemented independently later without touching the routing
infrastructure.

## Files Changed

| File | Change |
|------|--------|
| `common/platform.rs` | Add distro type, `/etc/os-release` parsing, update `Os` enum |
| `common/privilege.rs` | New file: runtime escalation detection, replaces hardcoded sudo |
| `common/package_manager.rs` | Distro-aware routing, stub implementations for unimplemented distros |
| `common/command.rs` | Remove `run_sudo`, delegate to privilege module |
| `common/mod.rs` | Add privilege module |
| `install/tools/doas.rs` | New file: doas installer for bootstrapping privilege escalation |
| `install/tools/azure.rs` | Guard apt calls behind Debian check |
| `install/tools/docker.rs` | Guard apt calls behind Debian check |
| `install/tools/dotnet.rs` | Guard apt calls behind Debian check |
| `install/tools/eza.rs` | Guard apt calls behind Debian check |
| `install/tools/github.rs` | Guard apt calls behind Debian check |
| `install/tools/terraform.rs` | Guard apt calls behind Debian check |
| `install/tools/base.rs` | Guard apt package list behind Debian check |
| `install/tools/obsidian.rs` | Guard dpkg call behind Debian check |
| `install/tools/neovim.rs` | Guard apt fallback behind Debian check |
| `install/orchestrator.rs` | Add doas pre-flight bootstrap when running as root with no escalation |
| `init.sh` | Add distro detection, install doas on Alpine before running bashc |
