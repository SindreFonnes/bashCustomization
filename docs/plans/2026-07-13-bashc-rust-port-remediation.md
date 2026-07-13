# bashc Rust Port Remediation Plan

**Date:** 2026-07-13  
**Source review:** `docs/reviews/2026-07-13-bashc-rust-port-review.md`  
**Objective:** Make the Rust-assisted shell framework dependable for supported fresh-machine setup without losing the readable, current-shell behavior that belongs in shell.

## Status legend

- [ ] Not started
- [~] In progress
- [x] Complete and verified
- [!] Blocked or explicitly deferred with a documented reason

## Governing requirements

- Shell aliases, functions, exports, completion, and other current-process behavior remain in shell unless a Rust interface can preserve their semantics and reference value.
- Supported means the documented end-to-end outcome works on that platform. Detection or a graceful stub alone does not qualify as support.
- Installation and config operations are idempotent: rerunning them repairs incomplete state without damaging correct state.
- Expected platform differences are reported as not applicable or guidance, not as installation failures.
- Destructive operations preserve recoverability until their replacement is proven successful.
- Every script reached from `main.sh` is valid in its intended shells, and source failures propagate to the caller.
- Release artifacts are produced only from code that passes the repository's required validation gates.

## Workstream A: Documentation and support contract

- [x] A1. Record a dated implementation review covering scope, architecture, maintainability, safeguards, portability, responsibility coverage, and validation results.
- [x] A2. Record a requirements-driven remediation backlog with verification criteria.
- [x] A3. Publish one current support matrix that distinguishes detected, dry-run capable, install capable, tested, and released platform/architecture combinations.
- [ ] A4. Reconcile phase status documents with current behavior; completed status must represent satisfied user outcomes rather than the presence of files.
- [x] A5. Replace the top-level README bootstrap instructions with an accurate description of the hybrid architecture, prerequisites, supported platforms, and recovery guidance.

### Acceptance criteria

- A reader can determine what works without consulting source code or historical plans.
- Stubs are labeled as planned or detected, never as fully supported.
- The documented bootstrap matches the released artifact behavior.

## Workstream B: Installer orchestration and state

- [~] B1. Distinguish installed, incomplete/repairable, not applicable, guidance-only, failed, and dry-run outcomes where those differences affect user behavior.
- [~] B2. Ensure `install all` completes successfully when all applicable tools succeed, without failing for tools intentionally unavailable on the current platform.
- [~] B3. Ensure a single-tool request satisfies or clearly reports all prerequisites, including package-manager prerequisites.
- [ ] B4. Detect and repair incomplete composite installations, including JavaScript tools and Debian compatibility links.
- [x] B5. Make dry-run show the platform-specific planned operation without making changes, including when a tool is already installed.
- [x] B6. Make verbose behavior match the CLI contract and provide useful captured failure output by default.
- [~] B7. Ensure the summary distinguishes newly installed, already complete, repaired, not applicable, guidance, planned, and failed operations without misleading success counts.
- [ ] B8. Reconcile sequential versus parallel execution requirements and remove unused runtime complexity.
- [ ] B9. Ensure ignored best-effort operations do not produce success when required postconditions are absent.

### Acceptance criteria

- `bashc install all` has deterministic, truthful exit status and summary behavior on every documented platform.
- An ordinary supported-platform user is not required to install an alternative privilege tool that the platform does not need.
- Re-running any installer either makes progress toward its declared postconditions or reports why it cannot.
- Dry-run and real execution use the same applicability and planning decisions.

## Workstream C: Bootstrap and release availability

- [x] C1. Define the complete fresh-machine outcome: repository location, persistent binary location, shell startup integration, user directories, optional Git setup, and tool installation.
- [x] C2. Make bootstrap leave a verified `bashc` binary available on `PATH` after temporary files are removed.
- [x] C3. Remove or implement every bootstrap instruction that references a self-install command.
- [x] C4. Make bootstrap either install/locate the repository or clearly require and validate an existing clone before invoking repository-dependent commands.
- [x] C5. Support Bash and Zsh startup integration without duplicate entries and with recovery instructions.
- [x] C6. Reject platform/architecture combinations for which no release artifact is produced before attempting a download.
- [!] C7. Release the config-capable binary before shell startup depends on the `configs` command. Deferred until the remediation changes are reviewed and a new version is intentionally tagged.

### Acceptance criteria

- A fresh supported machine can complete the documented setup from one entry point.
- Starting a new supported shell finds both `main.sh` and the persistent `bashc` binary.
- Every target selected by bootstrap exists in the release matrix.
- A failed bootstrap leaves clear recovery instructions and no false success message.

## Workstream D: Config-management safety

- [x] D1. Prevent link, replace, and discard operations from changing a target when the source is missing or unusable.
- [x] D2. Preserve or automatically restore the previous target when replacement link creation fails.
- [ ] D3. Require explicit, separately acknowledged authority for destructive targets outside the user's home/config scope.
- [x] D4. Reject duplicate active targets and invalid platform selectors when loading the manifest.
- [x] D5. Make self-managed state updates atomic and safe against concurrent shell startups.
- [x] D6. Make automatic startup checks tolerate races without noisy false failures or lost state.
- [ ] D7. Validate source containment against filesystem indirection where destructive decisions depend on the source.
- [x] D8. Document backup lifecycle, rollback behavior, and manual recovery.

### Acceptance criteria

- No forced strategy can permanently remove the only copy of user data before a valid replacement exists.
- A missing source never triggers target deletion.
- Concurrent `configs check` operations converge on one correct state.
- Invalid manifests fail validation with actionable diagnostics before mutating the filesystem.

## Workstream E: Shell sourcing reliability

- [x] E1. Fix the Bash syntax error in the Git extension without breaking Zsh behavior.
- [x] E2. Make every module loader stop and return non-zero when a required source fails.
- [x] E3. Replace the daily update alias dependency with behavior that works when called from a previously parsed Bash function.
- [x] E4. Make update state advance only after the update operation succeeds or is intentionally skipped.
- [x] E5. Use an actual date or elapsed-time rule rather than weekday equality for daily-update state.
- [x] E6. Add isolated Bash and Zsh startup smoke tests that exercise both normal and update-due paths.

### Acceptance criteria

- `source main.sh` returns zero only when all required modules loaded successfully.
- Bash and Zsh load the same intended public functions and aliases without syntax diagnostics.
- An update failure is visible and does not produce a success/reloaded message.

## Workstream F: Download, upgrade, and supply-chain safeguards

- [ ] F1. Define verification requirements for each direct artifact and downloaded installer script.
- [ ] F2. Verify architecture-specific assets and reject ambiguous fallback assets.
- [ ] F3. Validate expected repository signing-key fingerprints before trusting new apt sources.
- [ ] F4. Use authenticated transport for package repositories.
- [ ] F5. Add bounded network timeouts and a documented retry policy.
- [ ] F6. Preserve working installations until replacement artifacts are downloaded, verified, and staged successfully.
- [x] F7. Treat unavailable checksum tooling as a bootstrap failure unless the user explicitly chooses a documented reduced-assurance path.
- [ ] F8. Add dependency vulnerability and license-policy checks appropriate for release gating.

### Acceptance criteria

- Each network-delivered executable path has a documented trust and verification model.
- Wrong-architecture and unverifiable artifacts are rejected before installation.
- Interrupted upgrades leave the previously working tool available or provide automatic rollback.

## Workstream G: Tests and release gates

- [x] G1. Make both Rust crates pass formatting and Clippy with warnings denied.
- [~] G2. Add behavioral tests for `install all` applicability and exit status across modeled platforms.
- [~] G3. Add tests for partial-install repair and prerequisite handling.
- [x] G4. Add failure-injection tests for config replacement, rollback, missing sources, duplicate targets, and concurrent startup checks.
- [~] G5. Correct E2E distro expectations and require intended exit status plus intended outcome text.
- [x] G6. Make E2E images source-aware so stale binaries cannot satisfy a new test run.
- [~] G7. Ensure E2E resources are cleaned after success and failure.
- [~] G8. Classify existing ShellCheck findings and resolve all actionable errors in sourced or executed scripts.
- [~] G9. Gate pull requests and release tags on Rust formatting, Rust lint, unit tests, shell validation, Bash/Zsh startup smoke tests, and the appropriate E2E tier.
- [x] G10. Keep release publishing downstream of all required validation jobs.

### Acceptance criteria

- The checks documented as required can be run locally and in CI with the same outcome.
- A release cannot be created when any required validation fails.
- Platform tests fail for wrong behavior, not only for panics.

## Workstream H: Migration scope after stabilization

- [ ] H1. Reassess general scripts and identify which are standalone operations suitable for Rust and which should remain shell.
- [ ] H2. Expose platform information to shell through a stable interface before removing duplicate shell detection.
- [ ] H3. Expose version comparison through a tested public command before removing shell implementations.
- [ ] H4. Preserve discoverability by documenting what each migrated operation does, its platform behavior, and its dry-run plan.
- [ ] H5. Port additional general scripts only after Workstreams B through G meet their acceptance criteria for the supported baseline.

### Acceptance criteria

- Migration decisions are based on process-boundary requirements and maintainability, not a goal of eliminating shell.
- Users retain a straightforward way to inspect how aliases, functions, settings, and automated operations work.

## Initial implementation order

1. Correct installer applicability and summary truthfulness.
2. Prevent config target deletion when sources or replacement links are invalid.
3. Restore strict Bash/Zsh source failure behavior and repair the daily-update path.
4. Make bootstrap persistent and align it with actually released commands.
5. Add validation gates that reproduce the review checks.
6. Address installer repair, transaction, and supply-chain requirements.
7. Resume broader migration only after the supported baseline is stable.

## Progress recorded on 2026-07-13

The first stabilization pass implemented platform-aware installer outcomes,
detailed dry-run behavior, functional verbose output, persistent bootstrap,
transactional config conflict handling, manifest validation, atomic locked
config state, strict shell source propagation, daily-update retry semantics,
Bash/Zsh smoke tests, source-fresh E2E image defaults, and CI/release validation.

The local validation entry point is `tests/validate.sh`. At the end of this
pass, both Rust crates passed formatting and Clippy with warnings denied, all
237 main-crate tests passed, all E2E targets compiled, every shell script passed
Bash and Zsh syntax parsing, and the startup smoke suite passed in both shells.
Full distro E2E execution remains unverified because no Docker daemon was
available. The uncompleted checklist items above remain requirements, not
implicit follow-up claims.
