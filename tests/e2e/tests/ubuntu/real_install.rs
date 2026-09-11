use bashc_e2e::assertions::{assert_exit_ok, assert_stdout_contains};

use crate::setup;

/// Verify that apt-get update succeeds and leaves the package index usable.
#[tokio::test]
async fn apt_update_succeeds() {
    setup::ensure_apt_updated().await;
}

/// Verify that `bashc install ripgrep` exits successfully.
#[tokio::test]
async fn install_ripgrep_exits_zero() {
    setup::ensure_ripgrep_installed().await;
}

/// Verify that `rg --version` works after installation.
#[tokio::test]
async fn ripgrep_version_contains_ripgrep() {
    setup::ensure_ripgrep_installed().await;
    let container = setup::get_container().await;
    let result = container
        .exec(&["rg", "--version"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "ripgrep");
}
