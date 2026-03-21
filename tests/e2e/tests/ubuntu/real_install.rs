use bashc_e2e::assertions::{assert_exit_ok, assert_stdout_contains};

use crate::setup;

/// Verify that apt-get update succeeds.  The container init already warms the
/// apt cache, so this test simply confirms the package index is usable.
#[tokio::test]
async fn apt_update_succeeds() {
    let _lock = setup::apt_install_lock().await;
    let container = setup::get_container().await;
    let result = container
        .exec(&["apt-get", "update", "-qq"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
}

/// Verify that `bashc install ripgrep` exits 0.  Ripgrep is already
/// pre-installed by the container init, so this is an idempotent reinstall.
#[tokio::test]
async fn install_ripgrep_exits_zero() {
    let container = setup::get_container().await;
    // apt cache is pre-warmed and ripgrep is pre-installed by container init.
    let result = container
        .exec(&["bashc", "install", "ripgrep"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
}

/// Verify that `rg --version` works after install.  The container init has
/// already run `bashc install ripgrep`, so we just need to confirm the binary
/// is reachable and outputs the expected string.
#[tokio::test]
async fn ripgrep_version_contains_ripgrep() {
    let container = setup::get_container().await;
    // ripgrep is pre-installed by container init — just verify the binary.
    let result = container
        .exec(&["rg", "--version"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "ripgrep");
}
