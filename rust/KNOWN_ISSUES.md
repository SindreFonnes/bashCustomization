# Known Issues

## Resolved (2026-03-21)

The following Ubuntu/Debian issues have been fixed by adding a `Distro::Ubuntu`
variant to the platform enum and a shared `get_apt_codename()` helper:

- **Docker**: now uses correct repo URL per distro (`linux/ubuntu` vs `linux/debian`)
- **Azure CLI, .NET SDK, Terraform**: no longer fall back to `"jammy"` — error
  if `VERSION_CODENAME` is missing instead of silently using wrong codename
- **Base packages**: `add-apt-repository universe` is now guarded behind an
  Ubuntu check (skipped on plain Debian)

## Remaining issues

### Non-Debian Linux distro support is stub-only

Fedora/dnf, Arch/pacman, and Alpine/apk package manager backends return "not
yet implemented" errors. These distros are detected but cannot install most
tools through native package managers. Homebrew is available as a workaround
on Fedora.

### Base package list may not be fully Debian-compatible

- `nala` — may not be available in all Debian versions or repos
- `safe-rm` — may not be in all Debian repos

These packages are installed via `apt-get install -y` which will skip
unavailable packages if they're in a batch install, but individual failures
aren't caught.

### No self-install mechanism

There is no `bashc install bashc` command to install the binary to a permanent
location (e.g., `~/.local/bin`). Currently, `init.sh` downloads to a temp
directory and runs it, but doesn't persist the binary.
