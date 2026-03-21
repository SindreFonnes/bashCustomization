#!/bin/sh
# Docker-based integration tests for bashc across Linux distros.
# Runs from the repository root. Requires: docker.
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DOCKER_DIR="$SCRIPT_DIR"
BUILD_DIR="$SCRIPT_DIR/.build"

# Colors (if terminal supports them)
if [ -t 1 ]; then
    GREEN='\033[0;32m'
    RED='\033[0;31m'
    YELLOW='\033[0;33m'
    BOLD='\033[1m'
    NC='\033[0m'
else
    GREEN='' RED='' YELLOW='' BOLD='' NC=''
fi

passed=0
failed=0
skipped=0
results=""

# Colored output helper — avoids SC2059 by using %b for escape sequences
say() { printf '%b\n' "$*"; }

log_pass() {
    passed=$((passed + 1))
    results="$results
  ${GREEN}PASS${NC}  $1"
}

log_fail() {
    failed=$((failed + 1))
    results="$results
  ${RED}FAIL${NC}  $1: $2"
}

log_skip() {
    skipped=$((skipped + 1))
    results="$results
  ${YELLOW}SKIP${NC}  $1: $2"
}

# --- Build binaries via Docker ---

build_binaries() {
    say "${BOLD}Building bashc binaries via Docker...${NC}"
    mkdir -p "$BUILD_DIR"

    # Build a statically-linked musl binary via Docker (works on all Linux distros)
    say "  Building statically-linked bashc binary (this may take a few minutes on first run)..."
    build_output=$(docker build --output "type=local,dest=$BUILD_DIR" \
        -f "$DOCKER_DIR/Dockerfile.builder" "$REPO_ROOT" 2>&1) || {
        printf '%s\n' "$build_output" | tail -10
        say "  ${RED}Docker build failed${NC}"
        exit 1
    }
    printf '%s\n' "$build_output" | tail -3

    printf '  Binaries ready in %s\n\n' "$BUILD_DIR"
}

# --- Docker helpers ---

build_image() {
    distro="$1"
    dockerfile="$DOCKER_DIR/Dockerfile.$distro"

    printf '  Building bashc-test-%s...' "$distro"
    # Copy the binary next to the Dockerfile for COPY context
    cp "$BUILD_DIR/bashc" "$DOCKER_DIR/bashc"

    if docker build -t "bashc-test-$distro" -f "$dockerfile" "$DOCKER_DIR" >/dev/null 2>&1; then
        printf ' done\n'
        return 0
    else
        say " ${RED}failed${NC}"
        return 1
    fi
}

run_in() {
    distro="$1"
    shift
    docker run --rm "bashc-test-$distro" sh -c "$*" 2>&1
}

# --- Test functions ---

test_dry_run() {
    distro="$1"
    expected_distro_label="$2"
    test_name="$distro/dry-run"

    output=$(run_in "$distro" "bashc install --dry-run all") || {
        log_fail "$test_name" "exit code non-zero"
        printf '    Output: %s\n' "$output"
        return
    }

    if echo "$output" | grep -Eqi "$expected_distro_label"; then
        log_pass "$test_name"
    else
        log_fail "$test_name" "output missing distro label '$expected_distro_label'"
        printf '    First line: %s\n' "$(echo "$output" | head -1)"
    fi
}

test_real_install() {
    distro="$1"
    test_name="$distro/real-install"

    # Refresh package cache and install a lightweight CLI tool
    output=$(run_in "$distro" "apt-get update -qq && bashc install ripgrep && rg --version") || {
        log_fail "$test_name" "ripgrep install or version check failed"
        printf '    Output: %s\n' "$(echo "$output" | tail -5)"
        return
    }

    if echo "$output" | grep -q "ripgrep"; then
        log_pass "$test_name"
    else
        log_fail "$test_name" "rg --version output unexpected"
    fi
}

test_stub_message() {
    distro="$1"
    test_name="$distro/stub-message"

    # A real install should fail with "not yet supported", not crash
    output=$(run_in "$distro" "bashc install ripgrep 2>&1; exit 0")

    if echo "$output" | grep -Eqi "not yet supported|not yet implemented|not yet configured"; then
        log_pass "$test_name"
    else
        log_fail "$test_name" "expected 'not yet supported' message"
        printf '    Output: %s\n' "$(echo "$output" | tail -3)"
    fi
}

test_alpine_doas() {
    test_name="alpine/doas-bootstrap"

    # Run as root, no sudo/doas available — bashc should install doas
    output=$(run_in "alpine" "bashc install doas && command -v doas && cat /etc/doas.d/doas.conf") || {
        log_fail "$test_name" "doas install failed"
        printf '    Output: %s\n' "$(echo "$output" | tail -5)"
        return
    }

    if echo "$output" | grep -q "permit persist"; then
        log_pass "$test_name"
    else
        log_fail "$test_name" "doas.conf missing 'permit persist' line"
    fi
}

test_nixos_guidance() {
    test_name="nixos/guidance"

    output=$(run_in "nixos" "bashc install --dry-run all") || {
        log_fail "$test_name" "dry-run exited non-zero"
        printf '    Output: %s\n' "$(echo "$output" | tail -5)"
        return
    }

    # NixOS dry-run should mention the distro
    if echo "$output" | grep -Eqi "NixOS|nixos"; then
        log_pass "$test_name"
    else
        log_fail "$test_name" "output missing NixOS reference"
    fi
}

# --- Main ---

say "${BOLD}bashc Docker Integration Tests${NC}"
say "================================"
echo

# Check for docker
if ! command -v docker >/dev/null 2>&1; then
    say "${RED}docker not found. Install Docker to run these tests.${NC}"
    exit 1
fi

# Build
build_binaries

# Build images
say "${BOLD}Building Docker images...${NC}"
distros="debian ubuntu fedora arch alpine nixos"
for distro in $distros; do
    if ! build_image "$distro"; then
        log_skip "$distro" "image build failed or binary not available"
    fi
done

# Clean up copied binaries
rm -f "$DOCKER_DIR/bashc"

echo
say "${BOLD}Running tests...${NC}"
echo

# Dry-run tests (all distros)
for distro in $distros; do
    case "$distro" in
        debian)  label="Debian" ;;
        ubuntu)  label="Debian" ;;  # Ubuntu detected as Debian family
        fedora)  label="Fedora" ;;
        arch)    label="Arch" ;;
        alpine)  label="Alpine" ;;
        nixos)   label="NixOS|nixos|Unknown" ;;
    esac

    if docker image inspect "bashc-test-$distro" >/dev/null 2>&1; then
        test_dry_run "$distro" "$label"
    fi
done

# Real install tests (Debian/Ubuntu only)
for distro in debian ubuntu; do
    if docker image inspect "bashc-test-$distro" >/dev/null 2>&1; then
        test_real_install "$distro"
    fi
done

# Stub message tests (Fedora/Arch)
for distro in fedora arch; do
    if docker image inspect "bashc-test-$distro" >/dev/null 2>&1; then
        test_stub_message "$distro"
    fi
done

# Alpine doas bootstrap
if docker image inspect "bashc-test-alpine" >/dev/null 2>&1; then
    test_alpine_doas
fi

# NixOS guidance
if docker image inspect "bashc-test-nixos" >/dev/null 2>&1; then
    test_nixos_guidance
fi

# --- Summary ---

echo
say "${BOLD}Results${NC}"
say "================================"
printf '%b\n' "$results"
printf '\n  %b%s passed%b' "$GREEN" "$passed" "$NC"
if [ "$failed" -gt 0 ]; then
    printf ', %b%s failed%b' "$RED" "$failed" "$NC"
fi
if [ "$skipped" -gt 0 ]; then
    printf ', %b%s skipped%b' "$YELLOW" "$skipped" "$NC"
fi
echo

# Clean up build artifacts
rm -rf "$BUILD_DIR"

if [ "$failed" -gt 0 ]; then
    exit 1
fi
