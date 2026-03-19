use bashc_e2e::assertions::{assert_exit_ok, assert_stdout_contains, assert_stdout_not_contains};

use crate::setup;

#[tokio::test]
async fn dry_run_exits_zero() {
    let container = setup::get_container().await;
    let result = container
        .exec(&["bashc", "install", "--dry-run", "all"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
}

#[tokio::test]
async fn dry_run_detects_debian_family() {
    // Ubuntu is detected as Debian family via ID_LIKE=debian in /etc/os-release.
    let container = setup::get_container().await;
    let result = container
        .exec(&["bashc", "install", "--dry-run", "all"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "Debian");
}

#[tokio::test]
async fn dry_run_no_panics() {
    let container = setup::get_container().await;
    let result = container
        .exec(&["bashc", "install", "--dry-run", "all"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_not_contains(&result, "panic");
    assert_stdout_not_contains(&result, "RUST_BACKTRACE");
}
