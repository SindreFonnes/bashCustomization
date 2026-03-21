use bashc_e2e::assertions::{assert_command_exists, assert_exit_ok, assert_file_contains};

use crate::setup;

/// Verify that `bashc install doas` exits 0.  doas is pre-installed by the
/// container init, so this is an idempotent reinstall.
#[tokio::test]
async fn install_doas_exits_zero() {
    let container = setup::get_container().await;
    let result = container
        .exec(&["bashc", "install", "doas"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
}

/// Verify `doas` is reachable in the container PATH after install.
#[tokio::test]
async fn doas_command_exists_after_install() {
    let container = setup::get_container().await;
    // doas is pre-installed by container init.
    assert_command_exists(container, "doas")
        .await
        .expect("assert_command_exists failed");
}

/// Verify the doas configuration file contains "permit persist" after install.
#[tokio::test]
async fn doas_conf_contains_permit_persist() {
    let container = setup::get_container().await;
    // doas is pre-installed by container init; config file must exist.
    assert_file_contains(container, "/etc/doas.d/doas.conf", "permit persist")
        .await
        .expect("assert_file_contains failed");
}

/// Verify a dry-run of all installs still succeeds after doas is installed.
#[tokio::test]
async fn dry_run_all_succeeds_after_doas_installed() {
    let container = setup::get_container().await;
    // doas is pre-installed by container init.
    let result = container
        .exec(&["bashc", "install", "--dry-run", "all"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
}
