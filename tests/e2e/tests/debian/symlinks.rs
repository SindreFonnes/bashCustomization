use bashc_e2e::assertions::{assert_exit_ok, assert_stdout_contains};
use tokio::sync::OnceCell;

use crate::setup;

/// Install bat exactly once for the lifetime of this test binary.
///
/// On Debian, `apt install bat` places the binary at `/usr/bin/batcat`.
/// `bashc install bat` is expected to create a `/usr/local/bin/bat` symlink
/// so callers can invoke `bat` directly.
static BAT_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_bat_installed() {
    setup::ensure_apt_updated().await;
    BAT_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "bat"])
                .await
                .expect("install bat exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

/// Install fd exactly once for the lifetime of this test binary.
///
/// On Debian, `apt install fd-find` places the binary at `/usr/bin/fdfind`.
/// `bashc install fd` is expected to create a `/usr/local/bin/fd` symlink
/// so callers can invoke `fd` directly.
static FD_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_fd_installed() {
    setup::ensure_apt_updated().await;
    FD_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "fd"])
                .await
                .expect("install fd exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn install_bat_exits_zero() {
    ensure_bat_installed().await;
}

#[tokio::test]
async fn bat_binary_works_not_batcat() {
    ensure_bat_installed().await;

    let container = setup::get_container().await;
    // The symlink is placed in ~/.local/bin/bat. Use sh -c so that PATH
    // expansion (including HOME) resolves correctly inside the container.
    let result = container
        .exec(&["sh", "-c", "PATH=\"$HOME/.local/bin:$PATH\" bat --version"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "bat");
}

#[tokio::test]
async fn install_fd_exits_zero() {
    ensure_fd_installed().await;
}

#[tokio::test]
async fn fd_binary_works_not_fdfind() {
    ensure_fd_installed().await;

    let container = setup::get_container().await;
    // The symlink is placed in ~/.local/bin/fd. Use sh -c so that PATH
    // expansion (including HOME) resolves correctly inside the container.
    let result = container
        .exec(&["sh", "-c", "PATH=\"$HOME/.local/bin:$PATH\" fd --version"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "fd");
}
