#!/usr/bin/env bash

set -euo pipefail

project_root=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)

(
    cd "$project_root/rust"
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --all-targets
)

(
    cd "$project_root/tests/e2e"
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --all-targets --no-run
)

while IFS= read -r -d '' script; do
    bash -n "$script"
    zsh -n "$script"
done < <(
    find "$project_root" \
        \( -path "$project_root/.git" \
        -o -path "$project_root/rust/target" \
        -o -path "$project_root/tests/e2e/target" \) -prune \
        -o -type f -name '*.sh' -print0
)

shellcheck \
    "$project_root/init.sh" \
    "$project_root/tests/shell/smoke.sh" \
    "$project_root/tests/validate.sh"

"$project_root/tests/shell/smoke.sh"
