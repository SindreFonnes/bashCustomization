# configs/

This directory contains version-controlled config files managed by `bashc configs`. Each subdirectory holds the source file(s) for a tool, and `manifest.toml` declares where each file should be symlinked on the local machine. The `bashc configs link`, `unlink`, `status`, `diff`, and `check` subcommands operate on these entries.

## Per-platform variants

When a config file needs to differ between operating systems, split it into one file per platform and add one manifest entry per file. The naming convention is `<tool>/config.<platform>.<ext>`. For zellij, that looks like:

```
configs/zellij/config.macos.kdl
configs/zellij/config.linux.kdl
```

The corresponding `manifest.toml` entries both point to the same target path, with the `platform` field controlling which one is active on each machine:

```toml
[[config]]
name = "zellij"
source = "zellij/config.macos.kdl"
target = "~/.config/zellij/config.kdl"
platform = "macos"

[[config]]
name = "zellij"
source = "zellij/config.linux.kdl"
target = "~/.config/zellij/config.kdl"
platform = "linux"
```

Note: this is an illustrative example. The actual zellij entry in this repo uses a single cross-platform `config.kdl` because OSC52 makes the per-OS clipboard split unnecessary — see the trade-off note below.

**Trade-off:** when most of the file is shared between platforms, this duplicates content. That is an accepted cost in exchange for keeping the simple symlink model and avoiding a templating engine. Use this pattern only when files genuinely differ per OS. For trivial differences, prefer a cross-platform default instead — for example, zellij's clipboard integration uses OSC52 (supported everywhere) rather than `pbcopy`, which avoids the need for per-platform files entirely.

The `platform` field is optional. Entries without it apply on all platforms.

## What `bashc configs check` does

`bashc configs check` is purpose-built for shell startup. It loads the manifest filtered for the current platform and computes the state of each entry:

- **`NotLinked` with no file at the target path** — the symlink is created automatically (safe — no existing data would be clobbered), provided the repo source actually exists. A single summary line is printed listing the per-entry count and the de-duplicated group names of newly linked configs.

  ```
  bashc: linked 2 config files (claude, zellij)
  ```

- **`NotLinked` with the repo source missing** — left untouched and surfaced as drift with the tag `missing source`. Auto-linking would otherwise create a dangling symlink that `detect_state` then misreports as `Linked`.
- **`Conflict`** (a real file exists at the target) or **`WrongSymlink`** (symlink points elsewhere) — the entry is left untouched and a warning line is printed, ending with a hint to run `bashc configs status` for details.

  ```
  bashc: ⚠ 2 config files need attention (zellij: conflict, claude: wrong symlink) — run 'bashc configs status'
  ```
- **`Linked`** or **`SelfManaged`** — no action, no output.

The command exits 0 in all non-fatal cases, including when unresolved drift exists, so that a startup warning never breaks the shell. Use `bashc configs status` for a read-only inspection of all entries at any time.

## When `bashc configs check` runs

`bashc_check_configs` in `general_functions.sh` invokes `bashc configs check` on every interactive shell startup, after modules are loaded. It is skipped automatically for non-interactive shells.

## Opting out

Set `BASHC_SKIP_CONFIG_CHECK=1` in the environment to suppress the startup check entirely. This is useful for CI runners, heavily nested subshells, or any context where the output is unwanted.

## Auto-pruning of `local/managed_configs.toml`

`bashc configs check` also silently removes stale self-managed markers from `local/managed_configs.toml`. A marker is removed when either:

1. The target path is no longer referenced by any entry in the unfiltered manifest (the entry was deleted on every platform), or
2. The target path is referenced by an entry in the current platform's filtered manifest, and the file at that target is missing on disk (the user deleted the local file).

Cross-platform safety: a marker for a macOS-only entry is preserved when checked from Linux, because condition 2 only applies to entries visible in the current platform's filtered manifest. This prevents a Linux machine from removing a marker that the macOS manifest still depends on.

## Adding a new config

1. Create a subdirectory under `configs/` for the tool if it does not exist.
2. Add the config file(s) to that directory.
3. Add one or more `[[config]]` entries to `manifest.toml`. Include a `platform` field only if the entry should be restricted to a specific OS.
4. Run `bashc configs link <name>` to create the symlink on the current machine (or open a new interactive shell — `bashc configs check` will auto-link it on startup).

---

For the design rationale and trade-off analysis behind this feature, see the [design spec](../docs/superpowers/specs/2026-04-07-multi-os-config-handling-design.md).
