use std::path::{Path, PathBuf};

/// Configuration for a Linux distro under test.
#[derive(Debug, Clone)]
pub struct DistroConfig {
    /// Docker image tag, e.g. `"bashc-test-debian"`.
    pub image_tag: String,
    /// Dockerfile filename relative to `tests/docker/`, e.g. `"Dockerfile.debian"`.
    pub dockerfile: String,
    /// Substring expected in `bashc` distro-detection output.
    pub expected_distro_label: String,
    /// Whether to skip this distro on aarch64 (ARM64).
    pub skip_on_arm64: bool,
}

impl DistroConfig {
    /// Returns `true` if this distro should be skipped on the current architecture.
    pub fn should_skip(&self) -> bool {
        self.skip_on_arm64 && std::env::consts::ARCH == "aarch64"
    }
}

/// Pre-defined distro configurations.
pub fn all_distros() -> Vec<DistroConfig> {
    vec![
        DistroConfig {
            image_tag: "bashc-test-debian".into(),
            dockerfile: "Dockerfile.debian".into(),
            expected_distro_label: "Debian".into(),
            skip_on_arm64: false,
        },
        DistroConfig {
            image_tag: "bashc-test-ubuntu".into(),
            dockerfile: "Dockerfile.ubuntu".into(),
            expected_distro_label: "Debian".into(), // Ubuntu detected as Debian family
            skip_on_arm64: false,
        },
        DistroConfig {
            image_tag: "bashc-test-fedora".into(),
            dockerfile: "Dockerfile.fedora".into(),
            expected_distro_label: "Fedora".into(),
            skip_on_arm64: false,
        },
        DistroConfig {
            image_tag: "bashc-test-arch".into(),
            dockerfile: "Dockerfile.arch".into(),
            expected_distro_label: "Arch".into(),
            skip_on_arm64: true,
        },
        DistroConfig {
            image_tag: "bashc-test-alpine".into(),
            dockerfile: "Dockerfile.alpine".into(),
            expected_distro_label: "Alpine".into(),
            skip_on_arm64: false,
        },
        DistroConfig {
            image_tag: "bashc-test-nixos".into(),
            dockerfile: "Dockerfile.nixos".into(),
            expected_distro_label: "NixOS".into(),
            skip_on_arm64: false,
        },
    ]
}

/// Resolve the repository root from `CARGO_MANIFEST_DIR`.
///
/// The e2e crate lives at `<repo_root>/tests/e2e/`, so the repo root is two
/// levels up from `CARGO_MANIFEST_DIR`.
pub fn repo_root() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set -- must be run via `cargo`");
    Path::new(&manifest_dir)
        .join("..")
        .join("..")
        .canonicalize()
        .expect("failed to canonicalize repo root path")
}

/// Path to `tests/docker/` within the repository.
pub fn docker_dir() -> PathBuf {
    repo_root().join("tests").join("docker")
}
