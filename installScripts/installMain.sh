if [[ $bashC != "" ]]; then
    export MYINSTALL_SCRIPT_FOLDER_LOCATION=$bashC/installScripts;
else
    export MYINSTALL_SCRIPT_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

export MYINSTALL_COMMON_FUNCTIONS_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/commonMyinstallFunctions.sh;
export MYINSTALL_SCRIPT_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/installScript.sh

source "$MYINSTALL_SCRIPT_FOLDER_LOCATION/installAliases.sh";

run_my_install () {
    # Prefer bashc Rust binary when available
    if command -v bashc &> /dev/null; then
        bashc install "$@";
        return $?;
    fi

    # Fall back to shell script dispatch
    "$MYINSTALL_SCRIPT_LOCATION" "$@";
}

if [[ $PROFILE_SHELL == "bash" ]]; then
    export -f run_my_install;
fi
