use bashc_e2e::assertions::{assert_exit_ok, assert_stdout_contains};

use crate::setup;

async fn assert_dry_run_outcome(tool: &str, expected: &str) {
    let container = setup::get_container().await;
    let result = container
        .exec(&["bashc", "install", "--dry-run", tool])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, expected);
    assert!(!result.stderr.contains("panicked at"));
}

#[tokio::test]
async fn applicable_tools_report_a_successful_plan() {
    for tool in [
        "brew",
        "go",
        "rust",
        "azure",
        "dotnet",
        "neovim",
        "java",
        "github",
        "terraform",
        "postgres",
        "kubectl",
        "ripgrep",
        "bat",
        "fd",
        "eza",
        "shellcheck",
        "nerd-font",
        "javascript",
    ] {
        assert_dry_run_outcome(tool, "planned (dry-run)").await;
    }
}

#[tokio::test]
async fn unsupported_tools_report_not_applicable_without_failing() {
    for tool in ["base", "doas", "docker", "obsidian"] {
        assert_dry_run_outcome(tool, "not applicable").await;
    }
}
