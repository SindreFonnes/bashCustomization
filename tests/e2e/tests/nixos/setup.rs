use anyhow::{Context, Result};
use bashc_e2e::container::TestContainer;
use bashc_e2e::distro::{docker_dir, repo_root};
use bollard::Docker;
use tokio::sync::OnceCell;

const BUILDER_IMAGE_TAG: &str = "bashc-builder";
const NIXOS_IMAGE_TAG: &str = "bashc-test-nixos";
const CONTAINER_NAME: &str = "bashc-e2e-nixos";

/// Shared container for all NixOS tests. Initialized once per test binary.
///
/// Container cleanup: the `create_and_start` function removes any leftover
/// container with the same name before creating a new one. This means each
/// test run starts clean. The container is left running after tests finish
/// (stopping it mid-run would break concurrent tests). It gets cleaned up
/// at the start of the next run.
///
/// NOTE: NixOS uses a non-standard PATH — bashc is installed at
/// `/root/.nix-profile/bin/bashc`.
static CONTAINER: OnceCell<TestContainer> = OnceCell::const_new();

/// Get or initialize the shared NixOS test container.
pub async fn get_container() -> &'static TestContainer {
    CONTAINER
        .get_or_init(|| async {
            init_container()
                .await
                .expect("failed to initialize NixOS test container")
        })
        .await
}

async fn init_container() -> Result<TestContainer> {
    let docker =
        Docker::connect_with_local_defaults().context("connecting to Docker daemon")?;

    let repo = repo_root();
    let docker_path = docker_dir();

    // Step 1: Build the builder image (compiles bashc as a musl binary).
    // Build context is the repo root because Dockerfile.builder COPYs rust/Cargo.toml etc.
    println!("==> Building builder image ({BUILDER_IMAGE_TAG})...");
    TestContainer::build_image(
        &docker,
        BUILDER_IMAGE_TAG,
        "tests/docker/Dockerfile.builder",
        &repo,
    )
    .await
    .context("building builder image")?;
    println!("==> Builder image ready.");

    // Step 2: Extract the bashc binary from the builder image into tests/docker/bashc.
    let binary_dest = docker_path.join("bashc");
    println!("==> Extracting bashc binary to {}...", binary_dest.display());
    TestContainer::extract_binary(&docker, BUILDER_IMAGE_TAG, &binary_dest)
        .await
        .context("extracting bashc binary")?;
    println!("==> Binary extracted.");

    // Step 3: Build the NixOS test image.
    // Build context is tests/docker/ because Dockerfile.nixos COPYs bashc from that dir.
    println!("==> Building NixOS test image ({NIXOS_IMAGE_TAG})...");
    TestContainer::build_image(
        &docker,
        NIXOS_IMAGE_TAG,
        "Dockerfile.nixos",
        &docker_path,
    )
    .await
    .context("building NixOS test image")?;
    println!("==> NixOS image ready.");

    // Step 4: Create and start the test container.
    // Note: create_and_start removes any leftover container with the same name.
    println!("==> Starting container ({CONTAINER_NAME})...");
    let container =
        TestContainer::create_and_start(&docker, NIXOS_IMAGE_TAG, CONTAINER_NAME)
            .await
            .context("creating and starting NixOS container")?;
    println!("==> Container running.");

    Ok(container)
}
