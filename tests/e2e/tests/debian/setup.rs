use anyhow::{Context, Result};
use bashc_e2e::container::TestContainer;
use bashc_e2e::distro::{docker_dir, repo_root};
use bollard::Docker;
use tokio::sync::{Mutex, OnceCell};

const BUILDER_IMAGE_TAG: &str = "bashc-builder";
const DEBIAN_IMAGE_TAG: &str = "bashc-test-debian";
const CONTAINER_NAME: &str = "bashc-e2e-debian";

/// Shared container for all Debian tests. Initialized once per test binary.
///
/// Container cleanup: the `create_and_start` function removes any leftover
/// container with the same name before creating a new one. This means each
/// test run starts clean. The container is left running after tests finish
/// (stopping it mid-run would break concurrent tests). It gets cleaned up
/// at the start of the next run.
static CONTAINER: OnceCell<TestContainer> = OnceCell::const_new();

/// Shared apt-get update guard — runs exactly once per test binary invocation.
///
/// Because all tests in this binary share a single process and a single
/// container, running `apt-get update` concurrently from multiple test
/// modules would cause apt lock contention. Centralising it here ensures it
/// executes at most once, regardless of how many modules call it.
static APT_UPDATED: OnceCell<()> = OnceCell::const_new();

/// Global mutex that serialises all apt-get install invocations.
///
/// apt holds exclusive file locks on `/var/lib/dpkg/lock-frontend` and
/// `/var/lib/apt/lists/lock`. If two `bashc install` commands run at the same
/// time inside the shared container they will race on those locks and one will
/// fail with exit code 100. Holding this mutex around every install call
/// guarantees at most one apt operation is in-flight at any moment.
static APT_INSTALL_LOCK: Mutex<()> = Mutex::const_new(());

/// Get or initialize the shared Debian test container.
pub async fn get_container() -> &'static TestContainer {
    CONTAINER
        .get_or_init(|| async {
            init_container()
                .await
                .expect("failed to initialize Debian test container")
        })
        .await
}

/// Acquire the global apt install lock and return the guard.
///
/// Hold the returned guard for the duration of any `apt-get install` or
/// `bashc install` call to prevent concurrent apt invocations inside the
/// shared container.
pub async fn apt_install_lock() -> tokio::sync::MutexGuard<'static, ()> {
    APT_INSTALL_LOCK.lock().await
}

/// Run `apt-get update -qq` exactly once across the entire test binary.
///
/// Multiple concurrent callers are safe: `OnceCell::get_or_init` serialises
/// them so only one invocation of `apt-get update` ever runs.
pub async fn ensure_apt_updated() {
    APT_UPDATED
        .get_or_init(|| async {
            let container = get_container().await;
            let result = container
                .exec(&["apt-get", "update", "-qq"])
                .await
                .expect("apt-get update exec failed");
            if result.exit_code != 0 {
                panic!(
                    "apt-get update failed with exit code {}\n--- stderr ---\n{}",
                    result.exit_code, result.stderr
                );
            }
        })
        .await;
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

    // Step 3: Build the Debian test image.
    // Build context is tests/docker/ because Dockerfile.debian COPYs bashc from that dir.
    println!("==> Building Debian test image ({DEBIAN_IMAGE_TAG})...");
    TestContainer::build_image(
        &docker,
        DEBIAN_IMAGE_TAG,
        "Dockerfile.debian",
        &docker_path,
    )
    .await
    .context("building Debian test image")?;
    println!("==> Debian image ready.");

    // Step 4: Create and start the test container.
    // Note: create_and_start removes any leftover container with the same name.
    println!("==> Starting container ({CONTAINER_NAME})...");
    let container =
        TestContainer::create_and_start(&docker, DEBIAN_IMAGE_TAG, CONTAINER_NAME)
            .await
            .context("creating and starting Debian container")?;
    println!("==> Container running.");

    Ok(container)
}
