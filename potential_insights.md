# Potential Insights

## Ubuntu vs Debian requires separate Distro enum variants

Originally `Distro::Debian` covered both Ubuntu and Debian. This caused 5
installer bugs where Ubuntu-specific assumptions (repo URLs, codenames,
`universe` repo) were applied to Debian. The fix was adding `Distro::Ubuntu`
as a separate variant while keeping `is_debian()` returning true for both.
Key principle: use `is_debian()` for "is this apt-based?" and `is_ubuntu()`
only for Ubuntu-specific operations.

## Codename detection should fail loudly, not fall back

Several installers originally fell back to `"jammy"` (Ubuntu 22.04) when
`VERSION_CODENAME` was missing from `/etc/os-release`. This silently produced
wrong apt repo configs on Debian. Better to error clearly so the user knows
the codename is missing than to silently configure the wrong repo.

## bashc install exit-code behavior

`bashc install <tool>` now exits non-zero when installation fails.
`run_by_name` bails on `InstallOutcome::Failed`, so the process exit code
reflects actual success or failure. Tests can and should rely on the exit code.

**Impact on eza:** `bashc install eza` installs from the third-party
`deb.gierens.de` apt repo. If that repo is unreachable from the container (DNS,
firewall, or network policy), the install fails with a non-zero exit code.
The `ensure_eza_installed()` helper in e2e tests handles this gracefully by
warning instead of panicking, and downstream tests skip if eza is not present.

## eza requires external network in container tests

Unlike `ripgrep`, `shellcheck`, `bat`, `fd`, `java`, and `postgres` (all
available in standard Debian/Ubuntu apt repos), `eza` comes from the
third-party `https://deb.gierens.de` repo. Tests that verify eza installs in
containers need to account for the possibility that this repo is unreachable.

## java -version prints to stderr, not stdout

`java -version` reports the version string on stderr, not stdout. Tests that
verify Java is installed should use `assert_stderr_contains` rather than
`assert_stdout_contains`. This is a known quirk of the JVM CLI.

## Ubuntu setup does not have apt_install_lock

The original Ubuntu `setup.rs` did not expose `apt_install_lock()` or
`ensure_apt_updated()` — it pre-warmed the cache in `init_container()` instead.
When `fast_installs.rs` was added for Ubuntu, these functions were added to
`setup.rs` to match the Debian pattern and avoid apt lock contention in
concurrent tests.

## Applicability is an executable support contract

An installer should report itself as applicable only when the current platform
has a complete route to its postcondition. In particular, “Homebrew can run on
Fedora” is not enough unless a single-tool invocation also ensures Homebrew
before selecting the formula path. Treating applicability as a UI/filtering hint
caused truthful not-applicable outcomes to become late package-manager failures.

## Preserve legacy shell installers as reference, not an implicit fallback

The older `installScripts/` files remain valuable as a searchable record of how
the original Bash setup worked. Automatically executing them when `bashc` is
missing, however, creates two installer contracts with different platform and
verification guarantees. Keeping them as explicit reference material preserves
the learning value while making the Rust install path the one supported
responsibility boundary.

## Exact repository-key sets avoid appended trust

Checking that a downloaded keyring contains one expected fingerprint is weaker
than checking its complete primary-key set: a compromised download could append
an attacker-controlled primary key alongside the legitimate one. Apt keyring
bootstrap should compare the exact set of primary fingerprints while allowing
the publisher's normal subkeys.

## CI validator versions must be visible and reproducible

The Ubuntu runner's apt package currently provides ShellCheck 0.9, while a
developer machine using Homebrew may run a newer release. A script can therefore
pass locally but fail the warning-level CI gate. Validation logs should print
tool versions and divide the suite into named stages; suspected CI-only lint
failures should be reproduced with the runner's packaged version.

## Zsh completion initialization is interactive-only

`compinit` may prompt when it finds insecure completion directories. On a CI
runner or another noninteractive shell there is no terminal for that prompt, so
initialization aborts and leaves `compdef` unavailable. Modules sourced from
`main.sh` must skip completion initialization and registration unless Zsh is
interactive; tests should explicitly verify this with stdin detached.
