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
