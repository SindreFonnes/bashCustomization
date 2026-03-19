# E2E Integration Test Crate

## Problem

The bashc binary targets 6 Linux distro families but has no automated way to
verify behavior across them. The current shell-based Docker test runner
(`tests/docker/run-tests.sh`) works for quick smoke tests but doesn't scale:
assertions are string greps, adding new tests means more shell functions, and
there's no structured reporting.

## Goals

- Automated, repeatable integration tests that run bashc inside real Linux
  containers for each supported distro
- Structured assertions with clear failure messages (not grep on output)
- Easy to add new tests as distro support and installers expand
- One container startup per distro (not per test) to keep runtime reasonable
- Self-contained: builds images automatically if missing, no manual setup steps

## Design

### Crate location and structure

A standalone Rust crate at `tests/e2e/` (not a workspace member of the main
bashc crate — bashc is a binary, not a library). Uses `bollard` for Docker
API interaction and `tokio` as the async runtime.

The crate has two layers:
- **`src/`** — shared library code: container lifecycle, assertion helpers,
  distro configuration
- **`tests/`** — one module directory per distro, with sub-modules per
  feature area

### Distro modules

Each distro is a directory under `tests/` with a `mod.rs` and sub-modules:

```
tests/
  debian/
    mod.rs              # container setup, imports sub-modules
    dry_run.rs
    real_install.rs
    symlinks.rs         # Debian-specific: bat→batcat, fd→fdfind
  ubuntu/
    mod.rs
    dry_run.rs
    real_install.rs
  fedora/
    mod.rs
    dry_run.rs
    stub_messages.rs
  arch/
    mod.rs
    dry_run.rs
    stub_messages.rs
  alpine/
    mod.rs
    dry_run.rs
    doas.rs
  nixos/
    mod.rs
    dry_run.rs
    guidance.rs
```

Sub-modules can be split further as coverage grows (e.g.,
`debian/real_install/` becoming a directory with per-tool files).

### Container lifecycle (`src/container.rs`)

**Requirements:**
- A container wrapper that manages the full lifecycle: build image, create
  container, start, execute commands, stop, remove
- Build uses the existing Dockerfiles in `tests/docker/`
- The bashc musl binary is built first via `Dockerfile.builder`, then each
  distro image gets the binary copied in
- Auto-detect: skip image build if the image already exists. Rebuild if the
  `REBUILD_IMAGES` environment variable is set
- Execute commands inside a running container and return structured results:
  stdout, stderr, and exit code as separate fields
- Cleanup on drop — stop and remove the container so tests don't leak
  containers
- One container instance shared across all tests within a distro module

### Distro configuration (`src/distro.rs`)

**Requirements:**
- A distro type that holds: Docker image tag, Dockerfile path, expected distro
  label in bashc output, and host architecture skip conditions (e.g., Arch has
  no arm64 Docker image)
- Pre-defined configurations for all 6 distros
- Used by each distro's `mod.rs` to set up the container with the right config

### Assertion helpers (`src/lib.rs` or a sub-module)

**Requirements:**
- Helpers for common assertion patterns to keep test code readable:
  - Command exited successfully
  - Command failed (for testing error paths)
  - Stdout contains a substring
  - Stdout does not contain a substring
  - A binary exists on PATH inside the container
  - A file exists with expected content inside the container
- These should produce clear failure messages showing what was expected vs
  what was found, including the full command output on failure

### Test coverage per distro

**All distros (dry-run):**
- `bashc install --dry-run all` exits 0
- Output contains the correct distro label
- Output lists all registered tools (currently 22)
- No crash, no panic output

**Debian and Ubuntu (real installs):**
- Install of lightweight tools succeeds (ripgrep, bat, fd are good candidates
  — small, fast to install, easy to verify)
- The installed binary is functional (e.g., `rg --version` returns expected
  output)
- Debian-specific: bat/fd symlinks are created correctly (bat not batcat,
  fd not fdfind)
- Apt-based package installation works end-to-end

**Fedora and Arch (stub verification):**
- Dry-run succeeds
- Attempting a real install returns a clear "not yet supported" error
- The error message includes the distro name
- The process does not crash (exits with an error, not a panic/segfault)

**Alpine (doas bootstrap):**
- Container starts as root with no sudo/doas/su
- `bashc install doas` succeeds
- `doas` binary is on PATH after install
- The doas config file exists with the correct permission rule
- `bashc install --dry-run all` succeeds after doas is installed

**NixOS (declarative guidance):**
- Dry-run succeeds
- Output contains NixOS-specific guidance text (references to
  `environment.systemPackages` or similar)

### Image build strategy

The first time tests run (or when `REBUILD_IMAGES=1` is set):

1. Build the `bashc-builder` image from `Dockerfile.builder` — this compiles
   a statically-linked musl binary inside Docker
2. Extract the bashc binary from the builder image
3. Build each distro test image using its Dockerfile + the extracted binary

Subsequent runs skip all builds if images already exist. This keeps iteration
fast when running tests repeatedly.

### Running

```sh
cd tests/e2e
cargo test                          # all distros
cargo test debian                   # one distro
cargo test debian::dry_run          # one feature area
REBUILD_IMAGES=1 cargo test         # force rebuild
```

Tests should run with `--test-threads=1` by default or handle parallel
container creation safely. Since each distro uses a different image/container
name, parallel execution across distros should be safe, but parallel tests
within a distro sharing a container need sequential execution.

### Relationship to existing shell tests

The shell-based `tests/docker/run-tests.sh` remains as a quick manual smoke
test. The Rust e2e crate is the authoritative test suite for CI and thorough
verification. They share the same Dockerfiles.

### Dependencies

| Crate | Purpose |
|-------|---------|
| `bollard` | Docker API client |
| `tokio` | Async runtime for bollard |
| `anyhow` | Error handling in test helpers |

### Constraints

- Tests must not require any manual setup beyond having Docker running
- Tests must clean up after themselves (no leftover containers)
- Arch tests should be skipped automatically on arm64 hosts (no arm64 image)
- The test crate has no dependency on the bashc crate itself — it tests the
  binary via Docker, not the Rust code directly
