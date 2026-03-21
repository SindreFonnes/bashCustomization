# E2E Integration Test Crate Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a standalone Rust test crate that uses Docker (via `bollard`) to run bashc inside real Linux containers and verify behavior per distro.

**Architecture:** Standalone crate at `tests/e2e/` with a library layer (`src/`) for container management, distro config, and assertion helpers, plus a test layer (`tests/`) with one module directory per distro. Each distro module spins up one container and runs multiple assertions against it. Images are auto-built on first run and cached.

**Tech Stack:** Rust, bollard (Docker API), tokio, anyhow

**Spec:** `docs/specs/2026-03-19-e2e-test-crate.md`

---

## Testing approach

These are integration tests — the thing being tested is the bashc binary inside Docker containers. The test crate itself doesn't need unit tests; the tests ARE the product. Verify the crate works by running `cargo test` and confirming Docker containers are created, commands execute, and assertions pass/fail as expected.

---

## Chunk 1: Crate scaffold and container lifecycle

### Task 1: Initialize the e2e crate

**Files:**
- Create: `tests/e2e/Cargo.toml`
- Create: `tests/e2e/src/lib.rs`

**Requirements:**
- Standalone crate (not a workspace member) at `tests/e2e/`
- Dependencies: `bollard`, `tokio` (with full features), `anyhow`
- `src/lib.rs` is the library root — for now just declare the modules that will be created in subsequent tasks
- Verify it compiles with `cargo check`

- [ ] **Step 1:** Create `Cargo.toml` with dependencies.
- [ ] **Step 2:** Create `src/lib.rs` with placeholder module declarations (commented out or empty).
- [ ] **Step 3:** Run `cargo check` from `tests/e2e/` — compiles.
- [ ] **Step 4:** Commit.

---

### Task 2: Container lifecycle module

**Files:**
- Create: `tests/e2e/src/container.rs`
- Modify: `tests/e2e/src/lib.rs` (add `pub mod container`)

**Context:** This is the core infrastructure. It wraps `bollard` to manage Docker containers for tests. The existing Dockerfiles live at `tests/docker/Dockerfile.*` relative to the repository root. The `Dockerfile.builder` compiles a musl binary; the distro Dockerfiles copy a pre-built `bashc` binary into the container.

**Requirements:**
- A struct that holds a `bollard::Docker` client, the container ID, and the image name
- **Image build:** Accept a Dockerfile path and build context path. Use `bollard`'s image build API. The build context needs to include the Dockerfile and any files referenced by `COPY` instructions.
- **Auto-detect builds:** Check if image exists before building. Skip build if image exists and `REBUILD_IMAGES` env var is not set.
- **Binary extraction:** After building the builder image, extract the `/bashc` binary from it. This binary is then placed in the Docker build context for distro images. Use `bollard`'s container create + copy-from-container API.
- **Container create + start:** Create a container from the built image with a long-running command (e.g., `sleep infinity` or `tail -f /dev/null`) so it stays alive for multiple exec calls.
- **Exec:** Run a command inside the running container. Return a struct containing: stdout (String), stderr (String), and exit code (i64). Use `bollard`'s exec create + exec start API. Collect the output stream into the stdout/stderr strings.
- **Cleanup:** On drop (or via an explicit method), stop and remove the container. Implement `Drop` or provide an async cleanup method that tests call.
- All methods are async (bollard is async).

- [ ] **Step 1:** Create `container.rs` with the container wrapper struct and image build function.
- [ ] **Step 2:** Implement binary extraction from the builder image.
- [ ] **Step 3:** Implement container create, start, and exec.
- [ ] **Step 4:** Implement cleanup.
- [ ] **Step 5:** Wire into `lib.rs`.
- [ ] **Step 6:** Run `cargo check` — compiles.
- [ ] **Step 7:** Commit.

---

### Task 3: Distro configuration module

**Files:**
- Create: `tests/e2e/src/distro.rs`
- Modify: `tests/e2e/src/lib.rs` (add `pub mod distro`)

**Context:** Each distro has different config: image name, Dockerfile path, expected distro label in bashc output, whether to skip on arm64. This module provides pre-defined configs for all 6 distros.

**Requirements:**
- A config struct holding: image tag (e.g., `"bashc-test-debian"`), Dockerfile path relative to repo root, the expected distro label in bashc output (e.g., `"Debian"`), a flag or check for whether the distro should be skipped on the current host architecture
- Pre-defined configurations for: Debian, Ubuntu, Fedora, Arch, Alpine, NixOS
- Arch should be marked as skip on `aarch64` (no arm64 Docker image available)
- A method or function to resolve the absolute Dockerfile path given the repository root. The repo root can be found by walking up from the crate's `CARGO_MANIFEST_DIR`
- A method that indicates whether the distro uses the musl binary (`Alpine`) vs glibc — though currently all use the musl binary since it's statically linked

- [ ] **Step 1:** Create `distro.rs` with the config struct and all 6 distro configurations.
- [ ] **Step 2:** Wire into `lib.rs`.
- [ ] **Step 3:** Run `cargo check` — compiles.
- [ ] **Step 4:** Commit.

---

### Task 4: Assertion helpers

**Files:**
- Create: `tests/e2e/src/assertions.rs`
- Modify: `tests/e2e/src/lib.rs` (add `pub mod assertions`)

**Context:** Tests need readable assertion helpers that produce clear failure messages. These operate on the exec result struct from `container.rs`.

**Requirements:**
- `assert_exit_ok(result)` — panics with full output if exit code != 0
- `assert_exit_err(result)` — panics if exit code == 0 (for testing expected failures)
- `assert_stdout_contains(result, substring)` — panics if stdout doesn't contain the substring, showing what was expected and the actual stdout
- `assert_stdout_not_contains(result, substring)` — panics if stdout contains the substring
- `assert_stderr_contains(result, substring)` — same for stderr
- Helpers that run a command and assert in one step:
  - `assert_command_exists(container, binary_name)` — runs `command -v <name>` inside the container, asserts exit 0
  - `assert_file_contains(container, path, content)` — runs `cat <path>` inside the container, asserts stdout contains content
- All assertion functions should include the command that was run, the full stdout/stderr, and the exit code in their panic messages for easy debugging

- [ ] **Step 1:** Create `assertions.rs` with all assertion functions.
- [ ] **Step 2:** Wire into `lib.rs`.
- [ ] **Step 3:** Run `cargo check` — compiles.
- [ ] **Step 4:** Commit.

---

## Chunk 2: First distro test (Debian) — prove the infrastructure works

### Task 5: Debian dry-run tests

**Files:**
- Create: `tests/e2e/tests/debian/mod.rs`
- Create: `tests/e2e/tests/debian/dry_run.rs`

**Context:** This is the first real test — it proves the entire infrastructure works end-to-end: image build, container lifecycle, exec, assertions. Debian is the best first target because it's a fully supported distro.

**Requirements:**
- `mod.rs` sets up the shared container for all Debian tests. Since Rust's test framework runs each `#[test]` function independently, sharing a container across tests in a module requires a synchronization mechanism — use `tokio::sync::OnceCell` or `std::sync::OnceLock` with a lazy-initialized container. The container is created on first use and cleaned up after all tests complete.
- `dry_run.rs` contains the dry-run test assertions:
  - `bashc install --dry-run all` exits 0
  - Output contains "Debian"
  - Output lists all 22 tools (check for a representative set of tool names: "go", "rust", "docker", "doas", "ripgrep", etc.)
  - Output does not contain "panic" or "RUST_BACKTRACE"
- Each assertion uses the helpers from `assertions.rs`
- The test file is `tests/debian/mod.rs` as a test target — Rust's integration test system treats each file in `tests/` as a separate test binary. A directory with `mod.rs` works as a test binary that can have sub-modules.

**Important:** Before this test can run, the builder image must exist and the bashc binary must be extracted. The container setup in `mod.rs` must handle this: ensure builder image exists → extract binary → ensure distro image exists → create container.

- [ ] **Step 1:** Create `tests/debian/mod.rs` with container setup using the Debian distro config.
- [ ] **Step 2:** Create `tests/debian/dry_run.rs` with dry-run assertions.
- [ ] **Step 3:** Run `cargo test --test debian` from `tests/e2e/` — should build images, start container, run assertions, clean up. This is the first real end-to-end test run.
- [ ] **Step 4:** Debug and fix any issues with the Docker API interaction (image build context, exec output streaming, etc.). This task may require iteration.
- [ ] **Step 5:** Commit.

---

### Task 6: Debian real-install and symlink tests

**Files:**
- Create: `tests/e2e/tests/debian/real_install.rs`
- Create: `tests/e2e/tests/debian/symlinks.rs`

**Context:** These tests verify that actual package installation works on Debian. They run inside the same container as the dry-run tests.

**Requirements:**

**real_install.rs:**
- Run `apt-get update -qq` first (package cache may be stale)
- Install ripgrep via `bashc install ripgrep` — assert exit 0
- Verify `rg --version` runs and output contains "ripgrep"
- This is a slow test (downloads packages) — that's expected for e2e

**symlinks.rs:**
- Install bat via `bashc install bat` — assert exit 0
- Verify `bat --version` works (not `batcat`)
- Install fd via `bashc install fd` — assert exit 0
- Verify `fd --version` works (not `fdfind`)
- These test the Debian-specific symlink creation logic

- [ ] **Step 1:** Create `real_install.rs` with ripgrep install test.
- [ ] **Step 2:** Create `symlinks.rs` with bat and fd install + symlink verification.
- [ ] **Step 3:** Run `cargo test --test debian` — all Debian tests pass.
- [ ] **Step 4:** Commit.

---

## Chunk 3: Remaining distro tests

### Task 7: Ubuntu tests

**Files:**
- Create: `tests/e2e/tests/ubuntu/mod.rs`
- Create: `tests/e2e/tests/ubuntu/dry_run.rs`
- Create: `tests/e2e/tests/ubuntu/real_install.rs`

**Requirements:**
- Same pattern as Debian but using Ubuntu distro config
- Dry-run: output contains "Debian" (Ubuntu is detected as Debian family via ID_LIKE)
- Real install: ripgrep via apt works
- No symlink tests needed (same behavior as Debian)

- [ ] **Step 1:** Create all three files following the Debian pattern.
- [ ] **Step 2:** Run `cargo test --test ubuntu` — passes.
- [ ] **Step 3:** Commit.

---

### Task 8: Fedora tests

**Files:**
- Create: `tests/e2e/tests/fedora/mod.rs`
- Create: `tests/e2e/tests/fedora/dry_run.rs`
- Create: `tests/e2e/tests/fedora/stub_messages.rs`

**Requirements:**
- Dry-run: output contains "Fedora"
- Stub messages: `bashc install ripgrep` returns a non-zero exit code with output containing "not yet supported" or "not yet implemented". The process must not panic or crash.

- [ ] **Step 1:** Create all three files.
- [ ] **Step 2:** Run `cargo test --test fedora` — passes.
- [ ] **Step 3:** Commit.

---

### Task 9: Arch tests

**Files:**
- Create: `tests/e2e/tests/arch/mod.rs`
- Create: `tests/e2e/tests/arch/dry_run.rs`
- Create: `tests/e2e/tests/arch/stub_messages.rs`

**Requirements:**
- Same pattern as Fedora but using Arch distro config
- Dry-run: output contains "Arch"
- Stub messages: same pattern as Fedora
- **Must skip entirely on aarch64/arm64 hosts** — the `archlinux:latest` image has no arm64 variant. The distro config's skip flag should be checked at the start of `mod.rs` and all tests skipped if the host is arm64.

- [ ] **Step 1:** Create all three files with arm64 skip logic.
- [ ] **Step 2:** Run `cargo test --test arch` — skips on arm64 Mac, would pass on x86_64.
- [ ] **Step 3:** Commit.

---

### Task 10: Alpine tests

**Files:**
- Create: `tests/e2e/tests/alpine/mod.rs`
- Create: `tests/e2e/tests/alpine/dry_run.rs`
- Create: `tests/e2e/tests/alpine/doas.rs`

**Requirements:**

**dry_run.rs:**
- `bashc install --dry-run all` exits 0
- Output contains "Alpine"

**doas.rs:**
- The container runs as root with no sudo/doas/su preinstalled
- `bashc install doas` exits 0
- `doas` binary exists on PATH after install (use `assert_command_exists`)
- `/etc/doas.d/doas.conf` exists and contains "permit persist" (use `assert_file_contains`)
- After doas is installed, `bashc install --dry-run all` still exits 0

- [ ] **Step 1:** Create all three files.
- [ ] **Step 2:** Run `cargo test --test alpine` — passes.
- [ ] **Step 3:** Commit.

---

### Task 11: NixOS tests

**Files:**
- Create: `tests/e2e/tests/nixos/mod.rs`
- Create: `tests/e2e/tests/nixos/dry_run.rs`
- Create: `tests/e2e/tests/nixos/guidance.rs`

**Requirements:**

**dry_run.rs:**
- `bashc install --dry-run all` exits 0
- Output contains "NixOS" or "nixos" (case-insensitive check)

**guidance.rs:**
- The dry-run or a real install attempt produces output containing declarative guidance (e.g., "environment.systemPackages" or similar NixOS configuration hints)

Note: The `nixos/nix` Docker image has a non-standard PATH. The existing `Dockerfile.nixos` places bashc at `/root/.nix-profile/bin/bashc`. This should work with the container setup.

- [ ] **Step 1:** Create all three files.
- [ ] **Step 2:** Run `cargo test --test nixos` — passes.
- [ ] **Step 3:** Commit.

---

## Chunk 4: Full suite verification

### Task 12: Run full test suite and fix issues

- [ ] **Step 1:** Run `cargo test` from `tests/e2e/` with no filters — all distro tests execute.
- [ ] **Step 2:** Fix any failures or container cleanup issues.
- [ ] **Step 3:** Run `cargo test` again — all pass (Arch skipped on arm64).
- [ ] **Step 4:** Verify containers are cleaned up: `docker ps -a` should show no leftover `bashc-test-*` containers.
- [ ] **Step 5:** Test the `REBUILD_IMAGES` flag: `REBUILD_IMAGES=1 cargo test --test debian` — should rebuild images before running tests.
- [ ] **Step 6:** Commit any final fixes.
