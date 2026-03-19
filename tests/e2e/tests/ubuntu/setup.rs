use anyhow::{Context, Result};
use bashc_e2e::container::TestContainer;
use bashc_e2e::distro::{docker_dir, repo_root};
use bollard::Docker;
use tokio::sync::{Mutex, OnceCell};

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

/// Global mutex that serialises all apt-get install invocations.
///
/// apt holds exclusive file locks on `/var/lib/dpkg/lock-frontend` and
/// `/var/lib/apt/lists/lock`. If two `bashc install` commands run at the same
/// time inside the shared container they will race on those locks and one will
/// fail with exit code 100. Holding this mutex around every install call
/// guarantees at most one apt operation is in-flight at any moment.
static APT_INSTALL_LOCK: Mutex<()> = Mutex::const_new(());

/// Shared apt-get update guard — the container init already warms the apt
/// cache, so this is a no-op after init. Kept for API symmetry with the
/// Debian setup so fast_installs.rs can call `setup::ensure_apt_updated()`
/// without branching on distro.
static APT_UPDATED: OnceCell<()> = OnceCell::const_new();

/// Acquire the global apt install lock and return the guard.
///
/// Hold the returned guard for the duration of any `apt-get install` or
/// `bashc install` call to prevent concurrent apt invocations inside the
/// shared container.
pub async fn apt_install_lock() -> tokio::sync::MutexGuard<'static, ()> {
    APT_INSTALL_LOCK.lock().await
}

/// Ensure the apt cache has been warmed. The Ubuntu container init already
/// runs `apt-get update`, so this function just ensures the container is
/// started (which triggers the init) and marks the update as complete.
pub async fn ensure_apt_updated() {
    APT_UPDATED
        .get_or_init(|| async {
            // Calling get_container() ensures init_container() has run, which
            // already executed apt-get update as part of container setup.
            get_container().await;
        })
        .await;
}

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
