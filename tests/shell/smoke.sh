#!/bin/sh

set -eu

project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/../.." && pwd)
test_root=$(mktemp -d "${TMPDIR:-/tmp}/bashc-shell-smoke.XXXXXX")
trap 'rm -rf "$test_root"' EXIT HUP INT TERM

run_shell_checks() {
    shell_name=$1
    shell_path=$(command -v "$shell_name")

    # The single-quoted program is intentionally expanded by the child shell.
    # shellcheck disable=SC2016
    env \
        BASHC_ROOT="$project_root" \
        BASHC_SKIP_UPDATE_CHECK=1 \
        BASHC_SKIP_CONFIG_CHECK=1 \
        "$shell_path" -c '
            . "$BASHC_ROOT/main.sh" || exit 1
            command -v git_pull_repo >/dev/null 2>&1
        '

    success_marker="$test_root/${shell_name}-success"
    # shellcheck disable=SC2016
    env \
        BASHC_ROOT="$project_root" \
        BASHC_SKIP_UPDATE_CHECK=1 \
        BASHC_SKIP_CONFIG_CHECK=1 \
        BASHC_UPDATE_STATE_FILE="$success_marker" \
        "$shell_path" -c '
            . "$BASHC_ROOT/main.sh" || exit 1
            unset BASHC_SKIP_UPDATE_CHECK
            git_pull_repo() { return 0; }
            check_for_shell_update_once_a_day || exit 1
            test "$(cat "$BASHC_UPDATE_STATE_FILE")" = "$(date +%F)"
        '

    failure_marker="$test_root/${shell_name}-failure"
    printf '%s\n' "2000-01-01" > "$failure_marker"
    # shellcheck disable=SC2016
    env \
        BASHC_ROOT="$project_root" \
        BASHC_SKIP_UPDATE_CHECK=1 \
        BASHC_SKIP_CONFIG_CHECK=1 \
        BASHC_UPDATE_STATE_FILE="$failure_marker" \
        "$shell_path" -c '
            . "$BASHC_ROOT/main.sh" || exit 1
            unset BASHC_SKIP_UPDATE_CHECK
            git_pull_repo() { return 1; }
            if check_for_shell_update_once_a_day; then
                exit 1
            fi
            test "$(cat "$BASHC_UPDATE_STATE_FILE")" = "2000-01-01"
        '

    startup_home="$test_root/${shell_name}-home"
    failure_bin="$test_root/${shell_name}-failure-bin"
    startup_failure_marker="$test_root/${shell_name}-startup-failure"
    mkdir -p "$startup_home" "$failure_bin"
    printf '#!/bin/sh\nexit 1\n' > "$failure_bin/git"
    chmod +x "$failure_bin/git"
    printf '%s\n' "2000-01-01" > "$startup_failure_marker"
    # shellcheck disable=SC2016
    env \
        HOME="$startup_home" \
        PATH="$failure_bin:$PATH" \
        BASHC_ROOT="$project_root" \
        BASHC_SKIP_CONFIG_CHECK=1 \
        BASHC_UPDATE_STATE_FILE="$startup_failure_marker" \
        "$shell_path" -c '
            . "$BASHC_ROOT/main.sh" || exit 1
            test "$(cat "$BASHC_UPDATE_STATE_FILE")" = "2000-01-01"
        '

    printf 'shell smoke checks passed: %s\n' "$shell_name"
}

run_shell_checks bash
run_shell_checks zsh
