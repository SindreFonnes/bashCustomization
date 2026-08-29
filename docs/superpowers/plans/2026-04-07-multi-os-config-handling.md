# Multi-OS Config Handling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Plan style note:** Per `CLAUDE.md`, this plan is requirements-driven. Interface contracts and test scenarios are specified, but implementation bodies are left to the implementer based on existing codebase patterns. Read the spec at `docs/superpowers/specs/2026-04-07-multi-os-config-handling-design.md` before starting any task.

**Goal:** Add a `bashc configs check` subcommand that auto-links safe drift on shell startup, warns about unresolved drift, and prunes stale `local/managed_configs.toml` entries with cross-platform safety. Hook it into shell init via `general_functions.sh` + `main.sh`. Enable per-OS config files via the existing manifest `platform` filter (no new code; documented as a convention).

**Architecture:** A new `configs::check` Rust module reuses the existing `manifest`, `state`, and `link` helpers. The check is non-interactive, exits 0 in all non-fatal cases, and produces at most two terse output lines (auto-linked summary + drift warning). A new shell function in `general_functions.sh` invokes the binary on interactive shell startup with environment-variable opt-out and graceful no-op when the binary is missing.

**Tech Stack:** Rust (existing crates: `clap`, `anyhow`, `toml`, `serde`, `dialoguer`, `tempfile` for tests). Bash/zsh-compatible shell scripting (POSIX where possible).

---

## File Structure

**Create:**
- `rust/src/configs/check.rs` — `bashc configs check` command implementation + tests
- `configs/README.md` — convention + check command documentation

**Modify:**
- `rust/src/configs/manifest.rs` — add unfiltered manifest loader
- `rust/src/configs/state.rs` — add stale-entry prune function
- `rust/src/configs/link.rs` — promote `create_symlink` to `pub(crate)` so `check.rs` can call it
- `rust/src/configs/mod.rs` — register the new `check` module
- `rust/src/main.rs` — add `Check` variant to `ConfigsAction` and wire it up
- `general_functions.sh` — add `bashc_check_configs` function
- `main.sh` — call `bashc_check_configs` after `load_shell_extentionfiles`
- `CLAUDE.md` — add pointer to `configs/README.md`

**Decomposition rationale:** five tasks, each producing one logical commit. Tasks 1–2 are pure additions to existing files (small surface area, low risk of regressions). Task 3 is the main feature work and depends on 1+2. Task 4 is the shell glue (no Rust). Task 5 is docs.

---

## Task 1: Add `load_manifest_unfiltered` helper

**Why:** The cleanup pass needs to know "what targets does the manifest reference *across all platforms*" so it does not delete a self-managed marker for a macOS-only entry when running on Linux. The existing `load_manifest` filters by platform and is unsuitable for this.

**Files:**
- Modify: `rust/src/configs/manifest.rs`

**Interface contract:**
```rust
/// Load the manifest from `<project_root>/configs/manifest.toml` without
/// applying any platform filter. Used by cross-platform safety checks
/// (e.g., self-managed marker cleanup) that must reason about all entries
/// regardless of the current OS.
pub fn load_manifest_unfiltered(project_root: &Path) -> Result<Vec<ConfigEntry>>;
```

**Implementation notes:**
- Reuse the existing `load_manifest_from_str` plumbing. The cleanest path is to introduce a small `PlatformFilter` enum (e.g., `Current(&Platform)` / `All`) passed to the inner parser, or to add a sibling private function `load_manifest_from_str_unfiltered`. Implementer's call.
- Tilde expansion in target paths must still happen — pass the resolved `home_dir()` through, same as `load_manifest`.
- Source resolution (joining `project_root/configs/<source>`) is identical.

**Steps:**

- [ ] **Step 1: Write failing tests in `rust/src/configs/manifest.rs::tests`**

  Add three tests covering:

  1. `load_unfiltered_returns_all_entries_regardless_of_platform` — TOML with one `macos` entry, one `linux` entry, one unfiltered entry. Assert the returned `Vec` has 3 entries.
  2. `load_unfiltered_still_expands_tilde` — entry with `target = "~/.foo"`. Assert the resolved target uses the supplied `home`.
  3. `load_unfiltered_still_resolves_sources_to_configs_dir` — entry with `source = "claude/CLAUDE.md"` and a fake project root. Assert the resolved source ends with `<root>/configs/claude/CLAUDE.md`.

  These tests should be unit-style and not call the public `load_manifest_unfiltered` directly if doing so would require touching `$HOME`. Instead, factor an internal helper that takes `home: &str` as a parameter (mirroring how `load_manifest_from_str` is structured today) and test that.

- [ ] **Step 2: Run the new tests**

  ```
  cd rust && cargo test configs::manifest::tests::load_unfiltered
  ```
  Expected: all three FAIL with "function not found" or similar.

- [ ] **Step 3: Implement `load_manifest_unfiltered` and any needed inner helper**

  - Add the public function with the signature above.
  - Whatever inner helper you add must keep the existing `load_manifest_from_str` working unchanged (or be a thin wrapper around a shared inner function).
  - Do not duplicate the per-entry parsing/source-resolution/tilde-expansion logic — refactor to share.

- [ ] **Step 4: Run all manifest tests + the full configs suite**

  ```
  cd rust && cargo test configs::manifest && cargo test configs
  ```
  Expected: all PASS, including the new tests and the existing `load_manifest_from_str`-based tests.

- [ ] **Step 5: Run clippy on the touched file**

  ```
  cd rust && cargo clippy --all-targets 2>&1 | grep -A2 "src/configs/manifest"
  ```
  Expected: no new warnings.

- [ ] **Step 6: Commit**

  ```
  git add rust/src/configs/manifest.rs
  git commit -m "feat(configs): add load_manifest_unfiltered for cross-platform reachability checks"
  ```

---

## Task 2: Add `prune_stale_self_managed` helper

**Why:** Section 4 of the spec — clean up `local/managed_configs.toml` entries that are no longer reachable. Two conditions for staleness, expressed as the OR of:

1. The marker's target is **not in the unfiltered manifest** (entry has been removed on every platform).
2. The marker's target **is in the *current platform's* filtered manifest** AND the target file does not exist on disk (user deleted it locally).

Cross-platform safety follows: a marker for an entry that is in the unfiltered manifest but **not** in the current filtered manifest is preserved regardless of whether the target file exists locally — because we have no business deciding whether files for other platforms "should" exist on this machine.

**Files:**
- Modify: `rust/src/configs/state.rs`

**Interface contract:**
```rust
/// Prune entries from `local/managed_configs.toml` that are no longer
/// reachable. An entry is stale when **either**:
///   1. its `target` is NOT in `all_platform_targets` (entry no longer in
///      the manifest at all), OR
///   2. its `target` IS in `current_platform_targets` AND the file at
///      that target does not exist on disk.
///
/// Cross-platform safety: a marker whose target appears in
/// `all_platform_targets` but NOT in `current_platform_targets` is
/// preserved unconditionally (it belongs to a different OS's view of the
/// manifest, and we make no judgement about whether the file should
/// exist on this machine).
///
/// `all_platform_targets` MUST come from `load_manifest_unfiltered`.
/// `current_platform_targets` MUST come from `load_manifest` for the
/// current platform.
///
/// Returns the number of entries removed (for testing/observability).
/// Silent — does not print anything.
pub(crate) fn prune_stale_self_managed(
    project_root: &Path,
    current_platform_targets: &[String],
    all_platform_targets: &[String],
) -> Result<usize>;
```

**Implementation notes:**
- Reuse `load_self_managed` to read the file and `remove_self_managed` (which already handles the empty-file deletion).
- Compare targets as strings (consistent with how `is_self_managed` does it).
- Iterate over a snapshot of the loaded entries — do not mutate while iterating.
- It is acceptable to call `remove_self_managed` once per stale entry, or to write the pruned file once at the end. Implementer's call. If you choose batch-write, factor out the file-writing logic so `add_self_managed` and `remove_self_managed` can share it.
- The function does NOT load the manifest itself; the caller is responsible for providing both target slices.

**Steps:**

- [ ] **Step 1: Write failing tests in `rust/src/configs/state.rs::tests`**

  Add five tests, one per row of the staleness table in spec section 4:

  1. `prune_removes_entry_with_missing_target_when_in_current_filtered_manifest` — set up: marker for target T, T does NOT exist on disk, T is in `current_platform_targets`, T is in `all_platform_targets`. Expect: returns `1`, marker removed (condition 2).
  2. `prune_removes_entry_not_in_unfiltered_manifest` — set up: marker for target T, T exists on disk, T is NOT in `all_platform_targets`, T is NOT in `current_platform_targets`. Expect: returns `1`, marker removed (condition 1).
  3. `prune_preserves_marker_for_other_platform_entry_with_missing_file` — **critical cross-platform safety test**. Set up: marker for target T, T does NOT exist on disk (simulating "this is a macOS-only entry checked from Linux, the macOS path doesn't exist on this Linux box"), T is NOT in `current_platform_targets`, T IS in `all_platform_targets`. Expect: returns `0`, marker preserved.
  4. `prune_preserves_marker_for_other_platform_entry_with_existing_file` — set up: marker for target T, T exists on disk, T is NOT in `current_platform_targets`, T IS in `all_platform_targets`. Expect: returns `0`, marker preserved.
  5. `prune_preserves_marker_when_in_current_manifest_and_file_exists` — baseline happy path. Set up: marker for target T, T exists on disk, T IS in both target slices. Expect: returns `0`, marker preserved.

  Plus one administrative test:

  6. `prune_returns_zero_when_no_markers` — empty self-managed list. Expect: returns `0`, no error, no file written.

- [ ] **Step 2: Run the new tests**

  ```
  cd rust && cargo test configs::state::tests::prune
  ```
  Expected: all FAIL with "function not found".

- [ ] **Step 3: Implement `prune_stale_self_managed`**

  Add the function with the contract above. Keep it private to the crate.

- [ ] **Step 4: Run all state + configs tests**

  ```
  cd rust && cargo test configs::state && cargo test configs
  ```
  Expected: all PASS, no regressions in existing self-managed tests.

- [ ] **Step 5: Run clippy on the touched file**

  ```
  cd rust && cargo clippy --all-targets 2>&1 | grep -A2 "src/configs/state"
  ```
  Expected: no new warnings.

- [ ] **Step 6: Commit**

  ```
  git add rust/src/configs/state.rs
  git commit -m "feat(configs): add prune_stale_self_managed helper"
  ```

---

## Task 3: Implement `bashc configs check` subcommand

**Why:** Spec section 2 — the main feature. Auto-link safe `NotLinked` cases, summarize what was done, warn about unresolved drift, prune stale self-managed entries.

**Files:**
- Create: `rust/src/configs/check.rs`
- Modify: `rust/src/configs/mod.rs` (register new module)
- Modify: `rust/src/configs/link.rs` (visibility change for `create_symlink`)
- Modify: `rust/src/main.rs` (add `Check` variant and wire it up)

**Interface contract:**
```rust
// in rust/src/configs/check.rs
pub fn run_check(project_root: &Path, platform: &Platform) -> Result<()>;

// also exposed for testing — same write-to-buffer pattern as link.rs/status.rs.
// Takes both the filtered (current platform) and unfiltered (all platforms)
// entry slices so that the self-managed cleanup pass can apply the
// cross-platform-safe staleness rule from spec section 4.
fn write_check(
    writer: &mut impl Write,
    filtered_entries: &[ConfigEntry],
    unfiltered_entries: &[ConfigEntry],
    self_managed: &[SelfManagedEntry],
    project_root: &Path,
) -> Result<()>;
```

The `Check` variant in `ConfigsAction` takes no arguments (no name filter, no flags). Future flags can be added later.

**Behavior requirements (from spec section 2):**
- Loads filtered manifest (current platform) via `load_manifest`.
- Loads self-managed list via `load_self_managed`.
- For each entry, computes `EntryState` via `detect_state`.
- Auto-links any entry where `state == NotLinked` AND `target.symlink_metadata().is_err()` (target truly absent — not even a broken symlink). Reuses `create_symlink` from `link.rs`.
- Collects entries in `Conflict` or `WrongSymlink` for the warning.
- Skips `Linked` and `SelfManaged` entries silently.
- After processing, calls `prune_stale_self_managed`. Builds `current_platform_targets` from the already-loaded filtered manifest and `all_platform_targets` from `load_manifest_unfiltered(project_root)?`. Both are `Vec<String>` of `entry.target.to_string_lossy().to_string()`.
- Output rules:
  - Silent (no output) if 0 auto-linked AND 0 unresolved drift.
  - One line `bashc: linked N configs (name1, name2)` if anything was auto-linked. Names are de-duplicated and sorted (use the sort+dedup pattern already used in `link.rs`/`status.rs` after PR #4 fixes).
  - One line `bashc: ⚠ N configs need attention (name1: conflict, name2: wrong symlink) — run 'bashc configs status'` if any unresolved drift remains. Per-entry tag is `conflict` for `Conflict` and `wrong symlink` for `WrongSymlink`.
  - Both lines may be printed in the same run.
- Always returns `Ok(())` for the in-band cases above. Hard errors (manifest unreadable, project root missing) bubble up via `?` as the command's `Result<()>` and cause a non-zero exit — that is the only way the command fails.

**Visibility change in link.rs:** make `create_symlink` `pub(crate)` and add a one-line doc comment. Existing tests still cover it.

**Wire-up in main.rs:**
- Add `Check {}` variant to `ConfigsAction` with a docstring `/// Auto-link safe drift and warn about anything that needs attention. Designed for shell-startup invocation.`
- In the `match` block in `main()`, handle `ConfigsAction::Check {}` by calling `configs::check::run_check(&project_root, &platform)?`.

**Steps:**

- [ ] **Step 1: Write failing tests in `rust/src/configs/check.rs`**

  Create the file with `mod tests` containing the following test cases. Use `tempdir()` and the same `make_entry` helper pattern as `link.rs::tests`. Each test calls `write_check` directly (the writer-injection pattern) so output can be captured.

  1. `check_silent_when_all_linked` — one entry, target is a symlink to source. Capture output. Assert: empty buffer, target still a symlink.
  2. `check_silent_when_self_managed` — one entry, target is a regular file, marker present in self-managed list. Assert: empty buffer, target untouched.
  3. `check_auto_links_not_linked_when_target_absent` — one entry, target does not exist. Assert: target becomes a symlink to source, output contains `linked 1 configs` and the entry name.
  4. `check_auto_links_multiple_entries_and_dedups_names_in_summary` — three entries with two distinct group names (e.g., two `claude` + one `zellij`), all `NotLinked`. Assert: all three become symlinks, output contains `linked 3 configs (claude, zellij)` (alphabetized, deduped names).
  5. `check_warns_on_conflict_without_modifying_file` — one entry, target is a regular file with content. Assert: target unchanged (still a file with same content), output contains `⚠ 1 configs need attention` and `conflict`.
  6. `check_warns_on_wrong_symlink_without_modifying` — one entry, target is a symlink to a different file. Assert: symlink unchanged, output contains `wrong symlink`.
  7. `check_mixed_auto_links_safe_and_warns_unsafe` — two entries: one `NotLinked` (target absent) and one `Conflict` (target is a regular file). Assert: first becomes a symlink, second is untouched, output contains BOTH lines.
  8. `check_prunes_marker_when_entry_no_longer_in_manifest` — set up: a self-managed marker whose target is NOT in the (filtered or unfiltered) entry list passed to `write_check`. Assert: after `write_check`, the marker is removed from `local/managed_configs.toml`, output mentions nothing about pruning.
  9. `check_prunes_marker_when_target_missing_and_in_current_filtered_manifest` — set up: a self-managed marker whose target file does NOT exist on disk, where the manifest entry IS in the current platform's filtered slice. Assert: marker removed.
  10. `check_preserves_marker_for_other_platform_entry` — **critical cross-platform safety test**. Set up: an entry exists in the unfiltered superset but NOT in the current filtered slice; the marker exists; the target file does not exist on disk. Assert: marker preserved.
  11. `check_returns_ok_even_with_unresolved_drift` — same as test 5 but assert the function's return value is `Ok(())`, not `Err`.

  Note: because `write_check` operates on entries directly, tests 8–10 need to inject both the "filtered" and "unfiltered" entry slices. Adjust the `write_check` signature to take both — e.g.:

  ```rust
  fn write_check(
      writer: &mut impl Write,
      filtered_entries: &[ConfigEntry],
      unfiltered_entries: &[ConfigEntry],
      self_managed: &[SelfManagedEntry],
      project_root: &Path,
  ) -> Result<()>;
  ```

  The public `run_check` derives both internally (`load_manifest` for filtered, `load_manifest_unfiltered` for unfiltered).

- [ ] **Step 2: Run the new tests**

  ```
  cd rust && cargo test configs::check
  ```
  Expected: ALL FAIL — module does not yet compile (no `check.rs`). First fix the compilation error by creating an empty stub, then re-run; tests should fail with "function not found" or assertion failures.

- [ ] **Step 3: Promote `create_symlink` in link.rs**

  Change `fn create_symlink` to `pub(crate) fn create_symlink`. Add a one-line doc comment explaining it is shared with the `check` module. Run `cargo test configs::link` to confirm no regressions.

- [ ] **Step 4: Register the new module**

  Add `pub(crate) mod check;` to `rust/src/configs/mod.rs`.

- [ ] **Step 5: Implement `write_check` and `run_check` in `check.rs`**

  Follow the behavior requirements above. Reuse:
  - `crate::configs::manifest::load_manifest`, `load_manifest_unfiltered`
  - `crate::configs::state::{detect_state, load_self_managed, prune_stale_self_managed}`
  - `crate::configs::link::create_symlink`
  - `crate::configs::home_dir`
  - `crate::configs::{ConfigEntry, EntryState}`

  The summary line construction should follow the pattern already established for sorted-and-deduped name lists (sort, dedup, join with `, `). Use Unicode `⚠` for the warning prefix to match the existing `✓`/`✗`/`○` characters used by `status`.

- [ ] **Step 6: Wire up the CLI variant in `main.rs`**

  Add `Check {}` to `ConfigsAction` and call `configs::check::run_check(&project_root, &platform)?` in the match arm.

- [ ] **Step 7: Run all tests**

  ```
  cd rust && cargo test
  ```
  Expected: 181 + (number of new tests in this task) PASS, 0 FAIL.

- [ ] **Step 8: Run clippy**

  ```
  cd rust && cargo clippy --all-targets 2>&1 | grep -B1 -A3 "src/configs"
  ```
  Expected: no new warnings in any `src/configs/*.rs` file.

- [ ] **Step 9: Manual smoke test**

  Build and run the actual binary against the live project root:
  ```
  cd rust && cargo build && ./target/debug/bashc configs check
  ```
  Expected: either silent (everything in sync), or one of the documented one-line outputs. Should NOT prompt for anything. Should NOT panic. Exit code should be 0 unless there is a hard error.

- [ ] **Step 10: Commit**

  ```
  git add rust/src/configs/check.rs rust/src/configs/mod.rs rust/src/configs/link.rs rust/src/main.rs
  git commit -m "feat(configs): add 'bashc configs check' subcommand for shell-startup auto-link"
  ```

---

## Task 4: Shell integration

**Why:** Spec section 3 — call `bashc configs check` from interactive shell startup with safety guards (interactive-only, env-var opt-out, no-op when binary missing).

**Files:**
- Modify: `general_functions.sh` — add `bashc_check_configs` function
- Modify: `main.sh` — invoke the function after `load_shell_extentionfiles` and after `check_for_shell_update_once_a_day`

**Function contract:** the spec section 3 lists the five required behaviors. To restate them as a checklist the implementer can verify:

1. Skip if shell is non-interactive. Detect via `case $- in *i*) ;; *) return 0 ;; esac` (POSIX-compatible; works in both bash and zsh).
2. Skip if `BASHC_SKIP_CONFIG_CHECK` is non-empty: `[ -n "${BASHC_SKIP_CONFIG_CHECK:-}" ] && return 0`.
3. Skip if `bashc` not on `PATH`: `command -v bashc >/dev/null 2>&1 || return 0`.
4. Otherwise invoke `bashc configs check`. Output goes to the user's terminal (do not redirect).
5. Swallow non-zero exits so the shell init never fails. The simplest pattern: append `|| true` to the invocation, or wrap the call in a subshell that always returns success.

**Constraints:**
- Must be POSIX-compatible enough to work in both bash and zsh. No bashisms. Use `[` not `[[`. Use `case` not `[[ ... =~ ]]`. Use `command -v` not `which`.
- Function name follows the project's `snake_case` convention.
- Do NOT load this from a `programExtensions/bashc/` directory — keeping it in `general_functions.sh` matches its core-utility nature and avoids the extra scaffolding the per-program-extension pattern requires.
- The call site in `main.sh` goes AFTER `check_for_shell_update_once_a_day` so the check sees the latest extensions.

**Steps:**

- [ ] **Step 1: Add `bashc_check_configs` to `general_functions.sh`**

  Append the function at the end of the file (after the existing helpers). It should be ~10 lines and follow the contract above. Keep all syntax POSIX-compatible.

- [ ] **Step 2: Run shellcheck on the modified file**

  ```
  shellcheck general_functions.sh
  ```
  Expected: no warnings or errors. Fix any that appear.

- [ ] **Step 3: Add the call site in `main.sh`**

  After the existing `check_for_shell_update_once_a_day` line at the bottom of `main.sh`, add a call to `bashc_check_configs`. Since `main.sh` already sources `general_functions.sh` at the top, the function will be defined.

- [ ] **Step 4: Run shellcheck on main.sh**

  ```
  shellcheck main.sh
  ```
  Expected: no new warnings. (`main.sh` may have pre-existing warnings unrelated to this change — ignore those, just confirm nothing new.)

- [ ] **Step 5: Manual verification — fresh shell sourcing**

  In a new terminal:
  ```
  source ~/bashCustomization/main.sh
  ```
  Expected: shell startup completes without errors, and either no extra output (all configs in sync) or one of the documented one-line outputs from `bashc configs check`. Try once with `BASHC_SKIP_CONFIG_CHECK=1` set — verify the check is skipped.

- [ ] **Step 6: Manual verification — non-interactive invocation**

  ```
  bash -c '. ~/bashCustomization/main.sh; echo done'
  ```
  Expected: completes without invoking `bashc configs check` (because the shell is non-interactive). The `done` line should print.

- [ ] **Step 7: Manual verification — missing binary safety**

  Temporarily rename or `PATH`-mask the `bashc` binary, then source `main.sh` in a fresh shell. Expected: no error, no output, shell sources successfully. Restore the binary afterwards.

- [ ] **Step 8: Commit**

  ```
  git add general_functions.sh main.sh
  git commit -m "feat(shell): run 'bashc configs check' on interactive shell startup"
  ```

---

## Task 5: Documentation

**Why:** Spec section 5 — make the per-platform-file convention and the `check` command discoverable without grepping.

**Files:**
- Create: `configs/README.md`
- Modify: `CLAUDE.md` (project root) — add a pointer

**`configs/README.md` content requirements:**

The README should cover, in this order:

1. **Purpose** — one paragraph on what `configs/` is and how `bashc configs` manages it.
2. **Per-platform variants** — explain the convention `<tool>/config.<platform>.<ext>` and show the zellij example using two `[[config]]` entries with `platform = "macos"` / `platform = "linux"` both pointing to the same target. Point out the trade-off: duplication vs. simplicity, and when to use it (only when files genuinely differ per OS — for trivial differences, prefer cross-platform defaults like OSC52 in the zellij case that this very feature was triggered by).
3. **What `bashc configs check` does** — describe the behavior briefly: auto-links safe `NotLinked` entries, warns about `Conflict` / `WrongSymlink`, prunes stale `local/managed_configs.toml` entries, exits 0. Reference `bashc configs status` as the read-only inspection command.
4. **When `bashc configs check` runs** — on every interactive shell startup, via `bashc_check_configs` in `general_functions.sh`. Skipped for non-interactive shells.
5. **Opting out** — set `BASHC_SKIP_CONFIG_CHECK=1` in the environment (e.g., for CI runners or noisy subshells).
6. **Auto-pruning of `local/managed_configs.toml`** — short note that the check command silently removes self-managed markers for entries whose target is gone or no longer in the manifest, with cross-platform safety (a macOS-only entry's marker is preserved when checked from Linux because the unfiltered manifest is consulted).
7. **Adding a new config** — quick how-to: create the directory under `configs/`, add an entry to `manifest.toml`, run `bashc configs link <name>`. Mention the `platform` field as optional.

Keep it under 100 lines. Link out to the spec at `docs/superpowers/specs/2026-04-07-multi-os-config-handling-design.md` as the design rationale source for anyone who wants the deeper context.

**`CLAUDE.md` change:** under the existing "Project structure" section, add a one-line entry pointing future readers (and agents) at `configs/README.md`. Suggested wording: `- `configs/` — version-controlled config files managed by `bashc configs`. See `configs/README.md` for the per-platform-file convention and shell-startup behavior.`

**Steps:**

- [ ] **Step 1: Write `configs/README.md`**

  Follow the section order above. Use plain Markdown — no tables unless they actually clarify (none needed here).

- [ ] **Step 2: Spot-check the doc against the spec**

  Re-read `docs/superpowers/specs/2026-04-07-multi-os-config-handling-design.md` and confirm the README does not contradict it. Any discrepancy should be fixed in the README (the spec is the source of truth).

- [ ] **Step 3: Update `CLAUDE.md`**

  Add the one-line pointer entry to the "Project structure" section.

- [ ] **Step 4: Commit**

  ```
  git add configs/README.md CLAUDE.md
  git commit -m "docs(configs): add README for configs directory and check command"
  ```

---

## Self-review

**Spec coverage:**
- Spec section 1 (per-platform sources, no new code) → Task 5 documentation only. ✓
- Spec section 2 (`bashc configs check` command) → Task 3. ✓
- Spec section 3 (shell integration) → Task 4. ✓
- Spec section 4 (stale `managed_configs.toml` cleanup with cross-platform safety) → Tasks 1 + 2 + 3 (the `load_manifest_unfiltered` helper, the `prune_stale_self_managed` helper, and the call from `run_check`). ✓
- Spec section 5 (documentation) → Task 5. ✓
- Testing requirements (all bullets) → covered by the tests listed in Tasks 1, 2, and 3. The cross-platform safety test specifically appears in Task 2 step 1 test 3 (revised) AND Task 3 step 1 test 9.
- Architecture notes (file location, helper reuse) → followed in Task 3.

**Placeholder scan:**
- No "TBD" / "TODO" / "implement later".
- Each step has a concrete action and a way to verify it.
- Test scenarios describe arrange/act/assert clearly.
- Function signatures are explicit.
- Implementation bodies are deliberately not included per the project's CLAUDE.md guidance — the implementer reads existing patterns and writes the code.

**Type / signature consistency:**
- `load_manifest_unfiltered(project_root: &Path) -> Result<Vec<ConfigEntry>>` — Task 1. Used in Task 3 step 5. ✓
- `prune_stale_self_managed(project_root: &Path, current_platform_targets: &[String], all_platform_targets: &[String]) -> Result<usize>` — Task 2. Called in Task 3 from `run_check` and `write_check` after constructing both target vecs from the filtered and unfiltered manifests. ✓
- `create_symlink` visibility change — Task 3 step 3. Used in Task 3 step 5. ✓
- `run_check(project_root: &Path, platform: &Platform) -> Result<()>` — Task 3 interface. Called from `main.rs` in Task 3 step 6. ✓
- `write_check(writer, filtered_entries, unfiltered_entries, self_managed, project_root)` — Task 3 interface. The dual entry-slice parameters are required by the staleness rule in spec section 4. ✓
- `bashc_check_configs` shell function — Task 4. Called from `main.sh` in Task 4 step 3. ✓

**Scope check:**
- Single coherent feature: drift detection at shell startup with multi-OS support documented as a convention. Five tasks, five commits, all on the same branch. Aligns with the user's preference to bundle into PR #4.

No issues found.
