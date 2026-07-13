# bashc Rust Port Review

> **Remediation note (2026-07-13):** This document records the reviewed
> baseline and is intentionally not rewritten to erase the findings. Many
> safeguards and orchestration findings were addressed later the same day. The
> current implementation checklist and remaining clean-host evidence are in
> `docs/plans/2026-07-13-bashc-rust-port-remediation.md` and
> `docs/support.md`.

**Date:** 2026-07-13  
**Status:** Complete  
**Scope:** The Rust `bashc` implementation, its integration with the shell framework, the migration specifications and plans, bootstrap and release paths, configuration management, tests, portability, maintainability, and operational safeguards.

## Executive summary

The Rust port is a promising Phase 1 foundation, but it is not yet a dependable replacement for the shell project or a safe fresh-machine bootstrap. Its strongest area is configuration management. The installer code is generally readable and modular, but its current contracts cannot accurately represent platform applicability, partial installations, repair work, or transactional failure.

The port nominally covers most of the dedicated installer names and adds useful config-file management. It still owns only a minority of the framework's total responsibilities: shell startup, aliases, functions, settings, environment mutation, update behavior, general-script dispatch, and much of initial machine setup remain in shell. This division is not inherently a problem. A faithful migration should remain hybrid because operations that mutate the current shell, and the reference value of readable aliases and functions, are part of the project's original purpose.

The current implementation should be treated as an active development branch rather than a completed migration phase. The highest-priority work is to make installation outcomes platform-aware, make bootstrap persistent and coherent, prevent config-management data loss, restore a trustworthy Bash sourcing chain, and put validation in front of releases.

## Review basis

The review compared the implementation against:

- The original shell framework sourced from `main.sh`.
- The migration design and phase plans under `docs/specs/` and `docs/plans/`.
- The Rust implementation under `rust/src/`.
- The config manifest and shell-startup integration.
- The bootstrap and release workflow.
- Unit, lint, formatting, syntax, sourcing, and E2E test infrastructure.

The review was read-only. No tracked files were changed while collecting findings.

## Overall assessment

| Area | Assessment |
| --- | --- |
| Architectural direction | Sound hybrid direction, but the written goal overstates what should move out of shell. |
| Rust readability | Generally good module boundaries; config management is stronger than installation orchestration. |
| Maintainability | Moderate. Clear files and error context are offset by weak installer state contracts, registry boilerplate, stale documentation, and limited behavioral tests. |
| Cross-platform behavior | Detection is improved, but detected platforms are often only stubbed and are described too broadly as supported. |
| Safeguards | Mixed. Some downloads and filesystem states are handled carefully, while destructive config operations, install rollback, and supply-chain verification remain insufficient. |
| Responsibility coverage | A substantial portion of installer names, plus config management; a minority of the complete shell framework. |
| Production readiness | Not ready for use as the authoritative fresh-machine setup path. |

## Strengths worth preserving

### Clear top-level separation

The separation between `common`, `install`, and `configs` makes the Rust project approachable. Platform parsing, download helpers, privilege handling, package routing, installer modules, and config operations can be found without extensive tracing.

### Improved platform modeling

The Rust platform layer is materially stronger than the scattered shell checks. It models macOS, Linux, WSL, architecture, distro families, and the Ubuntu/Debian distinction explicitly. Unsupported architecture and OS errors are clearer than the shell fallback behavior.

### Useful error context

Most Rust operations use `Result` and add contextual failure messages. Privileged execution normally preserves argument boundaries, including explicit escaping for the `su -c` fallback.

### Selected download verification

The Go and kubectl direct-download paths verify SHA-256 checksums. Temporary directories are used consistently for many downloaded artifacts.

### Mature config state model

Config management recognizes correct links, dangling correct links, wrong links, regular-file conflicts, missing targets, missing sources, and self-managed files. Status and diff behavior are substantially tested, and source paths receive lexical containment validation.

### Broad unit coverage

The main Rust crate has 224 passing unit tests. The tests provide good coverage for platform parsing, helper behavior, config state classification, config output, and selected installer decisions.

## Critical and high-priority findings

### 1. `install all` cannot represent platform applicability

Every registered tool participates in an `all` run. There is no outcome for a tool that is intentionally not applicable to the detected platform.

This makes expected platform differences look like failures:

- Doas rejects macOS but is always registered.
- Doas requires the process itself to be root on Debian/Ubuntu, so an ordinary user with working sudo still receives a failed `install all` result.
- Brew is always registered even on distro families where the implementation rejects it.
- Platform stubs and declarative NixOS guidance are mixed with genuine success and failure states.

The current dry-run behavior demonstrated the problem on macOS by planning a doas installation. A real run would record the platform rejection as a failure.

### 2. The bootstrap does not produce a persistent installation

`init.sh` downloads `bashc` into a temporary directory, runs it, and deletes it. It then recommends `bashc install bashc`, but that installer does not exist.

The bootstrap also does not clone the repository, configure Bash or Zsh startup, persist the binary on `PATH`, configure Git, or create the project directories handled by the old bootstrap.

The remote repository currently exposes only the `v0.1.0` tag. That version predates the `configs` command, while the development branch invokes `bashc configs check` during shell startup. The config implementation is therefore not available through the documented release bootstrap.

On ARM Alpine, the bootstrap constructs an `aarch64-unknown-linux-musl` asset name, but the release workflow does not build that target.

### 3. Config linking can lose user data

Manifest targets are required to be absolute, but are not constrained to an approved area. The current manifest uses sensible paths under the user's home directory, while the format permits targets such as the home directory itself or system directories.

Forced discard removes the existing file or recursively removes the existing directory before the replacement symlink is known to be viable. If link creation fails, the original is gone. Conflict detection also prioritizes the existing target over source existence, so forced discard can remove a valid local target and create a dangling link when the repo source is missing.

Replace mode is recoverable but not transactional: it removes an older backup, moves the current target, and only then creates the symlink. There is no automatic rollback if the last step fails.

The self-managed state file is updated with ordinary read-modify-write operations. Concurrent interactive shells can race while creating links or updating this file, potentially producing startup errors or lost state.

### 4. Bash startup reports success after a source failure

`programExtensions/git/ilyaFunctions.sh` contains a backslash-continued pipeline with inline comments that Bash parses as an unmatched command substitution. Zsh accepts the file.

The extension loader does not propagate the failure. Later commands return success, so `main.sh` prints that extensions loaded and itself returns zero even though the Git extension was not fully sourced.

The daily update path has a second Bash-specific issue: `updateShell` is an alias defined after the function body that refers to it was parsed. When the update path runs, Bash reports `updateShell: command not found`, reloads anyway, and returns success.

These behaviors violate the framework's requirement that every sourced module be sourceable without cascading or hidden failure.

## Significant design and correctness findings

### Installer state is too coarse

The Boolean `is_installed()` contract treats several composite tools as complete when only one component exists. Examples include JavaScript tooling, the Debian `batcat` compatibility link, the Debian `fdfind` compatibility link, JDK completeness, and kubectl-related companion tools.

This prevents idempotent repair and makes the orchestrator skip work that the installer itself was written to perform.

### Single-tool prerequisites are not guaranteed

On a fresh macOS machine, a direct package-based tool install can invoke `brew` without first ensuring Homebrew exists. Prerequisite ordering is currently implicit in `install all`, rather than guaranteed by each requested operation.

### Dry-run and verbose do not meet their contracts

The orchestrator intercepts dry-run before calling installer-specific planning branches. Detailed messages such as package-manager choice, URL, checksum, and destination are therefore unreachable. Already-installed tools are skipped before their plans can be inspected.

`--verbose` is parsed but discarded. Subprocess output is always inherited through visible command execution, so there is no quiet default with failure-only output.

### Documentation and implementation disagree about concurrency

The design and migration status describe phased parallel installation. The implementation deliberately runs installer phases sequentially to avoid package-manager and system-state races. The sequential choice is reasonable, but the documentation, Tokio dependency, and completion claim should reflect the actual requirement.

### Some installers report success after ignored failures

The macOS base installer warns about each failed package and returns success. WSL Docker group creation and membership changes ignore errors. The Ubuntu universe repository addition is also best-effort. These paths can produce a successful top-level outcome while required postconditions are not satisfied.

### Install replacement is not consistently transactional

The Go direct installer removes the existing `/usr/local/go` tree before the new archive is known to extract successfully. Other direct downloads move artifacts into their final location before all finalization steps have succeeded. Interrupted or failed upgrades can therefore damage a working installation.

## Security and supply-chain findings

The implementation has useful foundations but does not yet meet the design's broad verification goal.

- Homebrew, rustup, nvm, pnpm, and Bun execute downloaded shell code directly.
- Neovim, Obsidian, and Nerd Font downloads are not checksum-verified.
- Obsidian falls back to any `.deb` asset if an architecture-specific asset is not found.
- Apt signing keys are installed without validating an expected fingerprint.
- The Eza repository uses an HTTP URL.
- HTTP clients have no explicit timeout or retry policy.
- Bootstrap checksum verification is skipped when neither checksum utility is available.
- A checksum hosted beside an artifact detects accidental corruption but does not independently authenticate a compromised release publisher.
- No dependency security audit is configured as a release or continuous-integration requirement.

## Testing and release findings

### Main crate

- `cargo test --all-targets`: 224 passed, 0 failed.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt --all -- --check`: failed across 13 Rust source files.

### E2E crate

- `cargo test --all-targets --no-run`: compiled successfully.
- Formatting check: failed.
- Clippy with warnings denied: failed with two findings.
- Full container execution was not possible during the review because the Docker daemon was unavailable.

The E2E assertions also have reliability gaps:

- Ubuntu tests expect the older Debian-family label.
- NixOS guidance tests accept broad distro text without requiring the intended successful guidance outcome.
- Fedora and Arch tests mostly prove absence of panic rather than correct errors.
- Existing images are reused without source-aware invalidation unless a manual rebuild variable is set.
- Shared containers are left running despite the cleanup requirement.

### Shell framework

- `init.sh` passed POSIX syntax validation and ShellCheck.
- Zsh syntax validation passed across the shell scripts.
- Bash syntax validation found the Git extension failure.
- An isolated Zsh startup succeeded.
- An isolated Bash startup emitted the syntax failure but returned success.
- The isolated Bash daily-update path also emitted `updateShell: command not found` and returned success.
- A repository-wide ShellCheck warning-level scan reported 65 errors and 185 warnings. Some are expected for source fragments without shebang directives, while array handling, quoting, local scope, and control-flow findings remain genuine.

### Release automation

The tag-triggered release workflow performs builds and publishes artifacts without first requiring formatting, Clippy, unit tests, ShellCheck, Bash/Zsh sourcing smoke tests, dependency audit, or E2E tests. A tagged commit can therefore be released despite known validation failures.

## Responsibility and migration status

### Responsibilities that appropriately remain in shell

- Aliases and interactive shell functions.
- Exports and changes that must affect the current process.
- Completion and prompt integration.
- Readable examples that preserve the project's value as a shell reference.

### Responsibilities currently implemented in Rust

- Platform and distro detection for Rust commands.
- Privilege escalation selection.
- A registry of 22 installation operations.
- Package-manager and direct-download helpers.
- Config linking, unlinking, status, diff, and startup checking.

### Responsibilities planned but not implemented in Rust

- General-script dispatch and its individual operations.
- Public platform output for shell integration.
- Public version comparison.
- Shell initialization generation.
- Persistent self-installation.
- A complete machine bootstrap that connects repository, binary, and shell startup.

The migration plan's completed Phase 1 label reflects the presence of installer modules, not dependable fulfillment of the phase's user-facing requirements.

## Final verdict

The Rust port should be kept and strengthened, not discarded or rewritten wholesale. Its platform model, error handling, config state work, and module boundaries are a better foundation than the original install-script collection.

The immediate objective should be stabilization rather than broader migration. The project needs trustworthy platform outcomes, repairable installer state, persistent bootstrap, transactional config behavior, strict source failure propagation, and release gates. Once those foundations are in place, the existing architecture is suitable for continued migration of standalone general scripts while aliases, functions, and current-shell mutations remain intentionally readable shell code.
