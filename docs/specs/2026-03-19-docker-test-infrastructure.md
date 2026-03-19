# Docker Test Infrastructure

## Problem

The bashc binary targets 6 Linux distro families but can only be tested on
the developer's host OS. There's no way to verify distro detection, package
manager routing, or doas bootstrapping work correctly on each target without
manually provisioning machines.

## Design

### Test matrix

| Image | Test level | What it verifies |
|-------|-----------|------------------|
| `debian:bookworm` | dry-run + real installs | Full apt path, distro detection as Debian |
| `ubuntu:24.04` | dry-run + real installs | ID_LIKE=debian derivative detection, apt path |
| `fedora:41` | dry-run only | Fedora detection, "not yet supported" stubs |
| `archlinux:latest` | dry-run only | Arch detection, "not yet supported" stubs |
| `alpine:3.21` | dry-run + doas bootstrap | Alpine detection, doas install via apk, privilege escalation |
| `nixos/nix:latest` | dry-run only | NixOS detection, declarative guidance output |

### File structure

```
tests/docker/
  run-tests.sh          # Main test runner script
  Dockerfile.debian
  Dockerfile.ubuntu
  Dockerfile.fedora
  Dockerfile.arch
  Dockerfile.alpine
  Dockerfile.nixos
```

### Test runner (`run-tests.sh`)

**Requirements:**
- Cross-compile bashc for `x86_64-unknown-linux-gnu` (glibc distros) and `x86_64-unknown-linux-musl` (Alpine) from the developer's Mac
- Use `cross` if available, fall back to `cargo build --target` with appropriate toolchains
- Build each Docker image, copying the correct binary into it
- Run tests for each distro and capture stdout/stderr
- Check exit codes and scan output for expected strings (distro name, "not yet supported", etc.)
- Print a summary table of pass/fail per distro
- Exit non-zero if any test failed

### Dockerfiles

Each Dockerfile should be minimal:
- Use the distro's official base image
- Install only `curl` if not present (needed for some installers)
- Copy the pre-built bashc binary to `/usr/local/bin/`
- No build toolchain inside the container — the binary is compiled on the host

### Test assertions per distro

**All distros (dry-run):**
- `bashc install --dry-run all` exits 0
- Output contains the correct distro name (e.g., "Debian", "Fedora", "Alpine")
- Output lists all 22 tools

**Debian/Ubuntu (real installs):**
- `bashc install ripgrep bat fd` exits 0
- `rg --version`, `bat --version`, `fd --version` succeed after install
- Tests run as root inside the container (no sudo needed)

**Alpine (doas bootstrap):**
- Container starts as root with no sudo/doas/su
- `bashc install doas` exits 0
- `doas` is on PATH after install
- `/etc/doas.d/doas.conf` exists with correct content
- `bashc install --dry-run all` succeeds after doas is installed

**Fedora/Arch (stub verification):**
- `bashc install --dry-run all` exits 0
- Real install of any tool returns a "not yet supported" error (not a crash)

**NixOS (guidance verification):**
- `bashc install --dry-run all` exits 0
- Output contains declarative guidance text (e.g., "environment.systemPackages")

### Cross-compilation

The script needs to produce two binaries:
- `x86_64-unknown-linux-gnu` — for Debian, Ubuntu, Fedora, Arch, NixOS
- `x86_64-unknown-linux-musl` — for Alpine

On macOS, this requires either:
- `cross` tool (recommended — handles toolchain setup automatically)
- Or manually installed cross-compilation toolchains via rustup

The script should detect which is available and use it, or error with
instructions to install `cross` (`cargo install cross`).
