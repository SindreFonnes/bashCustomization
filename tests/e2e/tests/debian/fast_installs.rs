use bashc_e2e::assertions::{assert_exit_ok, assert_stderr_contains, assert_stdout_contains};
use tokio::sync::OnceCell;

use crate::setup;

// ---------------------------------------------------------------------------
// shellcheck
// ---------------------------------------------------------------------------

/// Install shellcheck exactly once for the lifetime of this test binary.
static SHELLCHECK_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_shellcheck_installed() {
    setup::ensure_apt_updated().await;
    SHELLCHECK_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "shellcheck"])
                .await
                .expect("install shellcheck exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn install_shellcheck_exits_zero() {
    ensure_shellcheck_installed().await;
}

#[tokio::test]
async fn shellcheck_version_works() {
    ensure_shellcheck_installed().await;

    let container = setup::get_container().await;
    let result = container
        .exec(&["shellcheck", "--version"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "shellcheck");
}

// ---------------------------------------------------------------------------
// eza
// ---------------------------------------------------------------------------

/// Install eza exactly once for the lifetime of this test binary.
///
/// eza is installed via a third-party apt repository (deb.gierens.de) which
/// requires adding a GPG key and a sources.list entry before installing.
static EZA_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_eza_installed() {
    setup::ensure_apt_updated().await;
    EZA_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "eza"])
                .await
                .expect("install eza exec failed");
            // eza depends on the third-party deb.gierens.de repo.
            // If unreachable, install will fail — don't panic here,
            // let downstream tests detect eza's absence and skip.
            if result.exit_code != 0 {
                eprintln!(
                    "Warning: eza install exited {} (third-party repo may be unreachable)",
                    result.exit_code
                );
            }
        })
        .await;
}

#[tokio::test]
async fn install_eza_does_not_panic() {
    ensure_eza_installed().await;
}

#[tokio::test]
async fn eza_version_works() {
    ensure_eza_installed().await;

    let container = setup::get_container().await;

    // Check if eza is actually on PATH — the install may have silently failed
    // if the third-party deb.gierens.de repo was unreachable from the container.
    let check = container
        .exec(&["sh", "-c", "command -v eza"])
        .await
        .expect("exec failed");
    if check.exit_code != 0 {
        // Skip: eza was not installed (likely a network/repo access issue in CI).
        return;
    }

    let result = container
        .exec(&["eza", "--version"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "eza");
}

// ---------------------------------------------------------------------------
// java
// ---------------------------------------------------------------------------

/// Install java exactly once for the lifetime of this test binary.
///
/// `bashc install java` installs default-jre and default-jdk via apt.
/// Java reports its version to stderr rather than stdout.
static JAVA_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_java_installed() {
    setup::ensure_apt_updated().await;
    JAVA_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "java"])
                .await
                .expect("install java exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn install_java_exits_zero() {
    ensure_java_installed().await;
}

#[tokio::test]
async fn java_version_works() {
    ensure_java_installed().await;

    let container = setup::get_container().await;
    // java -version prints to stderr, not stdout.
    let result = container
        .exec(&["java", "-version"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stderr_contains(&result, "version");
}

// ---------------------------------------------------------------------------
// postgres
// ---------------------------------------------------------------------------

/// Install postgres exactly once for the lifetime of this test binary.
///
/// `bashc install postgres` installs postgresql and postgresql-contrib via apt.
static POSTGRES_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_postgres_installed() {
    setup::ensure_apt_updated().await;
    POSTGRES_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "postgres"])
                .await
                .expect("install postgres exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn install_postgres_exits_zero() {
    ensure_postgres_installed().await;
}

#[tokio::test]
async fn psql_version_works() {
    ensure_postgres_installed().await;

    let container = setup::get_container().await;
    let result = container
        .exec(&["psql", "--version"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "psql");
}
