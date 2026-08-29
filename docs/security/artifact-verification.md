# Network Artifact Verification Policy

**Effective date:** 2026-07-13<br>
**Scope:** The supported `init.sh` and Rust `bashc install` paths.

The shell files under `installScripts/` are thin compatibility launchers into
`bashc install`, so they inherit this policy. They require the Rust binary and
return an actionable error when `bashc` is unavailable instead of maintaining
a separate installer path.

## Required behavior

Every supported network-delivered executable, archive, package, or installer
script must:

- use HTTPS transport;
- select an exact operating-system and architecture asset where variants exist;
- finish downloading before execution or privileged installation begins;
- have a publisher-provided SHA-256 value, an exact pinned SHA-256 value, or an
  apt signature chain rooted in an expected repository-key fingerprint;
- fail closed when verification metadata is absent, ambiguous, unsupported, or
  mismatched;
- preserve the working installation until the replacement is verified and
  staged; and
- report enough context to distinguish network, verification, staging, and
  activation failures.

A checksum published beside an artifact protects against corruption and CDN
substitution but does not independently protect against compromise of the
publisher account. Pinned installer-script hashes additionally require a source
review before an upstream script change is accepted.

## Current trust inventory

| Path | Selection | Verification before mutation |
| --- | --- | --- |
| `init.sh` release binary | Exact release target allowlist | Release `.sha256`; bootstrap fails if no checksum utility exists |
| Go archive | Exact OS/architecture entry from `go.dev` release JSON | SHA-256 supplied by the release entry |
| kubectl binary | Exact OS/architecture path | Publisher `.sha256` sidecar |
| Neovim AppImage | Exact `x86_64` or `arm64` GitHub release asset | Mandatory `sha256:` digest from GitHub release metadata |
| Obsidian Debian package | Exact `amd64` `.deb`; no cross-architecture fallback | Mandatory `sha256:` digest from GitHub release metadata |
| JetBrains Mono Nerd Font | Exact release asset name | Exact entry in the release's `SHA-256.txt`; duplicate/missing entries fail |
| Homebrew installer | Official HTTPS script at an immutable reviewed commit | Pinned SHA-256 in source; complete download before execution |
| rustup | Brew formula on Brew-supported hosts; official HTTPS fallback elsewhere | Homebrew bottle verification, or pinned SHA-256 before fallback execution |
| nvm installer | Versioned upstream URL | Pinned SHA-256 in source; complete download before execution |
| pnpm installer | Brew formula on Brew-supported hosts; official HTTPS fallback elsewhere | Homebrew bottle verification, or pinned SHA-256 before fallback execution |
| Bun installer | Brew formula on Brew-supported hosts; official HTTPS fallback elsewhere | Homebrew bottle verification, or pinned SHA-256 before fallback execution |
| Docker apt repository | Distro-specific HTTPS source | Exact Docker primary-key fingerprint plus apt Release signatures |
| Microsoft apt repositories | Distro-specific HTTPS source | Exact Microsoft primary-key fingerprint plus apt Release signatures |
| GitHub CLI apt repository | HTTPS source | Exact complete set of GitHub CLI primary-key fingerprints plus apt Release signatures |
| HashiCorp apt repository | HTTPS source | Exact HashiCorp primary-key fingerprint plus apt Release signatures |
| eza apt repository | HTTPS source | Exact eza primary-key fingerprint plus apt Release signatures |

Publisher bootstrap scripts can perform their own downstream downloads after
the pinned entry script starts. The pinned hash authenticates the reviewed
entry point; downstream trust remains governed by that upstream installer. This
is an explicit limitation, not equivalent to independently signed artifacts.

The repository clone and daily update trust the configured Git remote and its
HTTPS/SSH authentication. Commit-signature enforcement is not currently a
supported requirement, so unattended updates inherit the security of the Git
hosting account and selected branch.

## Network bounds and retries

Rust downloads use a 10-second connection timeout, a 120-second overall request
timeout, and at most three attempts. Connection/body failures, HTTP 408, HTTP
429, and HTTP 5xx responses are retried with short bounded backoff. Other HTTP
4xx responses fail immediately. Completed downloads replace their destination
from a same-directory staging file so a partial transfer cannot truncate a
working file.

Bootstrap curl requests use the same connection and request timeouts and at
most three total attempts. Installer retry behavior is intentionally bounded so
offline or invalid endpoints do not hang a fresh-machine setup indefinitely.

## Rotating a pinned script or repository key

An upstream hash or fingerprint change is a review event. The maintainer must:

1. Confirm the new material through the publisher's official documentation and
   HTTPS endpoint.
2. Review the changed installer script or key transition and identify any new
   downstream downloads, privileges, or persistent changes.
3. Update the pinned value and this inventory in the same change.
4. Run the unit, lint, shell, and applicable clean-host installer gates before
   release.

Do not weaken verification or add an unverified fallback merely to make an
upstream rotation pass.
