use crate::container::{ExecResult, TestContainer};
use anyhow::Result;

// ---------------------------------------------------------------------------
// Pure assertion helpers (operate on ExecResult)
// ---------------------------------------------------------------------------

/// Panic if the exit code is not 0. Prints full output context on failure.
pub fn assert_exit_ok(result: &ExecResult) {
    if result.exit_code != 0 {
        panic!(
            "Expected exit code 0, got {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            result.exit_code, result.stdout, result.stderr
        );
    }
}

/// Panic if the exit code is 0. Prints full output context on failure.
pub fn assert_exit_err(result: &ExecResult) {
    if result.exit_code == 0 {
        panic!(
            "Expected non-zero exit code, got 0\n--- stdout ---\n{}\n--- stderr ---\n{}",
            result.stdout, result.stderr
        );
    }
}

/// Panic if `text` is not found in stdout.
pub fn assert_stdout_contains(result: &ExecResult, text: &str) {
    if !result.stdout.contains(text) {
        panic!(
            "Expected stdout to contain {:?}, but it did not.\nexit_code: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            text, result.exit_code, result.stdout, result.stderr
        );
    }
}

/// Panic if `text` IS found in stdout.
pub fn assert_stdout_not_contains(result: &ExecResult, text: &str) {
    if result.stdout.contains(text) {
        panic!(
            "Expected stdout NOT to contain {:?}, but it did.\nexit_code: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            text, result.exit_code, result.stdout, result.stderr
        );
    }
}

/// Panic if `text` is not found in stderr.
pub fn assert_stderr_contains(result: &ExecResult, text: &str) {
    if !result.stderr.contains(text) {
        panic!(
            "Expected stderr to contain {:?}, but it did not.\nexit_code: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            text, result.exit_code, result.stdout, result.stderr
        );
    }
}

// ---------------------------------------------------------------------------
// Container-level assertion helpers (run exec + assert)
// ---------------------------------------------------------------------------

/// Assert that a command/binary exists in the container's PATH.
pub async fn assert_command_exists(container: &TestContainer, name: &str) -> Result<()> {
    let result = container.exec(&["sh", "-c", &format!("command -v {name}")]).await?;
    if result.exit_code != 0 {
        panic!(
            "Expected command {:?} to exist in container, but `command -v` failed.\nexit_code: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            name, result.exit_code, result.stdout, result.stderr
        );
    }
    Ok(())
}

/// Assert that a file in the container contains the given content substring.
pub async fn assert_file_contains(
    container: &TestContainer,
    path: &str,
    content: &str,
) -> Result<()> {
    let result = container.exec(&["cat", path]).await?;
    if result.exit_code != 0 {
        panic!(
            "Failed to cat {:?} in container.\nexit_code: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            path, result.exit_code, result.stdout, result.stderr
        );
    }
    if !result.stdout.contains(content) {
        panic!(
            "Expected file {:?} to contain {:?}, but it did not.\nexit_code: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            path, content, result.exit_code, result.stdout, result.stderr
        );
    }
    Ok(())
}
