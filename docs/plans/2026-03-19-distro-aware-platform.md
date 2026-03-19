# Distro-Aware Platform Support Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend bashc's platform model to detect Linux distro families and route package manager operations accordingly, with proper privilege escalation detection.

**Architecture:** Add a `Distro` enum to the existing `Platform` struct, detected via `/etc/os-release`. A new `privilege` module replaces hardcoded `sudo` calls with runtime detection of `sudo`/`doas`/`su`. The `package_manager` module gains distro-aware routing with stubs for unimplemented distros. A `doas` installer bootstraps privilege escalation on Alpine. Only macOS and Debian paths are functional; all other distros get clear "not yet supported" messages.

**Tech Stack:** Rust (existing crate), POSIX shell (init.sh)

**Spec:** `docs/specs/2026-03-19-distro-aware-platform-design.md`

---

## Testing approach

**What to test:** Distro detection parsing logic (pure string matching on `/etc/os-release` content), privilege escalation method detection logic (which method is preferred), and distro convenience methods on `Platform`.

**What NOT to test:** Actual package installation, subprocess execution, file system changes. These are integration concerns verified manually.

**Guideline:** The `/etc/os-release` parsing function should accept a string parameter (not read the file itself) so it can be tested with known content for each distro family. This is the key testability boundary — detection logic is pure, file reading is not.

---

## Chunk 1: Platform distro detection

### Task 1: Add Distro enum and `/etc/os-release` parser

**Files:**
- Modify: `rust/src/common/platform.rs`

**Context:** The current `Os` enum has flat variants `MacOs`, `Linux`, `Wsl`. The `Platform` struct derives `Copy`. Adding `Distro` with an `Unknown(String)` variant means `Os` can no longer be `Copy` — the implementer needs to handle this ripple (change to `Clone`, or make `Unknown` use a fixed-size representation). Evaluate the tradeoff.

**Requirements:**
- Add a `Distro` enum with variants: `Debian`, `Fedora`, `Arch`, `Alpine`, `NixOs`, `Unknown(String)`
- Change `Os::Linux` and `Os::Wsl` to carry a `Distro` value
- Implement a parsing function that takes `/etc/os-release` file content as a `&str` and returns a `Distro`. Match on `ID` and `ID_LIKE` fields. Examples of what each distro looks like in `/etc/os-release`:
  - Ubuntu: `ID=ubuntu`, `ID_LIKE=debian`
  - Fedora: `ID=fedora`
  - Rocky Linux: `ID="rocky"`, `ID_LIKE="rhel centos fedora"`
  - Arch: `ID=arch`
  - Manjaro: `ID=manjaro`, `ID_LIKE=arch`
  - Alpine: `ID=alpine`
  - NixOS: `ID=nixos`
- In `Platform::detect()`, read `/etc/os-release` on Linux to populate the distro. macOS has no `/etc/os-release` — handle gracefully
- All existing `match self.os` arms throughout the codebase will need updating due to the new enum shape. **Do not update other files in this task** — just get `platform.rs` compiling with the new types. Other files will be updated in subsequent tasks.

- [ ] **Step 1:** Write tests for the `/etc/os-release` parsing function. Cover: Ubuntu, Fedora, Rocky Linux, Arch, Manjaro, Alpine, NixOS, and an unknown distro. Test that `ID_LIKE` matching works for derivatives.

- [ ] **Step 2:** Run tests to verify they fail (function doesn't exist yet).

- [ ] **Step 3:** Implement the `Distro` enum and parsing function to make the tests pass.

- [ ] **Step 4:** Run tests to verify they pass.

- [ ] **Step 5:** Update `Os` enum to carry `Distro` in `Linux` and `Wsl` variants. Update `Platform::detect()` to read and parse `/etc/os-release` on Linux.

- [ ] **Step 6:** Update all existing helper methods on `Platform` (`is_linux()`, `is_mac()`, `is_wsl()`, `go_os()`, `go_arch()`, `Display` impl) to work with the new `Os` shape. Add new helpers: `distro()` accessor, `is_debian()`, `is_fedora()`, `is_arch()`, `is_alpine()`, `is_nixos()`.

- [ ] **Step 7:** Update existing platform tests to work with the new types.

- [ ] **Step 8:** Run `cargo test` in the `rust/` directory — platform tests pass. The rest of the crate will not compile yet (other files still match on old `Os` variants). That's expected.

- [ ] **Step 9:** Commit.

---

### Task 2: Fix compilation across the codebase for new Os enum

**Files:**
- Modify: `rust/src/common/package_manager.rs` (match arms only — no logic changes yet)
- Modify: `rust/src/install/orchestrator.rs` (if it matches on `Os` directly)
- Modify: All installer files in `rust/src/install/tools/` that reference `platform.os` directly
- Modify: `rust/src/install/mod.rs` (if `Platform` losing `Copy` affects the `Tool` enum or `InstallConfig`)

**Context:** Task 1 changed the shape of `Os`. This task is purely mechanical — update all match arms and type constraints so the crate compiles again. **Do not change any behavior** — just make it compile. Behavior changes come in later tasks.

The key consideration: if `Platform` can no longer be `Copy` (due to `Unknown(String)`), `InstallConfig` and the parallel execution code in `orchestrator.rs` need adjustment. The `ConfigSnapshot` struct and `Arc` sharing pattern may need `Clone` instead of `Copy`.

- [ ] **Step 1:** Run `cargo check` in `rust/` to see all compilation errors.

- [ ] **Step 2:** Fix each file's match arms to handle the new `Os` variants. For now, keep the same behavior: anything that was `Os::Linux` or `Os::Wsl` before should match `Os::Linux(_)` or `Os::Wsl(_)` with a wildcard for the distro.

- [ ] **Step 3:** Fix any `Copy` trait issues caused by `Unknown(String)` in `Distro`. The `Tool` enum, `ALL_TOOLS` const, and `ConfigSnapshot` all depend on `Platform` being `Copy` — evaluate whether to make `Distro` copy-safe (e.g., fixed-size representation) or switch affected code to use `Clone`.

- [ ] **Step 4:** Run `cargo check` — no errors.

- [ ] **Step 5:** Run `cargo test` — all existing tests pass with no behavior changes.

- [ ] **Step 6:** Commit.

---

## Chunk 2: Privilege escalation

### Task 3: Create privilege escalation module

**Files:**
- Create: `rust/src/common/privilege.rs`
- Modify: `rust/src/common/mod.rs` (add `pub mod privilege`)

**Context:** Currently `command::run_sudo()` hardcodes `sudo`. The new module detects the available escalation method at runtime. See spec section 2.

**Requirements:**
- Detect escalation method by checking PATH: `sudo` first, then `doas`, then `su` (preference order)
- If already root (`command::is_root()`), run commands directly without escalation
- A `run_privileged(program, args)` function that picks the right method and executes
- Handle `su -c` correctly — it takes the full command as a single string argument, unlike `sudo`/`doas` which take program + args
- If no method found, error with a message directing user to `bashc install doas`

- [ ] **Step 1:** Create `privilege.rs` with escalation detection and `run_privileged()`.

- [ ] **Step 2:** Add `pub mod privilege` to `common/mod.rs`.

- [ ] **Step 3:** Run `cargo check` — compiles.

- [ ] **Step 4:** Commit.

---

### Task 4: Migrate all `run_sudo` call sites to `privilege::run_privileged`

**Files:**
- Modify: `rust/src/common/package_manager.rs` (5 call sites at lines 117, 133, 146, 158, 159)
- Modify: `rust/src/install/tools/go.rs` (2 call sites at lines 106, 118)
- Modify: `rust/src/install/tools/kubectl.rs` (2 call sites at lines 79, 80)
- Modify: `rust/src/install/tools/obsidian.rs` (1 call site at line 88)
- Modify: `rust/src/install/tools/docker.rs` (2 call sites at lines 81, 92)
- Modify: `rust/src/install/tools/base.rs` (3 call sites at lines 64, 90, 100)
- Modify: `rust/src/common/command.rs` (remove `run_sudo` function)

**Context:** Straightforward search-and-replace. Every `command::run_sudo(prog, args)` becomes `privilege::run_privileged(prog, args)`. Every `if command::is_root() { run_visible(...) } else { command::run_sudo(...) }` pattern simplifies to just `privilege::run_privileged(...)` since the privilege module handles the root check internally.

- [ ] **Step 1:** Replace all `command::run_sudo()` calls with `privilege::run_privileged()`. Simplify the `is_root()`/`run_sudo()` patterns.

- [ ] **Step 2:** Remove `run_sudo` from `command.rs`.

- [ ] **Step 3:** Run `cargo check` — no errors, no references to `run_sudo`.

- [ ] **Step 4:** Run `cargo test` — all tests pass.

- [ ] **Step 5:** Commit.

---

## Chunk 3: Distro-aware package manager routing

### Task 5: Update package_manager.rs with distro-aware routing

**Files:**
- Modify: `rust/src/common/package_manager.rs`

**Context:** The `install()` function currently falls back to `apt_install()` for all Linux. It needs to route based on distro. See spec section 4.

**Requirements:**
- `install()` routes based on distro:
  - macOS → brew (unchanged)
  - Debian, Fedora → brew first, native fallback (apt / dnf)
  - Arch → pacman
  - Alpine → apk
  - NixOS → print declarative guidance, return Ok
  - Unknown → error with distro name and list of supported distros
- Add stub functions for `dnf_install`, `pacman_install`, `apk_install` that return errors like "Arch Linux support not yet implemented. Would install: {package}"
- `ensure_brew()` should skip/no-op on distros where brew is not applicable (Arch, Alpine, NixOS, Unknown)
- `apt_install`, `apt_add_gpg_key`, `apt_add_repo` remain unchanged — they're still used for Debian
- `needs_sudo_for_apt` should be renamed or generalized to reflect that different distros have different sudo needs. NixOS never needs sudo for package operations.

- [ ] **Step 1:** Add stub functions for dnf, pacman, apk, and nix guidance.

- [ ] **Step 2:** Update `install()` to route based on distro. It will need access to the platform's distro — either accept `&Platform` (already does) or the distro directly.

- [ ] **Step 3:** Update `ensure_brew()` to skip on non-applicable distros.

- [ ] **Step 4:** Update or replace `needs_sudo_for_apt` with distro-aware logic.

- [ ] **Step 5:** Run `cargo check` — compiles.

- [ ] **Step 6:** Commit.

---

### Task 6: Guard distro-specific apt calls in installers

**Files:**
- Modify: `rust/src/install/tools/azure.rs`
- Modify: `rust/src/install/tools/docker.rs`
- Modify: `rust/src/install/tools/dotnet.rs`
- Modify: `rust/src/install/tools/eza.rs`
- Modify: `rust/src/install/tools/github.rs`
- Modify: `rust/src/install/tools/terraform.rs`

**Context:** These six installers directly call `apt_add_gpg_key` and `apt_add_repo` in their fallback paths. These operations are Debian-specific. On other distros, the brew path may still work (for Debian/Fedora), but the apt fallback needs a guard. See spec section 5.

**Requirements:**
- The apt fallback code path in each installer should check that the platform is Debian before proceeding
- If not Debian and brew is not available, return a clear error: "third-party repo setup for {tool} not yet supported on {distro}"
- The `needs_sudo()` method on each installer should also be distro-aware — NixOS never needs sudo, and the sudo check should not assume apt
- Consider whether the distro check belongs in each installer or could be centralized in `package_manager` (e.g., a `add_gpg_key` that checks distro first). Either approach is fine — pick whichever keeps the code cleaner.

- [ ] **Step 1:** Update all six installers to guard their apt-specific fallback paths behind a Debian distro check.

- [ ] **Step 2:** Update `needs_sudo()` on each to be distro-aware.

- [ ] **Step 3:** Run `cargo check` — compiles.

- [ ] **Step 4:** Run `cargo test` — all tests pass.

- [ ] **Step 5:** Commit.

---

### Task 7: Update base, obsidian, and neovim installers

**Files:**
- Modify: `rust/src/install/tools/base.rs`
- Modify: `rust/src/install/tools/obsidian.rs`
- Modify: `rust/src/install/tools/neovim.rs`

**Context:** These three have different issues than the six above:
- `base.rs` — the entire Linux package list is Debian-specific (build-essential, nala, libfuse2, etc.). Other distros have different package names or don't need them. Also calls `add-apt-repository universe` which is Debian-only.
- `obsidian.rs` — uses `dpkg -i` / `apt-get install .deb` on Linux, which is Debian-only
- `neovim.rs` — appimage path is distro-agnostic, but the aarch64 apt fallback is Debian-only

**Requirements:**
- `base.rs`: On Debian, existing behavior unchanged. On macOS, unchanged. On other distros, print "base packages not yet configured for {distro}" and return Ok (not an error — don't block the rest of `install all`). Update `needs_sudo()` to be distro-aware.
- `obsidian.rs`: On Debian, existing .deb behavior unchanged. On macOS, unchanged. On other Linux distros, error with "Obsidian .deb install only supported on Debian-based distros. On {distro}, install manually." Update `needs_sudo()`.
- `neovim.rs`: The brew path and appimage path work on any distro. Only the aarch64 apt fallback (`apt_install("neovim")`) needs a Debian guard. On other distros for aarch64 without brew: error with guidance. Update `needs_sudo()`.

- [ ] **Step 1:** Update `base.rs` with distro guards and appropriate messaging.

- [ ] **Step 2:** Update `obsidian.rs` with Debian guard on the .deb install path.

- [ ] **Step 3:** Update `neovim.rs` with Debian guard on the aarch64 apt fallback.

- [ ] **Step 4:** Update `needs_sudo()` on all three.

- [ ] **Step 5:** Run `cargo check` — compiles.

- [ ] **Step 6:** Run `cargo test` — all tests pass.

- [ ] **Step 7:** Commit.

---

## Chunk 4: Doas installer and init.sh

### Task 8: Add doas installer

**Files:**
- Create: `rust/src/install/tools/doas.rs`
- Modify: `rust/src/install/tools/mod.rs` (add `pub mod doas`)
- Modify: `rust/src/install/mod.rs` (add `Doas` variant to `Tool` enum, add to `delegate!` macro arms, add to `ALL_TOOLS`)

**Context:** See spec section 3. This is tool #21 (or #22 counting base). It enables privilege escalation on systems that ship without sudo.

**Requirements:**
- Implements the `Installer` trait
- `name()` returns `"doas"`
- `is_installed()` checks if `doas` is on PATH
- `needs_sudo()` returns `false` — it requires root directly, not sudo (chicken-and-egg). The pre-flight check should not block on needing sudo.
- `install()` requires root — error clearly if not root with guidance to re-run as root
- On Alpine: install via `apk add doas`
- On Debian: install via `apt`
- On other distros: "doas installation not yet supported on {distro}"
- On macOS: "doas is not applicable on macOS (sudo is built-in)"
- After installing, create doas config directory and a config file that permits the `wheel` group to escalate with credential caching
- Print guidance about adding the user to the `wheel` group
- Phase 0 (runs before other tools)
- Register in the `Tool` enum, `delegate!` macro, and `ALL_TOOLS`
- Update `ALL_TOOLS` count in the test

- [ ] **Step 1:** Create `doas.rs` implementing `Installer`.

- [ ] **Step 2:** Register in `tools/mod.rs`, `install/mod.rs` `Tool` enum, `delegate!` macro, and `ALL_TOOLS`.

- [ ] **Step 3:** Update the `all_tools_count` test to reflect the new tool count.

- [ ] **Step 4:** Run `cargo check` — compiles.

- [ ] **Step 5:** Run `cargo test` — all tests pass.

- [ ] **Step 6:** Commit.

---

### Task 9: Add doas pre-flight bootstrap to orchestrator

**Files:**
- Modify: `rust/src/install/orchestrator.rs`

**Context:** During `run_all`, if running as root and no escalation method is available, auto-install doas before proceeding. This is the Rust-side bootstrap — `init.sh` handles the shell-side (Task 10).

**Requirements:**
- In the pre-flight section of `run_all()`, after checking sudo requirements but before running any phases:
  - If running as root and no escalation method is detected (no sudo, doas, or su on PATH)
  - And the platform is Alpine
  - Auto-invoke the doas installer
- This is a best-effort bootstrap. If it fails, print a warning and continue — individual tools that need escalation will fail with clear messages
- For non-`all` runs (`run_by_name`), do not auto-bootstrap — let the user handle it explicitly

- [ ] **Step 1:** Add the pre-flight doas bootstrap logic to `run_all()`.

- [ ] **Step 2:** Run `cargo check` — compiles.

- [ ] **Step 3:** Run `cargo test` — all tests pass.

- [ ] **Step 4:** Commit.

---

### Task 10: Update init.sh with distro detection and doas bootstrap

**Files:**
- Modify: `init.sh`

**Context:** See spec section 8. The shell script needs to handle Alpine's lack of sudo/doas before downloading and running bashc.

**Requirements:**
- Add a `detect_distro()` function that reads `/etc/os-release` and matches `ID` and `ID_LIKE` fields, same approach as the Rust parser. Returns a distro family string (debian, fedora, arch, alpine, nixos, unknown). On macOS (no `/etc/os-release`), return "macos" or skip.
- After platform detection, before downloading bashc:
  - If on Alpine, running as root, and neither `sudo` nor `doas` nor `su` is available:
  - Install doas via `apk add doas`
  - Create `/etc/doas.d/doas.conf` with `permit persist :wheel`
  - Print what was done
- On non-Alpine distros: no changes to existing flow
- Keep the script POSIX-compatible (sh, not bash)
- Run `shellcheck init.sh` and fix any warnings

- [ ] **Step 1:** Add `detect_distro()` function to init.sh.

- [ ] **Step 2:** Add the Alpine doas bootstrap block after platform detection.

- [ ] **Step 3:** Run `shellcheck init.sh` — no warnings.

- [ ] **Step 4:** Commit.

---

## Chunk 5: Final verification

### Task 11: Full build and dry-run verification

- [ ] **Step 1:** Run `cargo build` in `rust/` — no errors, no warnings (fix any warnings from earlier tasks).

- [ ] **Step 2:** Run `cargo test` in `rust/` — all tests pass.

- [ ] **Step 3:** Run `cargo run -- install --dry-run all` — all tools listed, no crashes. Verify the output includes the doas tool.

- [ ] **Step 4:** Run `cargo run -- install doas --dry-run` — shows what would be done.

- [ ] **Step 5:** Run `shellcheck init.sh` — clean.

- [ ] **Step 6:** Commit any final fixes.
