# Branch fixes and follow-up review

Reviewed `feature/bashc-configs-command` at `e902ab1` against local `main` at
`fce8c6c`, then reviewed the uncommitted fixes. The focus was Rust config
mutations, installer dispatch, shell command availability, and their tests.

All five findings from the initial review have been addressed:

- Config commands validate the complete active target layout before selecting
  a named group. Parent/child targets and aliases are rejected before mutation.
- Targets and backups cannot overlap the repository config tree or its
  ancestors, preventing a replacement from removing its own source.
- Shell startup activates an existing Homebrew prefix independently of the
  installer subprocess, including standard locations when Brew is absent from
  the initial PATH.
- OpenJDK precedes system Java wrappers even if its directory was already late
  in PATH. Existing user Rust toolchains retain precedence over Brew rustup.
- NixOS dispatch uses explicit package and module-option mappings, including
  composite toolchains. Normal and dry-run invocations show the configuration.

The second review found two related config cases and included them in the fix:
backup paths can collide with another entry's target, and a currently wrong
parent symlink can hide a dependency from checks that resolve only the final
parent. The latter was reproduced with a failing CLI test before extending the
validation to retain every parent directory entry. Both now have passing
regression coverage. No additional confirmed actionable findings remain from
this follow-up pass.

Validation completed successfully:

- `tests/validate.sh`: 272 Rust unit tests, eight CLI config regression tests,
  two E2E-library tests, formatting, Clippy with warnings denied, E2E test
  compilation, repository Bash/Zsh syntax checks, ShellCheck, shell bootstrap
  tests, and Bash/Zsh sourcing and tool-PATH tests.
- `tests/e2e/run.sh`: 52 distro behavior tests across Alpine, Arch, Debian,
  Fedora, NixOS, and Ubuntu, plus the E2E-library tests. The script built the
  current source and cleaned up its test resources.
- `git diff --check`.

Zsh 5.9 was extracted from Ubuntu packages into a temporary directory for
validation; no system package installation was needed. Docker required access
outside the sandbox. Native macOS fresh-machine installs, ARM hardware, and
the opt-in `full-install-tests` suite were not exercised. The passing distro
behavior tier does not establish full clean-machine installation support.
