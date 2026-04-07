# `bashc configs` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `bashc configs link|unlink|status` subcommand that manages symlinks between version-controlled config files in the repo and their expected locations on the system.

**Architecture:** A new `configs` module alongside `install` handles manifest parsing (TOML), symlink operations, interactive conflict resolution (dialoguer), and local state tracking. The manifest at `configs/manifest.toml` defines source-target mappings with optional platform guards and default resolution strategies. Machine-specific state (self-managed configs) is tracked in `local/managed_configs.toml` (gitignored).

**Tech Stack:** Rust, clap (CLI), toml + serde (manifest), dialoguer (interactive menus), anyhow (errors), std::os::unix::fs (symlinks — Unix-only, which matches the project's target platforms: macOS, Linux, WSL)

**Note:** The existing `main()` is `async fn` with `#[tokio::main]` for the install command's parallel execution. All configs functions are synchronous — they are called directly from the async main context, which is fine. Do not make configs functions async.

---

## Platform Matching

Manifest entries use an optional `platform` string field. The mapping to `common::platform::Os` variants:

| Manifest `platform` | Matches `Os` variants |
|---|---|
| `"macos"` | `Os::MacOs` |
| `"linux"` | `Os::Linux(_)` AND `Os::Wsl(_)` |
| (omitted) | All platforms |

WSL is treated as Linux for config purposes — if you use a tool on Linux, you almost certainly use it on WSL too.

---

## File Structure

### New project-root files
- `configs/manifest.toml` — config entry definitions (name, source, target, platform, strategy)
- `configs/claude/CLAUDE.md` — version-controlled Claude global instructions
- `configs/claude/settings.json` — version-controlled Claude settings
- `configs/zellij/config.kdl` — version-controlled Zellij config
- `configs/ghostty/config` — version-controlled Ghostty config (mac only)

### New Rust source files
- `rust/src/configs/mod.rs` — module exports, `ConfigEntry` struct, `EntryState` enum, public API
- `rust/src/configs/manifest.rs` — TOML parsing, path resolution, platform filtering
- `rust/src/configs/state.rs` — local state read/write (`local/managed_configs.toml`), entry state detection
- `rust/src/configs/link.rs` — link command: symlink creation, conflict resolution menu, `--force` handling
- `rust/src/configs/unlink.rs` — unlink command: symlink removal, `.bak` restore prompt
- `rust/src/configs/status.rs` — status command: formatted output of all config states

### Modified files
- `rust/Cargo.toml` — add `toml` dependency
- `rust/src/main.rs` — add `Configs` variant to `Commands` enum, route subcommands
- `rust/src/common/mod.rs` — add `project_root` module

### New common utility
- `rust/src/common/project_root.rs` — resolve bashCustomization root directory

---

## Key Types

### `ConfigEntry` (parsed from manifest)
Fields: `name` (group name, e.g. "claude"), `source` (relative to `configs/`), `target` (absolute path after tilde expansion), `platform` (optional: "macos", "linux"), `strategy` (default conflict resolution: "prompt", "replace", "discard", "keep")

### `EntryState` (computed at runtime)
Variants: `Linked` (symlink points to correct source), `SelfManaged` (user chose to keep local), `Conflict` (real file exists, not managed), `NotLinked` (target doesn't exist), `WrongSymlink` (symlink points elsewhere)

### `Strategy` enum
Variants: `Prompt`, `Replace`, `Discard`, `Keep` — maps to `--force` values and manifest `strategy` field

### Force flag
`--force <strategy>` is required to specify which strategy: `--force replace`, `--force discard`, `--force keep`. Without `--force`, the manifest's `strategy` field is used (defaulting to `prompt`).

**CLI-to-enum mapping:** The `Strategy` enum's `clap::ValueEnum` implementation should use these CLI strings: `replace`, `discard`, `keep`. The manifest TOML file uses the same strings. `Prompt` is not a valid `--force` value (it would be contradictory), but is valid in the manifest `strategy` field.

---

## Task 1: Add `toml` dependency and create module skeleton

**Files:**
- Modify: `rust/Cargo.toml`
- Create: `rust/src/configs/mod.rs`
- Create: `rust/src/configs/manifest.rs` (empty stub)
- Create: `rust/src/configs/state.rs` (empty stub)
- Create: `rust/src/configs/link.rs` (empty stub)
- Create: `rust/src/configs/unlink.rs` (empty stub)
- Create: `rust/src/configs/status.rs` (empty stub)
- Modify: `rust/src/main.rs` (add `mod configs;`)

- [ ] **Step 1:** Add `toml = "0.8"` to `[dependencies]` in Cargo.toml
- [ ] **Step 2:** Create `configs/mod.rs` with module declarations for manifest, state, link, unlink, status. No public API yet — just `mod` statements.
- [ ] **Step 3:** Create empty stub files for each submodule (manifest.rs, state.rs, link.rs, unlink.rs, status.rs) with just a comment or empty function so it compiles.
- [ ] **Step 4:** Add `mod configs;` to main.rs
- [ ] **Step 5:** Run `cargo check` in `rust/` to verify it compiles
- [ ] **Step 6:** Commit: "feat(configs): add toml dependency and module skeleton"

---

## Task 2: Project root detection

**Files:**
- Create: `rust/src/common/project_root.rs`
- Modify: `rust/src/common/mod.rs`
- Test: in-file `#[cfg(test)]` module

The binary needs to find the `bashCustomization` project root to locate `configs/manifest.toml` and the config source files.

Resolution order:
1. `BASHC_ROOT` environment variable (if set)
2. `$HOME/bashCustomization` (established convention)

Validate that the resolved path exists and is a directory. The existence of `configs/manifest.toml` is checked separately by the manifest loader — `project_root()` only validates the directory itself. Error if `$HOME` is not set.

- [ ] **Step 1:** Write tests — env var override, default fallback, missing directory error
- [ ] **Step 2:** Run tests to verify they fail
- [ ] **Step 3:** Implement `project_root()` -> `Result<PathBuf>` in `common/project_root.rs`
- [ ] **Step 4:** Add `pub mod project_root;` to `common/mod.rs`
- [ ] **Step 5:** Run tests to verify they pass
- [ ] **Step 6:** Commit: "feat(configs): add project root detection"

---

## Task 3: Manifest parsing

**Files:**
- Create: `configs/manifest.toml` (project root, not inside rust/)
- Modify: `rust/src/configs/manifest.rs`
- Modify: `rust/src/configs/mod.rs` (export types)

### Manifest format

```toml
[[config]]
name = "claude"
source = "claude/CLAUDE.md"
target = "~/.claude/CLAUDE.md"

[[config]]
name = "claude"
source = "claude/settings.json"
target = "~/.claude/settings.json"

[[config]]
name = "zellij"
source = "zellij/config.kdl"
target = "~/.config/zellij/config.kdl"

[[config]]
name = "ghostty"
source = "ghostty/config"
target = "~/Library/Application Support/com.mitchellh.ghostty/config"
platform = "macos"
```

### Requirements
- Parse TOML into `Vec<ConfigEntry>`
- Expand `~` to `$HOME` in target paths
- Resolve `source` to absolute path relative to `<project_root>/configs/`
- Filter out entries whose `platform` doesn't match current platform (use `common::platform::Platform`)
- `strategy` defaults to `"prompt"` if omitted
- `platform` is optional — if omitted, applies to all platforms
- Validate that source files exist on disk after resolution (warn, don't fail — the user may not have created the file yet)
- Support loading entries filtered by group name (for `bashc configs link claude`)

- [ ] **Step 1:** Create `configs/manifest.toml` at the project root with the four config groups (claude x2, zellij, ghostty)
- [ ] **Step 2:** Define `ConfigEntry`, `Strategy` enum, and `Manifest` struct in `configs/mod.rs`. Derive `Deserialize` on the raw TOML struct, keep the resolved `ConfigEntry` as a separate type.
- [ ] **Step 3:** Write tests for manifest parsing — valid manifest, missing fields use defaults, unknown platform is filtered, tilde expansion, source path resolution
- [ ] **Step 4:** Run tests to verify they fail
- [ ] **Step 5:** Implement `load_manifest(project_root, platform)` -> `Result<Vec<ConfigEntry>>` in manifest.rs. Implement `filter_by_name(entries, name)` -> `Vec<ConfigEntry>`.
- [ ] **Step 6:** Run tests to verify they pass
- [ ] **Step 7:** Commit: "feat(configs): manifest parsing with platform filtering"

---

## Task 4: Create initial config files

**Files:**
- Create: `configs/claude/CLAUDE.md` (copy from `~/.claude/CLAUDE.md`)
- Create: `configs/claude/settings.json` (copy from `~/.claude/settings.json`)
- Create: `configs/zellij/config.kdl` (copy from `~/.config/zellij/config.kdl`)
- Create: `configs/ghostty/config` (copy from `~/Library/Application Support/com.mitchellh.ghostty/config`)

Copy the current config files from their system locations into the repo's `configs/` directory. These become the version-controlled source of truth. Having them in place early means all subsequent tasks can test against real files.

- [ ] **Step 1:** Create directory structure `configs/claude/`, `configs/zellij/`, `configs/ghostty/`
- [ ] **Step 2:** Copy each config file into the appropriate directory
- [ ] **Step 3:** Verify `configs/manifest.toml` source paths match the actual files
- [ ] **Step 4:** Commit: "feat(configs): add initial config files"

---

## Task 5: Local state tracking

**Files:**
- Modify: `rust/src/configs/state.rs`
- Test: in-file `#[cfg(test)]`

The file `local/managed_configs.toml` (gitignored, inside the bashCustomization project) tracks configs the user chose to keep self-managed on this specific machine.

### Format

```toml
[[self_managed]]
name = "claude"
source = "claude/settings.json"
target = "~/.claude/settings.json"
```

### Requirements
- `load_self_managed(project_root)` -> `Result<Vec<SelfManagedEntry>>` — returns empty vec if file doesn't exist
- `add_self_managed(project_root, entry)` -> `Result<()>` — append entry, create file if needed
- `remove_self_managed(project_root, target)` -> `Result<()>` — remove entry by target path
- `is_self_managed(entries, target)` -> `bool`

### Entry state detection

`detect_state(config_entry, self_managed_entries)` -> `EntryState`

**Precedence rules (order matters):**
1. Target is a symlink pointing to the correct source -> `Linked` (even if also in self_managed — the symlink takes precedence, indicating the user re-linked after previously keeping local)
2. Target exists AND is in self_managed list -> `SelfManaged`
3. Target is a symlink pointing elsewhere -> `WrongSymlink`
4. Target exists as a regular file -> `Conflict`
5. Target is in self_managed list but file no longer exists -> `NotLinked` (stale entry — the self-managed file was deleted; clean up the stale self_managed entry and print a note)
6. Target does not exist -> `NotLinked`

**Stale self-managed entries:** If a config is in `managed_configs.toml` but the target file no longer exists on disk, `detect_state` returns `NotLinked`. The *caller* is responsible for cleaning up stale entries from `managed_configs.toml` (since `detect_state` doesn't have access to `project_root`). The link/status/unlink commands should check for `NotLinked` entries that are also in the self-managed list and remove them, printing a note.

- [ ] **Step 1:** Write tests for state detection — all five states, self-managed file round-trip (write then read)
- [ ] **Step 2:** Run tests to verify they fail
- [ ] **Step 3:** Implement `SelfManagedEntry`, serialization, load/add/remove functions
- [ ] **Step 4:** Implement `detect_state()`
- [ ] **Step 5:** Run tests to verify they pass
- [ ] **Step 6:** Commit: "feat(configs): local state tracking and entry state detection"

---

## Task 6: Status command

**Files:**
- Modify: `rust/src/configs/status.rs`
- Modify: `rust/src/configs/mod.rs` (export public fn)

### Requirements
- `run_status(project_root, platform, filter_name: Option<&str>)` -> `Result<()>`
- For each config entry (optionally filtered by name), detect state and print a formatted line:
  - `Linked`:       `  ✓ claude/CLAUDE.md → ~/.claude/CLAUDE.md`
  - `SelfManaged`:  `  ○ claude/settings.json → ~/.claude/settings.json [self-managed]`
  - `Conflict`:     `  ✗ zellij/config.kdl → ~/.config/zellij/config.kdl [conflict: local file exists]`
  - `WrongSymlink`: `  ✗ ... [conflict: symlink points elsewhere]`
  - `NotLinked`:    `  - ghostty/config → .../config [not linked]`
- Group output by config name (show a header per group)
- If filter_name is provided, only show entries matching that name; error if name not found in manifest

- [ ] **Step 1:** Write test — status output for a mix of entry states (use tempdir fixtures)
- [ ] **Step 2:** Run test to verify it fails
- [ ] **Step 3:** Implement `run_status()`
- [ ] **Step 4:** Run test to verify it passes
- [ ] **Step 5:** Commit: "feat(configs): status command"

---

## Task 7: Link command — non-conflict path

**Files:**
- Modify: `rust/src/configs/link.rs`
- Modify: `rust/src/configs/mod.rs` (export public fn)

Handle the straightforward cases first — no existing file at target.

### Requirements
- `run_link(project_root, platform, filter_name: Option<&str>, force: Option<Strategy>)` -> `Result<()>`
- For each entry:
  - If state is `Linked` -> print skip message, continue
  - If state is `NotLinked` -> create parent directories, create absolute symlink, print success
  - If state is `SelfManaged` -> print skip message (self-managed), continue
  - If state is `Conflict` or `WrongSymlink` -> defer to conflict resolution (Task 8)
- Symlinks must be absolute paths
- Source file must exist (error if not)

- [ ] **Step 1:** Write tests — link when target doesn't exist (tempdir), skip when already linked, create parent dirs
- [ ] **Step 2:** Run tests to verify they fail
- [ ] **Step 3:** Implement the non-conflict path in `run_link()`
- [ ] **Step 4:** Run tests to verify they pass
- [ ] **Step 5:** Commit: "feat(configs): link command (non-conflict path)"

---

## Task 8: Link command — conflict resolution

**Files:**
- Modify: `rust/src/configs/link.rs`

### Interactive menu (when strategy is `Prompt`)

When a `Conflict` or `WrongSymlink` state is encountered and strategy is `Prompt`, present:

```
⚠ ~/.claude/settings.json already exists and is not managed by bashc.

  [v] View both versions (repo and local)
  [r] Replace local — backup as .bak, then symlink
  [d] Replace local — discard original, then symlink
  [k] Keep local — mark as self-managed on this machine
  [s] Skip for now

> _
```

Use `dialoguer::Select` for the menu.

**View option:** Print both files' contents (label which is which), then re-show the menu. Use `std::fs::read_to_string` for both source and target. Handle binary/large files gracefully (show size + "binary file" message if not valid UTF-8, or truncate with a note if over a reasonable line count like 100 lines).

**Replace (backup):** Rename target to `<target>.bak`. If `.bak` already exists, warn and overwrite it. Then create symlink.

**Replace (discard):** Delete target. Then create symlink.

**Keep:** Add to `local/managed_configs.toml` via state module. Print confirmation.

**Skip:** Do nothing, continue to next entry.

### `--force` handling

When `--force <strategy>` is provided, skip the interactive menu and apply the specified strategy to all conflict/wrong-symlink entries:
- `--force replace` -> backup and symlink
- `--force discard` -> delete and symlink
- `--force keep` -> mark self-managed

### Manifest `strategy` field

If the manifest entry has `strategy = "replace"` (etc.) and `--force` is NOT provided, use the manifest strategy as the default. If manifest says `"prompt"` (or omitted), show the interactive menu.

- [ ] **Step 1:** Write tests — force replace creates .bak, force discard removes original, force keep adds to self-managed, .bak overwrite on repeated replace
- [ ] **Step 2:** Run tests to verify they fail
- [ ] **Step 3:** Implement conflict resolution logic — strategy dispatch, backup/discard/keep operations
- [ ] **Step 4:** Run tests to verify they pass
- [ ] **Step 5:** Add the interactive `dialoguer::Select` menu for `Prompt` strategy (not easily unit-testable, but verify manually)
- [ ] **Step 6:** Commit: "feat(configs): link conflict resolution with --force and interactive menu"

---

## Task 9: Unlink command

**Files:**
- Modify: `rust/src/configs/unlink.rs`
- Modify: `rust/src/configs/mod.rs` (export public fn)

### Requirements
- `run_unlink(project_root, platform, filter_name: Option<&str>, yes: bool)` -> `Result<()>`
- For each entry (optionally filtered by name):
  - If state is `Linked`:
    - Remove the symlink
    - If `<target>.bak` exists, prompt: "Restore backup? [y/n]" (or auto-yes if `--yes`)
      - Yes: rename `.bak` back to original target path
      - No: just delete the symlink (backup stays as `.bak` for manual handling)
    - If no `.bak`, just remove the symlink
  - If state is `SelfManaged`:
    - Ask: "Remove self-managed marker? [y/n]" (or auto-yes if `--yes`)
    - Yes: remove from `local/managed_configs.toml`
    - No: skip
  - If state is `NotLinked`, `Conflict`, `WrongSymlink`: print skip message
- Also remove self-managed entries when unlinking (if present)
- `--yes` flag skips all confirmation prompts (answers "yes" to all). Useful for scripting.

- [ ] **Step 1:** Write tests — unlink removes symlink, unlink with .bak restores when told yes, unlink self-managed entry removal
- [ ] **Step 2:** Run tests to verify they fail
- [ ] **Step 3:** Implement `run_unlink()`
- [ ] **Step 4:** Run tests to verify they pass
- [ ] **Step 5:** Commit: "feat(configs): unlink command with .bak restore"

---

## Task 10: Wire up CLI

**Files:**
- Modify: `rust/src/main.rs`

### CLI structure

```
bashc configs link [name] [--force <strategy>]
bashc configs unlink [name] [--yes]
bashc configs status [name]
```

Add a `Configs` variant to the `Commands` enum with a nested subcommand enum (`ConfigsAction`): `Link`, `Unlink`, `Status`. Each accepts an optional `name: Option<String>` positional argument. `Link` additionally accepts `--force <strategy>`. `Unlink` accepts `--yes` to skip confirmation prompts.

**Error on unknown name:** All three subcommands should error with a helpful message if `name` is provided but matches no entries in the manifest. List the available names in the error.

The `Strategy` enum should implement `clap::ValueEnum` so clap can parse it directly from the CLI.

In the match block of `main()`, detect platform, resolve project root, and dispatch to the appropriate `configs::` function.

- [ ] **Step 1:** Add `ConfigsAction` enum and `Configs` variant to `Commands`
- [ ] **Step 2:** Add match arm in `main()` that resolves project root and dispatches
- [ ] **Step 3:** Run `cargo check` — verify it compiles
- [ ] **Step 4:** Run `cargo test` — verify all tests still pass
- [ ] **Step 5:** Commit: "feat(configs): wire up CLI subcommands"

---

## Task 11: Manual integration test

Not automated — verify the full flow works end to end on the real system.

- [ ] **Step 1:** Build: `cargo build` in `rust/`
- [ ] **Step 2:** Run `bashc configs status` — should show current state of all configs
- [ ] **Step 3:** Back up current config files manually, then test `bashc configs link` — verify symlinks are created, conflict resolution works
- [ ] **Step 4:** Run `bashc configs status` — should show all as `Linked`
- [ ] **Step 5:** Run `bashc configs unlink` — verify symlinks are removed, `.bak` restore prompt works
- [ ] **Step 6:** Run `bashc configs link --force replace` — verify non-interactive backup+symlink
- [ ] **Step 7:** Restore any manual backups
- [ ] **Step 8:** Commit any fixes discovered during testing

---

## Notes

- The `local/managed_configs.toml` file is automatically gitignored by the existing `local/*` pattern in `.gitignore` (with `!local/local_main.sh` exception).
- The `configs/` directory at project root is new and will be committed to git.
- Future config groups (nvim, git, etc.) can be added by creating a directory under `configs/` and adding entries to `manifest.toml`.
- The `settings.json` for Claude supports `settings.local.json` for machine-specific overrides. The user may want to document this in the config file or README so that per-machine permissions differences are handled outside of bashc.
