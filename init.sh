#!/bin/sh
# Bootstrap script for bashCustomization
# Downloads and runs the bashc binary on a fresh machine.
# Requirements: curl, sh (POSIX)
set -e

REPO="sindre/bashCustomization"
BINARY_NAME="bashc"

curl_fetch() {
    curl --fail --silent --show-error --location \
        --connect-timeout 10 --max-time 120 \
        --retry 2 --retry-delay 1 --retry-connrefused "$@"
}

# --- Platform detection ---

detect_os() {
    case "$(uname -s)" in
        Darwin) echo "apple-darwin" ;;
        Linux)
            case "$(detect_distro)" in
                alpine) echo "unknown-linux-musl" ;;
                *)      echo "unknown-linux-gnu" ;;
            esac
            ;;
        *)
            echo "Error: Unsupported OS: $(uname -s)" >&2
            echo "Supported: macOS (Darwin), Linux" >&2
            exit 1
            ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)   echo "x86_64" ;;
        aarch64|arm64)   echo "aarch64" ;;
        *)
            echo "Error: Unsupported architecture: $(uname -m)" >&2
            echo "Supported: x86_64, aarch64/arm64" >&2
            exit 1
            ;;
    esac
}

detect_distro() {
    # On macOS there is no /etc/os-release
    if [ "$(uname -s)" = "Darwin" ]; then
        echo "macos"
        return
    fi

    if [ ! -f /etc/os-release ]; then
        echo "unknown"
        return
    fi

    # Read ID and ID_LIKE from /etc/os-release
    _id=""
    _id_like=""
    while IFS='=' read -r key value; do
        # Strip surrounding quotes from value
        value=$(printf '%s' "$value" | tr -d '"'"'")
        case "$key" in
            ID)      _id="$value" ;;
            ID_LIKE) _id_like="$value" ;;
        esac
    done < /etc/os-release

    # Match against known distro families; ID takes priority, then ID_LIKE
    for _field in "$_id" "$_id_like"; do
        case "$_field" in
            *alpine*)  echo "alpine";  return ;;
            *nixos*)   echo "nixos";   return ;;
            *arch*)    echo "arch";    return ;;
            *fedora*|*rhel*|*centos*|*suse*)
                       echo "fedora";  return ;;
            *debian*|*ubuntu*|*raspbian*)
                       echo "debian";  return ;;
        esac
    done

    echo "unknown"
}

# --- Privilege-escalation bootstrap (Alpine only) ---

bootstrap_doas_alpine() {
    # Only applies when running as root on Alpine with no sudo/doas/su available
    if [ "$(detect_distro)" != "alpine" ]; then
        return
    fi

    if [ "$(id -u)" != "0" ]; then
        return
    fi

    if command -v sudo >/dev/null 2>&1 || \
       command -v doas >/dev/null 2>&1 || \
       command -v su   >/dev/null 2>&1; then
        return
    fi

    echo "Alpine: no sudo/doas/su found — installing doas via apk..."
    apk add --no-cache doas

    if [ ! -d /etc/doas.d ]; then
        mkdir -p /etc/doas.d
    fi

    printf 'permit persist :wheel\n' > /etc/doas.d/doas.conf
    echo "Alpine: created /etc/doas.d/doas.conf with 'permit persist :wheel'"
}

# --- Checksum verification ---

verify_checksum() {
    file="$1"
    expected="$2"

    if command -v sha256sum >/dev/null 2>&1; then
        actual=$(sha256sum "$file" | cut -d' ' -f1)
    elif command -v shasum >/dev/null 2>&1; then
        actual=$(shasum -a 256 "$file" | cut -d' ' -f1)
    else
        echo "Error: No sha256sum or shasum found — cannot verify bashc" >&2
        return 1
    fi

    if [ "$actual" != "$expected" ]; then
        echo "Error: Checksum mismatch for $file" >&2
        echo "  expected: $expected" >&2
        echo "  actual:   $actual" >&2
        exit 1
    fi

    echo "Checksum OK"
}

# --- Main ---

OS=$(detect_os)
ARCH=$(detect_arch)
TARGET="${ARCH}-${OS}"

case "$TARGET" in
    x86_64-apple-darwin|aarch64-apple-darwin|x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu|x86_64-unknown-linux-musl)
        ;;
    *)
        echo "Error: No bashc release artifact is produced for ${TARGET}" >&2
        echo "Supported release targets: x86_64/aarch64 macOS, x86_64/aarch64 glibc Linux, x86_64 Alpine" >&2
        exit 1
        ;;
esac

echo "Detected platform: ${TARGET}"

# Bootstrap doas on Alpine when running as root with no privilege-escalation tool
bootstrap_doas_alpine

echo "Fetching latest release..."

# Get the latest release download URL
RELEASE_URL=$(curl_fetch "https://api.github.com/repos/${REPO}/releases/latest" | \
    grep "browser_download_url.*${BINARY_NAME}-${TARGET}\"" | \
    head -1 | \
    cut -d'"' -f4)

if [ -z "$RELEASE_URL" ]; then
    echo "Error: Could not find a release binary for ${TARGET}" >&2
    echo "Check https://github.com/${REPO}/releases for available binaries" >&2
    exit 1
fi

SHA_URL="${RELEASE_URL}.sha256"

TMPDIR=$(mktemp -d)
BINARY_PATH="${TMPDIR}/${BINARY_NAME}"
SHA_PATH="${TMPDIR}/${BINARY_NAME}.sha256"
STAGED_BINARY=""
cleanup() {
    rm -rf "$TMPDIR"
    if [ -n "$STAGED_BINARY" ]; then
        rm -f "$STAGED_BINARY"
    fi
}
trap cleanup EXIT HUP INT TERM

echo "Downloading ${BINARY_NAME} for ${TARGET}..."
curl_fetch -o "$BINARY_PATH" "$RELEASE_URL"

echo "Downloading checksum..."
curl_fetch -o "$SHA_PATH" "$SHA_URL"

# Extract expected hash (first field of sha256 file)
EXPECTED_HASH=$(cut -d' ' -f1 < "$SHA_PATH")
verify_checksum "$BINARY_PATH" "$EXPECTED_HASH"

chmod +x "$BINARY_PATH"

# Persist the verified binary before running it. The shell framework already
# adds ~/.mybin to PATH, and BASHC_INSTALL_DIR allows an explicit override.
INSTALL_DIR=${BASHC_INSTALL_DIR:-"$HOME/.mybin"}
PERSISTENT_BINARY="${INSTALL_DIR}/${BINARY_NAME}"
mkdir -p "$INSTALL_DIR"
STAGED_BINARY=$(mktemp "${INSTALL_DIR}/.bashc.XXXXXX")
cp "$BINARY_PATH" "$STAGED_BINARY"
chmod 755 "$STAGED_BINARY"
mv -f "$STAGED_BINARY" "$PERSISTENT_BINARY"
STAGED_BINARY=""
echo "Installed ${BINARY_NAME} to ${PERSISTENT_BINARY}"

setup_repository_and_shells() {
    project_root=${BASHC_ROOT:-"$HOME/bashCustomization"}

    if [ ! -d "$project_root" ]; then
        if ! command -v git >/dev/null 2>&1; then
            echo "Error: git is required to clone bashCustomization after tool setup" >&2
            return 1
        fi
        echo "Cloning bashCustomization to ${project_root}..."
        git clone "https://github.com/${REPO}.git" "$project_root"
    elif [ ! -f "$project_root/main.sh" ]; then
        echo "Error: ${project_root} exists but does not contain main.sh" >&2
        return 1
    fi

    add_startup_hook "$HOME/.bashrc" "$project_root"
    add_startup_hook "$HOME/.zshrc" "$project_root"

    echo "Shell startup configured for Bash and Zsh."
    echo "Start a new shell or source ${project_root}/main.sh to load the framework."
}

add_startup_hook() {
    startup_file=$1
    project_root=$2

    if [ -f "$startup_file" ] && \
       grep -F "export BASHC_ROOT=\"$project_root\"" "$startup_file" >/dev/null 2>&1; then
        return 0
    fi

    {
        printf '\n# bashCustomization\n'
        printf 'export BASHC_ROOT="%s"\n' "$project_root"
        # These variables must be expanded by the user's future shell.
        # shellcheck disable=SC2016
        printf 'if [ -f "$BASHC_ROOT/main.sh" ]; then\n'
        # shellcheck disable=SC2016
        printf '    . "$BASHC_ROOT/main.sh"\n'
        printf 'fi\n'
    } >> "$startup_file"
}

# Run bashc with provided arguments, or default to "install all"
if [ $# -eq 0 ]; then
    echo "Running: ${BINARY_NAME} install all"
    "$PERSISTENT_BINARY" install all
    setup_repository_and_shells
else
    echo "Running: ${BINARY_NAME} $*"
    "$PERSISTENT_BINARY" "$@"
fi

echo ""
echo "Done. bashc is installed at ${PERSISTENT_BINARY}."
