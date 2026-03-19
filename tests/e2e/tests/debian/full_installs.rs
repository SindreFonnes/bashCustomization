/// Full-tier install tests — only run when `FULL_INSTALL_TESTS=1` is set.
///
/// These tests install heavy tools (compilers, runtimes, CLIs) that take
/// significant time and network bandwidth. Gate each test behind the env var
/// check so the CI fast tier remains quick.
use bashc_e2e::assertions::{assert_exit_ok, assert_stdout_contains};
use tokio::sync::OnceCell;

use crate::setup;

// ---------------------------------------------------------------------------
// go
// ---------------------------------------------------------------------------

static GO_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_go_installed() {
    setup::ensure_apt_updated().await;
    GO_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "go"])
                .await
                .expect("install go exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn install_go_exits_zero() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_go_installed().await;
}

#[tokio::test]
async fn go_version_works() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_go_installed().await;

    let container = setup::get_container().await;
    // Go is installed to /usr/local/go/bin; use sh -c so PATH expansion works.
    let result = container
        .exec(&["sh", "-c", "PATH=\"/usr/local/go/bin:$PATH\" go version"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "go version");
}

// ---------------------------------------------------------------------------
// rust
// ---------------------------------------------------------------------------

static RUST_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_rust_installed() {
    setup::ensure_apt_updated().await;
    RUST_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "rust"])
                .await
                .expect("install rust exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn install_rust_exits_zero() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_rust_installed().await;
}

#[tokio::test]
async fn rustc_version_works() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_rust_installed().await;

    let container = setup::get_container().await;
    // rustup installs to ~/.cargo/bin; source the env or add to PATH.
    let result = container
        .exec(&["sh", "-c", "PATH=\"$HOME/.cargo/bin:$PATH\" rustc --version"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "rustc");
}

// ---------------------------------------------------------------------------
// kubectl
// ---------------------------------------------------------------------------

static KUBECTL_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_kubectl_installed() {
    setup::ensure_apt_updated().await;
    KUBECTL_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "kubectl"])
                .await
                .expect("install kubectl exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn install_kubectl_exits_zero() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_kubectl_installed().await;
}

#[tokio::test]
async fn kubectl_client_version_works() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_kubectl_installed().await;

    let container = setup::get_container().await;
    let result = container
        .exec(&["kubectl", "version", "--client"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "Client Version");
}

// ---------------------------------------------------------------------------
// docker
// ---------------------------------------------------------------------------

static DOCKER_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_docker_installed() {
    setup::ensure_apt_updated().await;
    DOCKER_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "docker"])
                .await
                .expect("install docker exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn install_docker_exits_zero() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_docker_installed().await;
}

#[tokio::test]
async fn docker_version_binary_works() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_docker_installed().await;

    let container = setup::get_container().await;
    // The docker daemon won't run inside a container, but the binary should exist.
    let result = container
        .exec(&["docker", "--version"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "Docker");
}

// ---------------------------------------------------------------------------
// azure
// ---------------------------------------------------------------------------

static AZURE_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_azure_installed() {
    setup::ensure_apt_updated().await;
    AZURE_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "azure"])
                .await
                .expect("install azure exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn install_azure_exits_zero() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_azure_installed().await;
}

#[tokio::test]
async fn az_version_works() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_azure_installed().await;

    let container = setup::get_container().await;
    let result = container
        .exec(&["az", "version"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "azure-cli");
}

// ---------------------------------------------------------------------------
// dotnet
// ---------------------------------------------------------------------------

static DOTNET_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_dotnet_installed() {
    setup::ensure_apt_updated().await;
    DOTNET_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "dotnet"])
                .await
                .expect("install dotnet exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn install_dotnet_exits_zero() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_dotnet_installed().await;
}

#[tokio::test]
async fn dotnet_version_works() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_dotnet_installed().await;

    let container = setup::get_container().await;
    let result = container
        .exec(&["dotnet", "--version"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    // dotnet --version prints just the version number, e.g. "8.0.100"
    // Verify we got a non-empty response by checking exit code only.
}

// ---------------------------------------------------------------------------
// neovim
// ---------------------------------------------------------------------------

static NEOVIM_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_neovim_installed() {
    setup::ensure_apt_updated().await;
    NEOVIM_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "neovim"])
                .await
                .expect("install neovim exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn install_neovim_exits_zero() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_neovim_installed().await;
}

#[tokio::test]
async fn nvim_version_works() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_neovim_installed().await;

    let container = setup::get_container().await;
    let result = container
        .exec(&["nvim", "--version"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "NVIM");
}

// ---------------------------------------------------------------------------
// github (gh CLI)
// ---------------------------------------------------------------------------

static GITHUB_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_github_installed() {
    setup::ensure_apt_updated().await;
    GITHUB_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "github"])
                .await
                .expect("install github exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn install_github_exits_zero() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_github_installed().await;
}

#[tokio::test]
async fn gh_version_works() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_github_installed().await;

    let container = setup::get_container().await;
    let result = container
        .exec(&["gh", "version"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "gh version");
}

// ---------------------------------------------------------------------------
// terraform
// ---------------------------------------------------------------------------

static TERRAFORM_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_terraform_installed() {
    setup::ensure_apt_updated().await;
    TERRAFORM_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "terraform"])
                .await
                .expect("install terraform exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn install_terraform_exits_zero() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_terraform_installed().await;
}

#[tokio::test]
async fn terraform_version_works() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_terraform_installed().await;

    let container = setup::get_container().await;
    let result = container
        .exec(&["terraform", "version"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "Terraform");
}

// ---------------------------------------------------------------------------
// nerd-font
// ---------------------------------------------------------------------------

static NERD_FONT_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_nerd_font_installed() {
    setup::ensure_apt_updated().await;
    NERD_FONT_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "nerd-font"])
                .await
                .expect("install nerd-font exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn install_nerd_font_exits_zero() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_nerd_font_installed().await;
}

#[tokio::test]
async fn nerd_font_files_exist() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_nerd_font_installed().await;

    let container = setup::get_container().await;
    // JetBrainsMono fonts are installed to ~/.local/share/fonts/JetBrainsMono/
    let result = container
        .exec(&[
            "sh",
            "-c",
            "ls \"$HOME/.local/share/fonts/JetBrainsMono/\" | grep -c JetBrainsMono",
        ])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
}

// ---------------------------------------------------------------------------
// javascript (nvm + node)
// ---------------------------------------------------------------------------

static JAVASCRIPT_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_javascript_installed() {
    setup::ensure_apt_updated().await;
    JAVASCRIPT_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "javascript"])
                .await
                .expect("install javascript exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn install_javascript_exits_zero() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_javascript_installed().await;
}

#[tokio::test]
async fn nvm_script_exists() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_javascript_installed().await;

    let container = setup::get_container().await;
    // nvm installs a shell script to ~/.nvm/nvm.sh
    let result = container
        .exec(&["sh", "-c", "test -f \"$HOME/.nvm/nvm.sh\""])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
}

#[tokio::test]
async fn node_is_available_via_nvm() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_javascript_installed().await;

    let container = setup::get_container().await;
    // Source nvm and check that node is accessible.
    let result = container
        .exec(&[
            "sh",
            "-c",
            r#"export NVM_DIR="$HOME/.nvm" && [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && node --version"#,
        ])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
    assert_stdout_contains(&result, "v");
}

// ---------------------------------------------------------------------------
// base (system packages)
// ---------------------------------------------------------------------------

static BASE_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_base_installed() {
    setup::ensure_apt_updated().await;
    BASE_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "base"])
                .await
                .expect("install base exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn install_base_exits_zero() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_base_installed().await;
}

// ---------------------------------------------------------------------------
// doas
// ---------------------------------------------------------------------------

static DOAS_INSTALLED: OnceCell<()> = OnceCell::const_new();

async fn ensure_doas_installed() {
    setup::ensure_apt_updated().await;
    DOAS_INSTALLED
        .get_or_init(|| async {
            let _guard = setup::apt_install_lock().await;
            let container = setup::get_container().await;
            let result = container
                .exec(&["bashc", "install", "doas"])
                .await
                .expect("install doas exec failed");
            assert_exit_ok(&result);
        })
        .await;
}

#[tokio::test]
async fn install_doas_exits_zero() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_doas_installed().await;
}

#[tokio::test]
async fn doas_binary_exists() {
    if std::env::var("FULL_INSTALL_TESTS").is_err() {
        return;
    }
    ensure_doas_installed().await;

    let container = setup::get_container().await;
    let result = container
        .exec(&["sh", "-c", "command -v doas"])
        .await
        .expect("exec failed");

    assert_exit_ok(&result);
}

// ---------------------------------------------------------------------------
// brew — skipped: brew is a package manager, not practically testable
// in an isolated container without significant setup overhead.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// obsidian — skipped: GUI application, no meaningful binary verification
// possible inside a headless container.
// ---------------------------------------------------------------------------
