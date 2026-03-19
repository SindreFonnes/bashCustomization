#!/bin/sh
# Bootstrap script for bashCustomization
# Downloads and runs the bashc binary on a fresh machine.
# Requirements: curl, sh (POSIX)
set -e

REPO="sindre/bashCustomization"
BINARY_NAME="bashc"

# --- Platform detection ---

detect_os() {
    case "$(uname -s)" in
        Darwin) echo "apple-darwin" ;;
        Linux)  echo "unknown-linux-gnu" ;;
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

# --- Checksum verification ---

verify_checksum() {
    file="$1"
    expected="$2"

    if command -v sha256sum >/dev/null 2>&1; then
        actual=$(sha256sum "$file" | cut -d' ' -f1)
    elif command -v shasum >/dev/null 2>&1; then
        actual=$(shasum -a 256 "$file" | cut -d' ' -f1)
    else
        echo "Warning: No sha256sum or shasum found — skipping checksum verification" >&2
        return 0
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

echo "Detected platform: ${TARGET}"
echo "Fetching latest release..."

# Get the latest release download URL
RELEASE_URL=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | \
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

echo "Downloading ${BINARY_NAME} for ${TARGET}..."
curl -fsSL -o "$BINARY_PATH" "$RELEASE_URL"

echo "Downloading checksum..."
curl -fsSL -o "$SHA_PATH" "$SHA_URL"

# Extract expected hash (first field of sha256 file)
EXPECTED_HASH=$(cut -d' ' -f1 < "$SHA_PATH")
verify_checksum "$BINARY_PATH" "$EXPECTED_HASH"

chmod +x "$BINARY_PATH"

# Run bashc with provided arguments, or default to "install all"
if [ $# -eq 0 ]; then
    echo "Running: ${BINARY_NAME} install all"
    "$BINARY_PATH" install all
else
    echo "Running: ${BINARY_NAME} $*"
    "$BINARY_PATH" "$@"
fi

# Clean up
rm -rf "$TMPDIR"

echo ""
echo "Done. To install bashc permanently, copy it to a directory on your PATH:"
echo "  sudo cp ${BINARY_PATH} /usr/local/bin/${BINARY_NAME}"
