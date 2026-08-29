#!/bin/sh

set -eu

tool=${1:-}
if [ -z "$tool" ]; then
    printf 'Usage: %s <bashc-tool> [arguments...]\n' "$0" >&2
    exit 2
fi
shift

if ! command -v bashc >/dev/null 2>&1; then
    project_root=${BASHC_ROOT:-"$HOME/bashCustomization"}
    printf 'bashc: the Rust binary is required; run %s/init.sh or build %s/rust\n' \
        "$project_root" "$project_root" >&2
    exit 1
fi

exec bashc install "$tool" "$@"
