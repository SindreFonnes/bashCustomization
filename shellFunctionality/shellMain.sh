if [[ $bashC != "" ]]; then
    export SHELL_EXTENTION_FOLDER_LOCATION=$bashC/shellFunctionality;
else
    export SHELL_EXTENTION_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

_bashc_source_file "$SHELL_EXTENTION_FOLDER_LOCATION/shellFunctions.sh" || return 1
_bashc_source_file "$SHELL_EXTENTION_FOLDER_LOCATION/shellAliases.sh" || return 1

