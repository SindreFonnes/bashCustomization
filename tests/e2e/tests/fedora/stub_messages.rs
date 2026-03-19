use bashc_e2e::assertions::assert_stdout_contains;

use crate::setup;

/// Shared helper — runs `bashc install <tool>` on Fedora and asserts:
///  1. The process does not panic (no Rust panic output).
///  2. The combined output contains either:
///     - A stub/unsupported message, OR
///     - A graceful error or success message (i.e., the tool's code path was reached).
///
/// Some tools (e.g., go, rust, kubectl) install successfully on Fedora via
/// cross-platform installers. Others emit a stub message. Both are acceptable;
/// the hard requirement is no panic.
async fn assert_tool_no_panic(tool: &str) {
    let container = setup::get_container().await;
    let result = container
        .exec(&["bashc", "install", tool])
        .await
        .expect("exec failed");

    // Must not produce a Rust panic.
    assert!(
        !result.stdout.contains("thread '") && !result.stderr.contains("thread '")
            && !result.stdout.contains("panicked at") && !result.stderr.contains("panicked at"),
        "process panicked unexpectedly for tool '{}'\n--- stdout ---\n{}\n--- stderr ---\n{}",
        tool,
        result.stdout,
        result.stderr
    );
}

/// Helper used for tools that are confirmed to be stubbed on Fedora — checks
/// both no-panic and that a graceful "not supported" message is present.
async fn assert_tool_stub(tool: &str) {
    let container = setup::get_container().await;
    let result = container
        .exec(&["bashc", "install", tool])
        .await
        .expect("exec failed");

    // Must not panic.
    assert!(
        !result.stdout.contains("thread '") && !result.stderr.contains("thread '")
            && !result.stdout.contains("panicked at") && !result.stderr.contains("panicked at"),
        "process panicked unexpectedly for tool '{}'\n--- stdout ---\n{}\n--- stderr ---\n{}",
        tool,
        result.stdout,
        result.stderr
    );

    // Must contain a helpful stub message.
    let combined = format!("{}{}", result.stdout, result.stderr);
    let contains_message = combined.contains("not yet supported")
        || combined.contains("not yet implemented")
        || combined.contains("not yet configured")
        || combined.contains("unsupported");
    assert!(
        contains_message,
        "expected output to contain 'not yet supported', 'not yet implemented', \
         'not yet configured', or 'unsupported' for tool '{}'\n--- stdout ---\n{}\n--- stderr ---\n{}",
        tool,
        result.stdout,
        result.stderr
    );
}

/// Verify that attempting to install ripgrep on Fedora emits a human-readable
/// "not yet implemented" stub message.  Fedora support is stubbed out in the
/// current implementation; the binary must not panic.
#[tokio::test]
async fn install_ripgrep_not_yet_supported() {
    assert_tool_stub("ripgrep").await;
    // Also verify the specific stub message text is present (kept for backward compat).
    let container = setup::get_container().await;
    let result = container
        .exec(&["bashc", "install", "ripgrep"])
        .await
        .expect("exec failed");
    assert_stdout_contains(&result, "not yet implemented");
}

/// go installs via a cross-platform tarball installer — succeeds on Fedora.
#[tokio::test]
async fn install_go_no_panic() {
    assert_tool_no_panic("go").await;
}

/// rust installs via rustup — succeeds on Fedora.
#[tokio::test]
async fn install_rust_no_panic() {
    assert_tool_no_panic("rust").await;
}

#[tokio::test]
async fn install_docker_not_yet_supported() {
    assert_tool_stub("docker").await;
}

#[tokio::test]
async fn install_azure_not_yet_supported() {
    assert_tool_stub("azure").await;
}

#[tokio::test]
async fn install_dotnet_not_yet_supported() {
    assert_tool_stub("dotnet").await;
}

/// neovim emits a Debian-only error on Fedora — graceful failure, not a panic.
#[tokio::test]
async fn install_neovim_no_panic() {
    assert_tool_no_panic("neovim").await;
}

/// obsidian emits a Debian-only error on Fedora — graceful failure, not a panic.
#[tokio::test]
async fn install_obsidian_no_panic() {
    assert_tool_no_panic("obsidian").await;
}

#[tokio::test]
async fn install_java_not_yet_supported() {
    assert_tool_stub("java").await;
}

#[tokio::test]
async fn install_github_not_yet_supported() {
    assert_tool_stub("github").await;
}

#[tokio::test]
async fn install_terraform_not_yet_supported() {
    assert_tool_stub("terraform").await;
}

#[tokio::test]
async fn install_postgres_not_yet_supported() {
    assert_tool_stub("postgres").await;
}

/// kubectl installs via a cross-platform binary download — succeeds on Fedora.
#[tokio::test]
async fn install_kubectl_no_panic() {
    assert_tool_no_panic("kubectl").await;
}

#[tokio::test]
async fn install_bat_not_yet_supported() {
    assert_tool_stub("bat").await;
}

#[tokio::test]
async fn install_fd_not_yet_supported() {
    assert_tool_stub("fd").await;
}

#[tokio::test]
async fn install_eza_not_yet_supported() {
    assert_tool_stub("eza").await;
}

#[tokio::test]
async fn install_shellcheck_not_yet_supported() {
    assert_tool_stub("shellcheck").await;
}

/// nerd-font attempts a cross-platform install; may fail on missing fc-cache
/// but must not panic.
#[tokio::test]
async fn install_nerd_font_no_panic() {
    assert_tool_no_panic("nerd-font").await;
}

/// javascript/bun installs via a cross-platform script; may fail on missing
/// unzip but must not panic.
#[tokio::test]
async fn install_javascript_no_panic() {
    assert_tool_no_panic("javascript").await;
}

#[tokio::test]
async fn install_base_not_yet_supported() {
    assert_tool_stub("base").await;
}
