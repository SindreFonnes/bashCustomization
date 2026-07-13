# shellcheck shell=bash

if [[ -n ${bashC:-} ]]; then
    export MYINSTALL_SCRIPT_FOLDER_LOCATION=$bashC/installScripts;
else
    # BASH_SOURCE is also populated by zsh when it sources this file.
    # shellcheck disable=SC2128
    MYINSTALL_SCRIPT_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
    export MYINSTALL_SCRIPT_FOLDER_LOCATION
fi

export MYINSTALL_COMMON_FUNCTIONS_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/commonMyinstallFunctions.sh;
export MYINSTALL_SCRIPT_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/installScript.sh

_bashc_source_file "$MYINSTALL_SCRIPT_FOLDER_LOCATION/installAliases.sh" || return 1

run_my_install () {
    if command -v bashc &> /dev/null; then
        bashc install "$@";
        return $?;
    fi

    printf 'bashc: the Rust binary is required for supported installs; run %s/init.sh or build %s/rust\n' "$bashC" "$bashC" >&2
    printf 'bashc: legacy installScripts are retained as reference material and are not a verified fallback\n' >&2
    return 1
}

if [[ $PROFILE_SHELL == "bash" ]]; then
    export -f run_my_install;
fi
