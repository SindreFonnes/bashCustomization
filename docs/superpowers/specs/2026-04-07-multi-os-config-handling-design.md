# Multi-OS Config Handling — Design

**Date:** 2026-04-07
**Status:** Draft

## Purpose

Allow `bashc configs` to handle config files that need to differ between operating systems, and surface drift between repo and local state at shell startup so the user notices when configs need attention.

## Background

The `bashc configs` command introduced in PR #4 manages symlinks between version-controlled config files in the repo and their system locations. The mechanism works well when a single source file is appropriate for every platform, but it breaks down when the same config needs to differ per OS — for example, zellij's `copy_command` should be `pbcopy` on macOS and is unsupported on Linux/WSL where OSC52 should be used instead.

In addition, drift between the repo and the local file system today is invisible unless the user runs `bashc configs status` manually. This means new manifest entries, deleted symlinks, and tools that overwrite their own config files all go unnoticed until something breaks.

## Goals

1. Support per-OS variations of config files without losing the symlink-based workflow.
2. Notify the user at shell startup when configs are out of sync, with safe automatic remediation where possible.
3. Avoid adding new authoring complexity (no templating engine, no custom DSL).
4. Keep shell startup latency negligible.

## Non-goals

- Templating, conditional rendering, or generated files. Sources remain plain files that are symlinked verbatim.
- Bidirectional sync. Symlinks already give us "edit live → repo updates" for free.
- Automatic resolution of conflicts where a real file (not a symlink) exists at the target. Those require user judgement.
- Cross-machine state sync. `local/managed_configs.toml` remains machine-local.

## Design

### 1. Per-platform source files (no new code)

Configs that need OS variation are split into multiple source files, one per platform, with one manifest entry per file. The existing `platform` filter in `manifest.toml` already handles loading only the appropriate entry.

**File-naming convention:** `<tool>/config.<platform>.<ext>`. For example:

- `configs/zellij/config.macos.kdl`
- `configs/zellij/config.linux.kdl`

Both entries in `manifest.toml` point to the same target path, but only one is loaded per machine because of the `platform` field. The convention is documented but not enforced — the manifest accepts any source path.

**Trade-off accepted:** when most of the file is shared, this duplicates content. The user explicitly accepted this in exchange for keeping the simple symlink model and not introducing a templating engine. Configs are not expected to change frequently.

### 2. New `bashc configs check` command

A new subcommand purpose-built for invocation during shell startup. Distinct from `link` because the defaults, output, and exit-code policy differ.

**Required behavior:**

- Loads the manifest filtered by the current platform.
- Computes `EntryState` for each entry using the existing `detect_state` helper.
- For entries in state `NotLinked` where the target path does not exist on disk: create the symlink (safe — no existing data to clobber). Increment a "linked" counter.
- For entries in state `Conflict` (a real file exists at target) or `WrongSymlink` (symlink points elsewhere): record the entry for the warning summary. Do **not** modify the file.
- For entries in state `Linked` or `SelfManaged`: take no action.
- After processing entries, also clean up stale `local/managed_configs.toml` entries — see section 4.

**Required output:**

- Nothing happened and nothing is wrong → silent (no output).
- One or more entries were auto-linked → a single line listing the count and the affected names. Example: `bashc: linked 2 configs (claude, zellij)`.
- One or more entries are in `Conflict` / `WrongSymlink` → a single warning line listing the count and the affected names, ending with a hint to run `bashc configs status` for details. Example: `bashc: ⚠ 2 configs need attention (zellij: conflict, claude: wrong symlink) — run 'bashc configs status'`.
- Both auto-linked and unresolved drift can occur in the same run. Both lines may be printed.

**Exit code policy:** the command exits 0 in all non-fatal cases, including when unresolved drift exists. This is a deliberate choice so that a transient warning during shell startup never breaks the shell. Hard errors (manifest unreadable, project root missing) propagate as non-zero exits — but the shell-side wrapper must tolerate this too.

**Non-interactive by default:** `check` never prompts. It is safe to invoke from any startup context.

**Why not add a flag to `link`?** `link` already has interactive prompting, per-entry verbose output, and force-strategy semantics. `check` has different defaults across all three. Reusing the existing core helpers (`detect_state`, `create_symlink`, the manifest loader) avoids logic duplication while keeping the command interfaces clean.

### 3. Shell integration

A new function added to `general_functions.sh` (or a dedicated file under `programExtensions/bashc/`, whichever is more consistent with the loading order — to be chosen during implementation). The function is invoked from the existing shell initialization flow after modules are loaded.

**Required behavior of the shell function:**

1. Returns immediately if the shell is not interactive (e.g., `[[ $- == *i* ]]` or POSIX equivalent — must work for both bash and zsh).
2. Returns immediately if `BASHC_SKIP_CONFIG_CHECK` is set to a non-empty value.
3. Returns immediately if the `bashc` binary is not on `PATH` (so the framework still works on machines where the Rust binary has not been built yet).
4. Otherwise, invokes `bashc configs check` and forwards its output to the user's terminal.
5. Never causes the shell to exit on failure — any non-zero exit from the binary is swallowed.

**Constraints:**

- Must be POSIX-compatible enough to work in both bash and zsh sessions, since both are supported.
- Must not block the shell on slow operations. If the check takes more than ~100 ms in normal operation, that becomes a bug to investigate (likely cache or skip).

### 4. Stale `local/managed_configs.toml` cleanup

`bashc configs check` also prunes stale entries from `local/managed_configs.toml`. An entry is "stale" when:

- The target path no longer exists on disk, **or**
- The target path is no longer referenced by any entry in the manifest **across all platforms** (not just the current platform's filtered view).

The "across all platforms" qualification matters: an entry that is marked self-managed on macOS for a macOS-only target must not be deleted when the check runs on a Linux machine, and vice versa. The cleanup pass therefore loads the manifest unfiltered when computing reachability.

Stale entries are removed silently (no output). Cleanup runs after the link/notify pass so it does not interfere with state detection.

This keeps the self-managed list from accumulating cruft when the user removes a manifest entry or deletes a target file.

### 5. Documentation

- A new `configs/README.md` documenting:
  - The per-platform-file convention with the zellij example
  - What `bashc configs check` does and when it runs
  - The `BASHC_SKIP_CONFIG_CHECK` opt-out
  - That `local/managed_configs.toml` is auto-pruned
- A short pointer in the project `CLAUDE.md` directing future agents (and the user) to `configs/README.md` so the convention is discoverable without grepping.
- The PR description for the implementing PR mentions the new subcommand alongside the existing `link|unlink|status|diff` list.

## Architecture notes

- `bashc configs check` lives in `rust/src/configs/check.rs`, exposed via a new `ConfigsAction::Check` variant in `rust/src/main.rs`.
- The check reuses existing helpers: `manifest::load_manifest`, `state::detect_state`, `state::load_self_managed`, `state::remove_self_managed`, and the symlink creation helper from `link.rs` (which should be promoted out of `link.rs` if it isn't already accessible — implementer's call).
- The shell function lives wherever the implementer determines is least disruptive to load order. Sourcing it must not break a shell where the Rust binary is missing.

## Testing requirements

- Unit tests for `bashc configs check` covering:
  - All-linked manifest → silent, exit 0, no changes.
  - One `NotLinked` entry with missing target → auto-linked, one-line output, file becomes a symlink.
  - One `Conflict` entry → warning printed, file untouched.
  - Mixed: one `NotLinked` + one `Conflict` → both lines printed, only the safe one is fixed.
  - Stale `managed_configs.toml` entry (target missing on disk) → entry removed, no user-visible output.
  - Stale `managed_configs.toml` entry (target no longer referenced by any manifest entry) → entry removed.
  - Cross-platform safety: a self-managed marker for an entry whose `platform` field excludes the current platform must **not** be removed by cleanup, since the entry still exists in the manifest as a whole.
- Test that the command exits 0 even when only unresolved drift remains.
- Manual smoke test that opening a fresh shell after deleting a target file produces the expected one-liner and re-creates the symlink.

## Open questions

None at design time.

## Future work

- Optional rate-limit cache (timestamp file in `local/`) if shell-startup latency becomes a problem in practice.
- Optional `bashc configs check --json` for tooling integration.
- A Rust port of the shell wrapper as a single fast binary call, if process startup overhead becomes the dominant cost.
