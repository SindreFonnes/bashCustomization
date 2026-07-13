#!/usr/bin/env bash

set -euo pipefail

project_root=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
container_names=(
    bashc-binary-extractor-"$$"
    bashc-e2e-debian
    bashc-e2e-ubuntu
    bashc-e2e-fedora
    bashc-e2e-arch
    bashc-e2e-alpine
    bashc-e2e-nixos
)
image_names=(
    bashc-builder
    bashc-test-debian
    bashc-test-ubuntu
    bashc-test-fedora
    bashc-test-arch
    bashc-test-alpine
    bashc-test-nixos
)

remove_resources() {
    docker rm -f "${container_names[@]}" >/dev/null 2>&1 || true
    docker image rm -f "${image_names[@]}" >/dev/null 2>&1 || true
    rm -f "$project_root/tests/docker/bashc"
}

finish() {
    if [[ -z ${KEEP_E2E_RESOURCES:-} ]]; then
        remove_resources
    fi
}

if ! docker info >/dev/null 2>&1; then
    printf 'Docker is required and its daemon must be running for E2E tests.\n' >&2
    exit 1
fi

# Start from no reusable images, then allow all distro test binaries in this
# one run to share the builder image produced from the current source tree.
remove_resources
trap finish EXIT HUP INT TERM
export REUSE_E2E_IMAGES=1

cd "$project_root/tests/e2e"
cargo test "$@"
