use bashc_e2e::assertions::{assert_exit_ok, assert_stdout_not_contains};

use crate::setup;

/// NixOS uses a non-standard PATH — bashc lives at /root/.nix-profile/bin/bashc.
const BASHC: &str = "/root/.nix-profile/bin/bashc";

#[tokio::test]
async fn dry_run_exits_zero() {
    let container = setup::get_container().await;
    let result = container
        .exec(&[BASHC, "install", "--dry-run", "all"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
}

#[tokio::test]
async fn dry_run_detects_nixos() {
    // Check for "NixOS" or "nixos" (case insensitive).
    let container = setup::get_container().await;
    let result = container
        .exec(&[BASHC, "install", "--dry-run", "all"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);

    let stdout_lower = result.stdout.to_lowercase();
    assert!(
        stdout_lower.contains("nixos"),
        "Expected stdout to contain 'NixOS' or 'nixos' (case insensitive), but it did not.\nexit_code: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        result.exit_code,
        result.stdout,
        result.stderr
    );
}

#[tokio::test]
async fn dry_run_no_panics() {
    let container = setup::get_container().await;
    let result = container
        .exec(&[BASHC, "install", "--dry-run", "all"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_not_contains(&result, "panic");
    assert_stdout_not_contains(&result, "RUST_BACKTRACE");
}
