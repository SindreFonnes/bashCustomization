use bashc_e2e::assertions::{assert_exit_ok, assert_stdout_contains};
use tokio::sync::OnceCell;

use crate::setup;

/// Install ripgrep exactly once for the lifetime of this test binary.
///
/// `apt-get update` is guaranteed to complete before the install runs.
/// The global `APT_INSTALL_LOCK` in `setup` serialises this against any
/// concurrent `bashc install` calls in other modules (e.g. `symlinks`).
static RIPGREP_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_ripgrep_installed() {
    setup::ensure_apt_updated().await;
    RIPGREP_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "ripgrep"])
                .await
                .expect("install ripgrep exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn apt_update_succeeds() {
    setup::ensure_apt_updated().await;
}

#[tokio::test]
async fn install_ripgrep_exits_zero() {
    ensure_ripgrep_installed().await;
}

#[tokio::test]
async fn ripgrep_version_contains_ripgrep() {
    ensure_ripgrep_installed().await;

    let container = setup::get_container().await;
    let result = container
        .exec(&["rg", "--version"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "ripgrep");
}
