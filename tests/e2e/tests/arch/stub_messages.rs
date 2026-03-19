use crate::setup;

/// Verify that attempting to install ripgrep on Arch returns a non-zero exit
/// code and emits a human-readable "not yet supported" message rather than
/// panicking.  Arch support is stubbed; the binary must fail gracefully.
#[tokio::test]
async fn install_ripgrep_not_yet_supported() {
    // archlinux:latest has no arm64 variant — skip on aarch64 hosts.
    let Some(container) = setup::get_container().await else {
        eprintln!("Skipping Arch test on aarch64 host");
        return;
    };

    let result = container
        .exec(&["bashc", "install", "ripgrep"])
        .await
        .expect("exec failed");

    // Must not panic.
    assert!(
        !result.stdout.contains("panic") && !result.stderr.contains("panic"),
        "process panicked unexpectedly\n--- stdout ---\n{}\n--- stderr ---\n{}",
        result.stdout,
        result.stderr
    );

    // Must return non-zero exit code.
    assert_ne!(
        result.exit_code, 0,
        "expected non-zero exit code for unsupported distro, got 0\n--- stdout ---\n{}\n--- stderr ---\n{}",
        result.stdout, result.stderr
    );

    // Must contain a helpful message about the feature not being available.
    let combined = format!("{}{}", result.stdout, result.stderr);
    let contains_message = combined.contains("not yet supported")
        || combined.contains("not yet implemented")
        || combined.contains("unsupported");
    assert!(
        contains_message,
        "expected output to contain 'not yet supported', 'not yet implemented', or 'unsupported'\n--- stdout ---\n{}\n--- stderr ---\n{}",
        result.stdout, result.stderr
    );
}
