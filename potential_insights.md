# Potential Insights

## bashc install always exits 0, even on silent failures

`bashc install <tool>` exits 0 regardless of whether the tool was actually
installed. The orchestrator (`install/orchestrator.rs`) converts
`InstallOutcome::Failed` into a printed message but still returns `Ok(())`.
This means e2e tests cannot rely on the exit code to verify installation
succeeded — they must verify the resulting binary works.

**Impact on eza:** `bashc install eza` installs from the third-party
`deb.gierens.de` apt repo. If that repo is unreachable from the container (DNS,
firewall, or network policy), the install silently fails but exits 0. The e2e
test `eza_version_works` therefore checks `command -v eza` before running
`eza --version`, and skips rather than fails if eza is not present.

**Possible fix:** The orchestrator could exit non-zero if any tool reports
`InstallOutcome::Failed`, at least when installing a single named tool
(`bashc install <name>`). This would let CI catch real failures rather than
masking them.

## eza requires external network in container tests

Unlike `ripgrep`, `shellcheck`, `bat`, `fd`, `java`, and `postgres` (all
available in standard Debian/Ubuntu apt repos), `eza` comes from the
third-party `http://deb.gierens.de` repo. Tests that verify eza installs in
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
