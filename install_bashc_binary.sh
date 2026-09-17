#!/bin/sh
# Fetch, verify, and persist the bashc GitHub-release binary.
# Source this file to use install_bashc_binary, or run it to install only.
# Requirements: curl, sh (POSIX)

REPO="${REPO:-SindreFonnes/bashCustomization}"
BINARY_NAME="${BINARY_NAME:-bashc}"

curl_fetch() {
    curl --fail --silent --show-error --location \
        --connect-timeout 10 --max-time 120 \
        --retry 2 --retry-delay 1 --retry-connrefused "$@"
}

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

cleanup_bashc_binary_install() {
    if [ -n "${BASHC_TMP_DIR:-}" ]; then
        rm -rf "$BASHC_TMP_DIR"
    fi
    if [ -n "${STAGED_BINARY:-}" ]; then
        rm -f "$STAGED_BINARY"
    fi
    if [ -n "${STAGED_INSTALL_STATE:-}" ]; then
        rm -f "$STAGED_INSTALL_STATE"
    fi
}

validate_single_line_path() {
    _bashc_path_label=$1
    _bashc_path_value=$2
    case "$_bashc_path_value" in
        *'
'*)
            echo "Error: ${_bashc_path_label} must not contain a newline" >&2
            return 1
            ;;
    esac
}

record_install_dir() {
    _bashc_state_dir="$HOME/.config/bashc"
    _bashc_state_file="${_bashc_state_dir}/install_dir"
    mkdir -p "$_bashc_state_dir"
    STAGED_INSTALL_STATE=$(mktemp "${_bashc_state_dir}/.install_dir.XXXXXX")
    printf '%s\n' "$1" > "$STAGED_INSTALL_STATE"
    chmod 600 "$STAGED_INSTALL_STATE"
    mv -f "$STAGED_INSTALL_STATE" "$_bashc_state_file"
    STAGED_INSTALL_STATE=""
}

resolve_release_url() {
    _bashc_target=$1
    curl_fetch "https://api.github.com/repos/${REPO}/releases/latest" | \
        grep "browser_download_url.*${BINARY_NAME}-${_bashc_target}\"" | \
        head -1 | \
        cut -d'"' -f4
}

install_bashc_binary() {
    _bashc_target=$1
    if [ -z "$_bashc_target" ]; then
        _bashc_target="$(detect_arch)-$(detect_os)"
    fi

    case "$_bashc_target" in
        x86_64-apple-darwin|aarch64-apple-darwin|x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu|x86_64-unknown-linux-musl)
            ;;
        *)
            echo "Error: No bashc release artifact is produced for ${_bashc_target}" >&2
            echo "Supported release targets: x86_64/aarch64 macOS, x86_64/aarch64 glibc Linux, x86_64 Alpine" >&2
            return 1
            ;;
    esac

    echo "Fetching latest release..."

    RELEASE_URL=$(resolve_release_url "$_bashc_target")

    if [ -z "$RELEASE_URL" ]; then
        echo "Error: Could not find a release binary for ${_bashc_target}" >&2
        echo "Check https://github.com/${REPO}/releases for available binaries" >&2
        return 1
    fi

    SHA_URL="${RELEASE_URL}.sha256"

    BASHC_TMP_DIR=$(mktemp -d)
    BINARY_PATH="${BASHC_TMP_DIR}/${BINARY_NAME}"
    SHA_PATH="${BASHC_TMP_DIR}/${BINARY_NAME}.sha256"
    STAGED_BINARY=""
    STAGED_INSTALL_STATE=""
    trap cleanup_bashc_binary_install EXIT HUP INT TERM

    echo "Downloading ${BINARY_NAME} for ${_bashc_target}..."
    curl_fetch -o "$BINARY_PATH" "$RELEASE_URL"

    echo "Downloading checksum..."
    curl_fetch -o "$SHA_PATH" "$SHA_URL"

    # Extract expected hash (first field of sha256 file)
    EXPECTED_HASH=$(cut -d' ' -f1 < "$SHA_PATH")
    verify_checksum "$BINARY_PATH" "$EXPECTED_HASH"

    chmod +x "$BINARY_PATH"

    # Persist the verified binary before running it. The selected install path
    # is recorded so future shells can put the same directory on PATH.
    INSTALL_DIR=${BASHC_INSTALL_DIR:-"$HOME/.mybin"}
    validate_single_line_path "BASHC_INSTALL_DIR" "$INSTALL_DIR" || return 1
    mkdir -p "$INSTALL_DIR"
    INSTALL_DIR=$(CDPATH='' cd -P -- "$INSTALL_DIR" && pwd -P)
    PERSISTENT_BINARY="${INSTALL_DIR}/${BINARY_NAME}"
    STAGED_BINARY=$(mktemp "${INSTALL_DIR}/.bashc.XXXXXX")
    cp "$BINARY_PATH" "$STAGED_BINARY"
    chmod 755 "$STAGED_BINARY"
    mv -f "$STAGED_BINARY" "$PERSISTENT_BINARY"
    STAGED_BINARY=""
    record_install_dir "$INSTALL_DIR"
    echo "Installed ${BINARY_NAME} to ${PERSISTENT_BINARY}"

    trap - EXIT HUP INT TERM
    cleanup_bashc_binary_install
    BASHC_TMP_DIR=""
}

if [ "${0##*/}" = "install_bashc_binary.sh" ]; then
    set -e
    install_bashc_binary "${1:-}"
fi
