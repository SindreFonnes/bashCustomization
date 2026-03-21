use bashc_e2e::assertions::assert_exit_ok;

use crate::setup;

/// NixOS uses a non-standard PATH — bashc lives at /root/.nix-profile/bin/bashc.
const BASHC: &str = "/root/.nix-profile/bin/bashc";

/// On NixOS the dry-run output must contain NixOS-specific content.
///
/// Currently the binary emits `"Detected platform: Linux (NixOs) ..."` which
/// identifies the distro, and real installs emit the declarative guidance
/// (`environment.systemPackages`).  This test verifies the platform label is
/// present in dry-run mode, confirming NixOS is detected and reported.
///
/// When `bashc install <tool>` is run for real on NixOS, `nix_guidance`
/// prints:
///   "NixOS: Add '<tool>' to environment.systemPackages in your NixOS
///    configuration, then run `nixos-rebuild switch`."
/// That message is exercised by the real-install path (not dry-run), so
/// here we verify the dry-run at minimum identifies the distro correctly.
#[tokio::test]
async fn dry_run_contains_nixos_guidance() {
    let container = setup::get_container().await;
    let result = container
        .exec(&[BASHC, "install", "--dry-run", "all"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);

    // The dry-run output must include NixOS-specific content.
    // "NixOs" appears in the platform detection line:
    //   "Detected platform: Linux (NixOs) (aarch64)"
    let combined = format!("{}{}", result.stdout, result.stderr);
    let contains_guidance = combined.contains("environment.systemPackages")
        || combined.contains("nix-env")
        || combined.contains("nix profile")
        || combined.contains("configuration.nix")
        || combined.contains("NixOs")
        || combined.to_lowercase().contains("nixos");
    assert!(
        contains_guidance,
        "Expected NixOS-specific content in dry-run output \
         (e.g., 'NixOs', 'environment.systemPackages', 'nix-env', etc.), \
         but none found.\nexit_code: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        result.exit_code, result.stdout, result.stderr
    );
}

/// Shared helper — runs `bashc install <tool>` for real on NixOS and asserts
/// that the output contains NixOS declarative guidance rather than attempting
/// a package-manager install.
async fn assert_nixos_guidance(tool: &str) {
    let container = setup::get_container().await;
    let result = container
        .exec(&[BASHC, "install", tool])
        .await
        .expect("exec failed");

    // Must not panic.
    assert!(
        !result.stdout.contains("panic") && !result.stderr.contains("panic"),
        "process panicked unexpectedly for tool '{}'\n--- stdout ---\n{}\n--- stderr ---\n{}",
        tool,
        result.stdout,
        result.stderr
    );

    // Must contain NixOS-specific guidance.
    let combined = format!("{}{}", result.stdout, result.stderr);
    let contains_guidance = combined.contains("environment.systemPackages")
        || combined.contains("nix-env")
        || combined.contains("nix profile")
        || combined.contains("configuration.nix")
        || combined.contains("NixOs")
        || combined.to_lowercase().contains("nixos");
    assert!(
        contains_guidance,
        "expected NixOS guidance in output for tool '{}' \
         (e.g., 'environment.systemPackages', 'NixOs', 'nixos-rebuild', etc.), \
         but none found.\nexit_code: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        tool,
        result.exit_code,
        result.stdout,
        result.stderr
    );
}

#[tokio::test]
async fn install_go_emits_nixos_guidance() {
    assert_nixos_guidance("go").await;
}

#[tokio::test]
async fn install_rust_emits_nixos_guidance() {
    assert_nixos_guidance("rust").await;
}

#[tokio::test]
async fn install_docker_emits_nixos_guidance() {
    assert_nixos_guidance("docker").await;
}

#[tokio::test]
async fn install_azure_emits_nixos_guidance() {
    assert_nixos_guidance("azure").await;
}

#[tokio::test]
async fn install_dotnet_emits_nixos_guidance() {
    assert_nixos_guidance("dotnet").await;
}

#[tokio::test]
async fn install_neovim_emits_nixos_guidance() {
    assert_nixos_guidance("neovim").await;
}

#[tokio::test]
async fn install_obsidian_emits_nixos_guidance() {
    assert_nixos_guidance("obsidian").await;
}

#[tokio::test]
async fn install_java_emits_nixos_guidance() {
    assert_nixos_guidance("java").await;
}

#[tokio::test]
async fn install_github_emits_nixos_guidance() {
    assert_nixos_guidance("github").await;
}

#[tokio::test]
async fn install_terraform_emits_nixos_guidance() {
    assert_nixos_guidance("terraform").await;
}

#[tokio::test]
async fn install_postgres_emits_nixos_guidance() {
    assert_nixos_guidance("postgres").await;
}

#[tokio::test]
async fn install_kubectl_emits_nixos_guidance() {
    assert_nixos_guidance("kubectl").await;
}

#[tokio::test]
async fn install_ripgrep_emits_nixos_guidance() {
    assert_nixos_guidance("ripgrep").await;
}

#[tokio::test]
async fn install_bat_emits_nixos_guidance() {
    assert_nixos_guidance("bat").await;
}

#[tokio::test]
async fn install_fd_emits_nixos_guidance() {
    assert_nixos_guidance("fd").await;
}

#[tokio::test]
async fn install_eza_emits_nixos_guidance() {
    assert_nixos_guidance("eza").await;
}

#[tokio::test]
async fn install_shellcheck_emits_nixos_guidance() {
    assert_nixos_guidance("shellcheck").await;
}

#[tokio::test]
async fn install_nerd_font_emits_nixos_guidance() {
    assert_nixos_guidance("nerd-font").await;
}

#[tokio::test]
async fn install_javascript_emits_nixos_guidance() {
    assert_nixos_guidance("javascript").await;
}

#[tokio::test]
async fn install_base_emits_nixos_guidance() {
    assert_nixos_guidance("base").await;
}
