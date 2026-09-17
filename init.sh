#!/bin/sh
# Bootstrap script for bashCustomization
# Downloads and runs the bashc binary on a fresh machine.
# Requirements: curl, sh (POSIX)
set -e

# Load release-binary fetch/install helpers from the same directory as this
# file, including when this script is sourced from Bash tests.
_bashc_self=$0
# shellcheck disable=SC3028
if [ -n "${BASH_SOURCE:-}" ]; then
    _bashc_self=$BASH_SOURCE
fi
_bashc_init_dir=$(CDPATH='' cd -- "$(dirname -- "$_bashc_self")" && pwd)
if [ ! -f "$_bashc_init_dir/install_bashc_binary.sh" ]; then
    echo "Error: missing ${_bashc_init_dir}/install_bashc_binary.sh" >&2
    echo "Run init.sh from a bashCustomization checkout so the binary installer is available." >&2
    exit 1
fi
# shellcheck source=install_bashc_binary.sh
. "$_bashc_init_dir/install_bashc_binary.sh"
unset _bashc_self _bashc_init_dir

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

shell_quote() {
    # Emit one POSIX-shell word. Newlines are rejected by the caller so command
    # substitution cannot silently alter the path.
    printf "'%s'" "$(printf '%s' "$1" | sed "s/'/'\\\\''/g")"
}

find_git() {
    # A Homebrew installation performed by the child bashc process cannot
    # update this parent shell's PATH. Prefer its standard locations before
    # falling back to the caller's existing PATH.
    for _bashc_git_candidate in \
        /opt/homebrew/bin/git \
        /usr/local/bin/git \
        /home/linuxbrew/.linuxbrew/bin/git
    do
        if [ -x "$_bashc_git_candidate" ]; then
            printf '%s\n' "$_bashc_git_candidate"
            return 0
        fi
    done
    command -v git 2>/dev/null
}

setup_repository_and_shells() {
    _bashc_project_root=${BASHC_ROOT:-"$HOME/bashCustomization"}
    validate_single_line_path "BASHC_ROOT" "$_bashc_project_root" || return 1

    if [ ! -d "$_bashc_project_root" ]; then
        if ! _bashc_git=$(find_git); then
            echo "Error: git is required to clone bashCustomization after tool setup" >&2
            return 1
        fi
        echo "Cloning bashCustomization to ${_bashc_project_root}..."
        "$_bashc_git" clone "https://github.com/${REPO}.git" "$_bashc_project_root"
    elif [ ! -f "$_bashc_project_root/main.sh" ]; then
        echo "Error: ${_bashc_project_root} exists but does not contain main.sh" >&2
        return 1
    fi

    _bashc_project_root=$(CDPATH='' cd -P -- "$_bashc_project_root" && pwd -P)

    add_startup_hook "$HOME/.bashrc" "$_bashc_project_root"
    add_startup_hook "$HOME/.zshrc" "$_bashc_project_root"

    echo "Shell startup configured for Bash and Zsh."
    echo "Start a new shell or source ${_bashc_project_root}/main.sh to load the framework."
}

add_startup_hook() {
    _bashc_startup_file=$1
    _bashc_hook_project_root=$2
    validate_single_line_path "project root" "$_bashc_hook_project_root" || return 1
    _bashc_startup_assignment="export BASHC_ROOT=$(shell_quote "$_bashc_hook_project_root")"
    _bashc_legacy_assignment="export BASHC_ROOT=\"$_bashc_hook_project_root\""

    if [ -f "$_bashc_startup_file" ] && \
       { grep -F -x "$_bashc_startup_assignment" "$_bashc_startup_file" >/dev/null 2>&1 || \
         grep -F -x "$_bashc_legacy_assignment" "$_bashc_startup_file" >/dev/null 2>&1; }; then
        return 0
    fi

    {
        printf '\n# bashCustomization\n'
        printf '%s\n' "$_bashc_startup_assignment"
        # These variables must be expanded by the user's future shell.
        # shellcheck disable=SC2016
        printf 'if [ -f "$BASHC_ROOT/main.sh" ]; then\n'
        # shellcheck disable=SC2016
        printf '    . "$BASHC_ROOT/main.sh"\n'
        printf 'fi\n'
    } >> "$_bashc_startup_file"
}

run_requested_action() {
    _bashc_setup_after_install=false
    if [ $# -eq 0 ]; then
        set -- install all
        _bashc_setup_after_install=true
    elif [ "$#" -eq 2 ] && [ "$1" = "install" ] && [ "$2" = "all" ]; then
        _bashc_setup_after_install=true
    fi

    echo "Running: ${BINARY_NAME} $*"
    if "$PERSISTENT_BINARY" "$@"; then
        _bashc_command_status=0
    else
        _bashc_command_status=$?
    fi

    _bashc_setup_status=0
    if [ "$_bashc_setup_after_install" = true ]; then
        if setup_repository_and_shells; then
            _bashc_setup_status=0
        else
            _bashc_setup_status=$?
        fi
    fi

    if [ "$_bashc_command_status" -ne 0 ]; then
        return "$_bashc_command_status"
    fi
    return "$_bashc_setup_status"
}

# --- Main ---

main() {
    OS=$(detect_os)
    ARCH=$(detect_arch)
    TARGET="${ARCH}-${OS}"

    echo "Detected platform: ${TARGET}"

    # Bootstrap doas on Alpine when running as root with no privilege-escalation tool
    bootstrap_doas_alpine

    install_bashc_binary "$TARGET" || return 1

    run_requested_action "$@"

    echo ""
    echo "Done. bashc is installed at ${PERSISTENT_BINARY}."
}

if [ "${BASHC_INIT_SOURCE_ONLY:-}" != "1" ]; then
    main "$@"
fi
