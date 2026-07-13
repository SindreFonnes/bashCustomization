# bashc Support Matrix

**Updated:** 2026-07-13

This matrix describes the current source tree, not an aspirational migration
phase. “Detected” means the Rust platform model recognizes the environment.
“Install baseline” means `install all` is intended to perform real work there;
it does not imply that every registered tool is available. “Verified” records
what the repository currently proves.

| Environment | Detected | Release target selected by bootstrap | Install baseline | Current verification |
| --- | --- | --- | --- | --- |
| macOS x86_64 | Yes | `x86_64-apple-darwin` | Intended | 260 main-crate tests, local dry-run, Bash/Zsh smoke; no clean-host E2E |
| macOS arm64 | Yes | `aarch64-apple-darwin` | Intended | Modeled unit tests and local arm64 dry-run; no clean-host E2E |
| Ubuntu/Debian x86_64 | Yes | `x86_64-unknown-linux-gnu` | Intended | Unit tests; source-fresh Docker behavior and full-install suites compile but were not executed locally |
| Ubuntu/Debian arm64 | Yes | `aarch64-unknown-linux-gnu` | Intended | Modeled by unit tests; no arm64 E2E |
| WSL on Ubuntu/Debian | Yes | `x86_64-unknown-linux-gnu` or `aarch64-unknown-linux-gnu` | Intended, not established | Platform unit tests only |
| Fedora-family x86_64 | Yes | `x86_64-unknown-linux-gnu` | Partial | Detection and selected behavior tests; several package paths remain stubs |
| Arch-family x86_64 | Yes | `x86_64-unknown-linux-gnu` | Partial | Detection and selected behavior tests; several package paths remain stubs |
| Alpine x86_64 | Yes | `x86_64-unknown-linux-musl` | Partial | Detection and selected behavior tests; several installers remain stubs |
| Alpine arm64 | Yes | None; rejected before download | No | Rejection path only |
| NixOS x86_64/arm64 | Yes if the binary can run | GNU Linux target, compatibility unverified | Guidance only | Unit tests require declarative guidance; host compatibility not established |
| Other Linux distributions | Recorded as unknown | GNU Linux target | No | Unsupported-path behavior only |

## Capability status

| Capability | macOS | Debian/Ubuntu | WSL | Fedora/Arch/Alpine | NixOS |
| --- | --- | --- | --- | --- | --- |
| Platform detection | Yes | Yes | Yes | Yes | Yes |
| Installer dry-run | Yes | Yes | Yes | Yes | Declarative plan |
| Config status/diff/link/unlink | Implemented | Implemented | Implemented | Implemented | Implemented if binary runs |
| `install all` clean-host proof | No | No | No | No | Not applicable |
| Source-fresh default container E2E definition | No | Yes | No | Yes | Guidance suite |
| E2E execution in latest review/remediation pass | No | Blocked by unavailable Docker daemon | No | Blocked by unavailable Docker daemon | Blocked by unavailable Docker daemon |

## Release availability

The release workflow defines five artifacts: x86_64 and arm64 macOS, x86_64
and arm64 glibc Linux, and x86_64 musl Linux. The only tag present in the local
repository is `v0.1.0`; that source does not contain the `configs` command.
Consequently, detection and a workflow target do not yet mean the current
config-capable source has been released.

Support should be promoted only after the applicable clean-host outcome is
tested, not merely because a platform can be detected or a binary can be built.

## Validation and security gates

Pull requests and releases run Rust format/lint/unit gates, shell syntax and
startup smoke tests, dependency advisory/license/source policy, and the
source-fresh non-destructive distro behavior tier. `tests/e2e/run.sh` removes
its images and containers after either success or failure. The slower real
package-install suites are available through the `full-install-tests` feature
but have not yet established the clean-host baseline recorded as missing above.
