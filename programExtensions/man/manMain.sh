if [[ $bashC != "" ]]; then
    export MAN_EXTENTION_FOLDER_LOCATION=$bashC/programExtensions/man;
else
    export MAN_EXTENTION_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

_bashc_source_file "$MAN_EXTENTION_FOLDER_LOCATION/manFunctions.sh" || return 1
_bashc_source_file "$MAN_EXTENTION_FOLDER_LOCATION/manAliases.sh" || return 1
