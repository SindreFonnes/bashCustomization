use anyhow::{Context, Result};
use bashc_e2e::container::TestContainer;
use bashc_e2e::distro::{docker_dir, repo_root};
use bollard::Docker;
use tokio::sync::OnceCell;

const BUILDER_IMAGE_TAG: &str = "bashc-builder";
const UBUNTU_IMAGE_TAG: &str = "bashc-test-ubuntu";
const CONTAINER_NAME: &str = "bashc-e2e-ubuntu";

/// Shared container for all Ubuntu tests. Initialized once per test binary.
///
/// Container cleanup: the `create_and_start` function removes any leftover
/// container with the same name before creating a new one. This means each
/// test run starts clean. The container is left running after tests finish
/// (stopping it mid-run would break concurrent tests). It gets cleaned up
/// at the start of the next run.
static CONTAINER: OnceCell<TestContainer> = OnceCell::const_new();

/// Get or initialize the shared Ubuntu test container.
pub async fn get_container() -> &'static TestContainer {
    CONTAINER
        .get_or_init(|| async {
            init_container()
                .await
                .expect("failed to initialize Ubuntu test container")
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

    // Step 3: Build the Ubuntu test image.
    // Build context is tests/docker/ because Dockerfile.ubuntu COPYs bashc from that dir.
    println!("==> Building Ubuntu test image ({UBUNTU_IMAGE_TAG})...");
    TestContainer::build_image(
        &docker,
        UBUNTU_IMAGE_TAG,
        "Dockerfile.ubuntu",
        &docker_path,
    )
    .await
    .context("building Ubuntu test image")?;
    println!("==> Ubuntu image ready.");

    // Step 4: Create and start the test container.
    // Note: create_and_start removes any leftover container with the same name.
    println!("==> Starting container ({CONTAINER_NAME})...");
    let container =
        TestContainer::create_and_start(&docker, UBUNTU_IMAGE_TAG, CONTAINER_NAME)
            .await
            .context("creating and starting Ubuntu container")?;
    println!("==> Container running.");

    // Step 5: Warm the apt cache once so that concurrent tests can install
    // packages without racing on the apt lock.
    println!("==> Running apt-get update to warm package cache...");
    let update = container
        .exec(&["apt-get", "update", "-qq"])
        .await
        .context("warming apt cache")?;
    if update.exit_code != 0 {
        anyhow::bail!(
            "apt-get update failed (exit {}): {}",
            update.exit_code,
            update.stderr
        );
    }
    println!("==> Package cache ready.");

    // Step 6: Pre-install ripgrep so concurrent tests that verify rg --version
    // do not race with each other on the apt install lock.
    println!("==> Pre-installing ripgrep...");
    let install = container
        .exec(&["bashc", "install", "ripgrep"])
        .await
        .context("pre-installing ripgrep")?;
    if install.exit_code != 0 {
        anyhow::bail!(
            "pre-install ripgrep failed (exit {}): stdout={} stderr={}",
            install.exit_code,
            install.stdout,
            install.stderr
        );
    }
    println!("==> ripgrep pre-installed.");

    Ok(container)
}
