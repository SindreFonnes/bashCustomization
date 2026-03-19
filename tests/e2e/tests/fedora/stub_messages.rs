use bashc_e2e::assertions::assert_stdout_contains;

use crate::setup;

/// Verify that attempting to install ripgrep on Fedora emits a human-readable
/// "not yet implemented" stub message.  Fedora support is stubbed out in the
/// current implementation; the binary must not panic.
#[tokio::test]
async fn install_ripgrep_not_yet_supported() {
    let container = setup::get_container().await;
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

    // Must contain a helpful message about the feature not being available.
    // Actual output: "Fedora/RHEL support not yet implemented. Would install: ripgrep"
    let combined = format!("{}{}", result.stdout, result.stderr);
    let contains_message = combined.contains("not yet supported")
        || combined.contains("not yet implemented")
        || combined.contains("unsupported");
    assert!(
        contains_message,
        "expected output to contain 'not yet supported', 'not yet implemented', or 'unsupported'\n--- stdout ---\n{}\n--- stderr ---\n{}",
        result.stdout, result.stderr
    );

    // Verify the specific stub message text is present.
    assert_stdout_contains(&result, "not yet implemented");
}
