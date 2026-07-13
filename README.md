# bashCustomization

`bashCustomization` is a hybrid Bash/Zsh customization framework. The shell
files remain the source of aliases, interactive functions, exports, completion,
and other behavior that must affect the current shell. The Rust `bashc` binary
handles standalone operations that benefit from structured platform detection,
error handling, installation orchestration, and config-file management.

The Rust migration is being stabilized and is not yet a complete replacement
for the shell framework. See the [current support matrix](docs/support.md) and
the [2026-07-13 implementation review](docs/reviews/2026-07-13-bashc-rust-port-review.md)
before relying on it for unattended fresh-machine setup.

The supported installer entry point is `bashc install`. The shell files under
`installScripts/` remain as readable migration/reference material, but are not
used as a fallback when the binary is missing because their older download and
platform assumptions do not meet the current installer safeguards.

## Load an existing checkout

The repository defaults to `$HOME/bashCustomization`. Set `BASHC_ROOT` when the
checkout lives elsewhere, then source `main.sh` from `.bashrc` and/or `.zshrc`:

```sh
export BASHC_ROOT="$HOME/bashCustomization"
if [ -f "$BASHC_ROOT/main.sh" ]; then
    . "$BASHC_ROOT/main.sh"
fi
```

`main.sh` is the entry point. It detects the current OS and shell, loads the
modules through `load_shell_extentionfiles`, performs the daily repository
update check, and asks `bashc configs check` to report config drift in
interactive shells. Set `BASHC_SKIP_UPDATE_CHECK=1` or
`BASHC_SKIP_CONFIG_CHECK=1` to disable those startup hooks while diagnosing a
problem.

## Build the current Rust binary

The current source version includes both installer and config commands:

```sh
cd "$BASHC_ROOT/rust"
cargo build --release
mkdir -p "$HOME/.mybin"
cp target/release/bashc "$HOME/.mybin/bashc"
```

The framework adds `$HOME/.mybin` to `PATH`. Useful commands include:

```sh
bashc install all --dry-run
bashc install ripgrep
bashc configs status
bashc configs link zellij
bashc configs diff
```

Use `--verbose` on install commands to stream full subprocess output. Config
targets and strategies are declared in `configs/manifest.toml`; review status
and diffs before forcing conflict resolution.

Config link and unlink operations are limited to targets within the current
home directory by default. Deliberate external targets require
`--allow-outside-home` on that invocation and are never changed by the automatic
startup check. See [configs/README.md](configs/README.md) for the source and
target containment rules.

### Config backup and rollback behavior

- `replace` moves the current target to `<target>.bak` and creates the link. If
  link creation fails, both the original target and any older backup are put
  back automatically.
- `discard` stages the original on the same filesystem and deletes it only
  after the replacement link succeeds. A failed link restores the original.
- A successful `replace` backup remains until `bashc configs unlink` restores
  it or the user removes it manually. Interactive unlink asks before restoring;
  `--yes` restores without prompting.
- A missing config source prevents every link strategy from changing its
  target.

## Release bootstrap status

`init.sh` now verifies the downloaded checksum, persists `bashc` under
`${BASHC_INSTALL_DIR:-$HOME/.mybin}`, clones or validates the repository, and
adds idempotent Bash and Zsh startup hooks. However, the only local release tag
is currently `v0.1.0`, which predates config management. Do not advertise the
remote one-command bootstrap as complete until a config-capable release has
passed the validation and release gates.

## Validation

Run the same non-container checks used by CI and release validation:

```sh
tests/validate.sh
```

This checks both Rust crates, validates every `.sh` file with Bash and Zsh,
runs focused ShellCheck gates, and exercises isolated startup/update smoke
tests. Full distro E2E tests additionally require a running Docker daemon.
Run `tests/e2e/run.sh` for the source-fresh, non-destructive distro behavior
tier used by CI and releases. Add `--features full-install-tests` to include
the slower package-install suites. The runner removes its containers, images,
and extracted binary on both success and failure unless
`KEEP_E2E_RESOURCES=1` is set.
The network artifact trust model and pinned-script update procedure are
documented in [artifact verification](docs/security/artifact-verification.md).
Run `tests/dependency-policy.sh` with `cargo-deny` installed to reproduce the
advisory, license, crate-ban, and source policy that also gates CI and releases.

## Recovery

- If startup fails, run `BASHC_SKIP_UPDATE_CHECK=1 BASHC_SKIP_CONFIG_CHECK=1`
  before sourcing `main.sh` and inspect the first reported module error.
- If `bashc` is missing, rebuild it as above or restore a verified release
  binary to `$HOME/.mybin/bashc`.
- Config replacement keeps the previous target as `<target>.bak`; inspect that
  backup before deleting it. Failed replacement and discard operations attempt
  automatic rollback and report the exact staging path if rollback is incomplete.
- Machine-specific overrides belong in `local/`, which is intentionally not
  version controlled.
