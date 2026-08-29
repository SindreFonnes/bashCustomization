#!/usr/bin/env bash

set -euo pipefail

project_root=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
test_root=$(mktemp -d "${TMPDIR:-/tmp}/bashc-bootstrap-test.XXXXXX")
trap 'rm -rf "$test_root"' EXIT HUP INT TERM

export BASHC_INIT_SOURCE_ONLY=1
# shellcheck disable=SC1091
. "$project_root/init.sh"
unset BASHC_INIT_SOURCE_ONLY

# Startup hooks must preserve paths literally, including shell metacharacters
# and single quotes, without executing path contents in a future shell.
injection_marker="$test_root/injected"
weird_root="${test_root}/repo 'quoted \$(touch ${injection_marker})"
mkdir -p "$weird_root"
startup_file="$test_root/startup"
add_startup_hook "$startup_file" "$weird_root"
add_startup_hook "$startup_file" "$weird_root"

if [[ $(grep -c '^# bashCustomization$' "$startup_file") -ne 1 ]]; then
    printf 'startup hook was not idempotent\n' >&2
    exit 1
fi

for shell_name in bash zsh; do
    shell_path=$(command -v "$shell_name")
    # shellcheck disable=SC2016
    env EXPECTED_ROOT="$weird_root" "$shell_path" -c '
        . "$1"
        test "$BASHC_ROOT" = "$EXPECTED_ROOT"
    ' bashc-bootstrap-test "$startup_file"
done

if [[ -e "$injection_marker" ]]; then
    printf 'startup hook executed contents of BASHC_ROOT\n' >&2
    exit 1
fi

newline_path=$(printf 'first\nsecond')
if validate_single_line_path BASHC_ROOT "$newline_path" 2>/dev/null; then
    printf 'newline-containing path was accepted\n' >&2
    exit 1
fi

# Repository and shell setup must still run after a failed full install, and
# explicit `install all` must behave like the no-argument default.
setup_marker="$test_root/setup-ran"
setup_repository_and_shells() {
    : > "$setup_marker"
}

PERSISTENT_BINARY=false
if run_requested_action install all; then
    printf 'failing installer unexpectedly succeeded\n' >&2
    exit 1
else
    action_status=$?
fi
if [[ $action_status -ne 1 || ! -f "$setup_marker" ]]; then
    printf 'full install failure skipped repository setup or lost its status\n' >&2
    exit 1
fi

rm -f "$setup_marker"
if run_requested_action configs status; then
    printf 'failing non-bootstrap action unexpectedly succeeded\n' >&2
    exit 1
else
    action_status=$?
fi
if [[ $action_status -ne 1 || -e "$setup_marker" ]]; then
    printf 'non-bootstrap action unexpectedly ran repository setup\n' >&2
    exit 1
fi

# Used by run_requested_action from the dynamically sourced bootstrap.
# shellcheck disable=SC2034
PERSISTENT_BINARY=true
run_requested_action
if [[ ! -f "$setup_marker" ]]; then
    printf 'default full install skipped repository setup\n' >&2
    exit 1
fi

# The bootstrap records a custom binary directory, and future Bash/Zsh
# sessions put that exact directory on PATH ahead of system binaries.
test_home="$test_root/home"
custom_bin="$test_root/custom bin"
mkdir -p "$test_home/.config/bashc" "$custom_bin"
printf '#!/bin/sh\nexit 0\n' > "$custom_bin/bashc"
chmod 755 "$custom_bin/bashc"

HOME="$test_home"
# Used by record_install_dir from the dynamically sourced bootstrap.
# shellcheck disable=SC2034
STAGED_INSTALL_STATE=""
record_install_dir "$custom_bin"
if [[ $(<"$test_home/.config/bashc/install_dir") != "$custom_bin" ]]; then
    printf 'custom install directory was not recorded\n' >&2
    exit 1
fi

for shell_name in bash zsh; do
    shell_path=$(command -v "$shell_name")
    # shellcheck disable=SC2016
    env \
        HOME="$test_home" \
        PATH="/usr/bin:/bin" \
        EXPECTED_BIN="$custom_bin" \
        BASHC_ROOT="$project_root" \
        BASHC_SKIP_UPDATE_CHECK=1 \
        BASHC_SKIP_CONFIG_CHECK=1 \
        "$shell_path" -c '
            . "$BASHC_ROOT/main.sh" || exit 1
            test "$BASHC_INSTALL_DIR" = "$EXPECTED_BIN"
            test "$(command -v bashc)" = "$EXPECTED_BIN/bashc"
            alias installStuff | grep -F "run_my_install base" >/dev/null
            mixed_case="Ab C"
            variable_to_lowercase mixed_case || exit 1
            test "$mixed_case" = "ab c"
            variable_to_uppercase mixed_case || exit 1
            test "$mixed_case" = "AB C"
            if variable_to_lowercase "invalid;name" 2>/dev/null; then exit 1; fi
        '
done

# Local customization writers generate code that is sourced on every shell
# startup. Preserve values literally and reject invalid identifiers so a quote
# or command substitution in user data cannot corrupt that startup chain.
local_injection_marker="$test_root/local-injected"
unsafe_local_value="literal ' \$(touch ${local_injection_marker})"
for shell_name in bash zsh; do
    shell_path=$(command -v "$shell_name")
    local_fixture="$test_root/local-$shell_name"
    mkdir -p "$local_fixture"
    : > "$local_fixture/local_variables.sh"
    : > "$local_fixture/local_aliases.sh"
    # shellcheck disable=SC2016
    env \
        LOCAL_FIXTURE="$local_fixture" \
        PROJECT_ROOT="$project_root" \
        UNSAFE_LOCAL_VALUE="$unsafe_local_value" \
        "$shell_path" -c '
            local_dir=$LOCAL_FIXTURE
            standard_editor=vi
            _bashc_source_file() { . "$1"; }
            . "$PROJECT_ROOT/local/local_main.sh" || exit 1
            add_local_variable sample "$UNSAFE_LOCAL_VALUE" || exit 1
            add_local_variable prefixed value true || exit 1
            add_local_alias sample-alias "$UNSAFE_LOCAL_VALUE" || exit 1
            if add_local_variable "invalid-name" value 2>/dev/null; then exit 1; fi
            . "$local_dir/local_variables.sh" || exit 1
            . "$local_dir/local_aliases.sh" || exit 1
            test "$sample" = "$UNSAFE_LOCAL_VALUE"
            test "$local_prefixed" = value
            alias sample-alias >/dev/null
        '
done

if [[ -e "$local_injection_marker" ]]; then
    printf 'local customization writer executed value contents\n' >&2
    exit 1
fi

# General-script compatibility names must dispatch installer work to bashc.
dispatch_bin="$test_root/dispatch-bin"
dispatch_marker="$test_root/dispatch-args"
mkdir -p "$dispatch_bin"
# The generated helper expands these variables when gScriptRun invokes it.
# shellcheck disable=SC2016
printf '#!/bin/sh\nprintf "%%s\\n" "$*" > "$BASHC_DISPATCH_MARKER"\n' > "$dispatch_bin/bashc"
chmod 755 "$dispatch_bin/bashc"

env \
    PATH="$dispatch_bin:/usr/bin:/bin" \
    BASHC_DISPATCH_MARKER="$dispatch_marker" \
    GENERAL_SCRIPTS_FOLDER_LOCATION="$project_root/generalScripts" \
    "$project_root/generalScripts/gScriptRun.sh" installStuff
if [[ $(<"$dispatch_marker") != "install base" ]]; then
    printf 'gscript installStuff bypassed the supported installer\n' >&2
    exit 1
fi

env \
    PATH="$dispatch_bin:/usr/bin:/bin" \
    BASHC_DISPATCH_MARKER="$dispatch_marker" \
    GENERAL_SCRIPTS_FOLDER_LOCATION="$project_root/generalScripts" \
    "$project_root/generalScripts/gScriptRun.sh" installNerdFont
if [[ $(<"$dispatch_marker") != "install nerd-font" ]]; then
    printf 'gscript installNerdFont bypassed the supported installer\n' >&2
    exit 1
fi

env \
    PATH="$dispatch_bin:/usr/bin:/bin" \
    BASHC_DISPATCH_MARKER="$dispatch_marker" \
    "$project_root/installScripts/ripgrep/installRipgrep.sh"
if [[ $(<"$dispatch_marker") != "install ripgrep" ]]; then
    printf 'legacy ripgrep launcher bypassed bashc\n' >&2
    exit 1
fi

env \
    PATH="$dispatch_bin:/usr/bin:/bin" \
    BASHC_DISPATCH_MARKER="$dispatch_marker" \
    "$project_root/installScripts/installScript.sh" rg --dry-run
if [[ $(<"$dispatch_marker") != "install ripgrep --dry-run" ]]; then
    printf 'legacy installer shorthand was not preserved\n' >&2
    exit 1
fi

env \
    PATH="$dispatch_bin:/usr/bin:/bin" \
    BASHC_DISPATCH_MARKER="$dispatch_marker" \
    "$project_root/installScripts/installScript.sh" js nvm --dry-run
if [[ $(<"$dispatch_marker") != "install javascript --dry-run" ]]; then
    printf 'legacy JavaScript selector was not normalized\n' >&2
    exit 1
fi

if env \
    PATH="$dispatch_bin:/usr/bin:/bin" \
    GENERAL_SCRIPTS_FOLDER_LOCATION="$project_root/generalScripts" \
    "$project_root/generalScripts/gScriptRun.sh" invalid >/dev/null 2>&1
then
    printf 'invalid gscript option returned success\n' >&2
    exit 1
fi

printf 'bootstrap shell checks passed\n'
