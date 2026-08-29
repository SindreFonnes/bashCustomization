#!/usr/bin/env bash

set -euo pipefail

project_root=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)

if ! command -v cargo-deny >/dev/null 2>&1; then
    printf 'cargo-deny is required; install it with: cargo install --locked cargo-deny\n' >&2
    exit 1
fi

cargo deny --manifest-path "$project_root/rust/Cargo.toml" check
cargo deny --manifest-path "$project_root/tests/e2e/Cargo.toml" check
