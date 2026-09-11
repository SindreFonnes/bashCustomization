if [[ $bashC != "" ]]; then
    export GIT_EXTENTION_FOLDER_LOCATION=$bashC/programExtensions/git;
else
    export GIT_EXTENTION_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

_bashc_source_file "$GIT_EXTENTION_FOLDER_LOCATION/gitAliases.sh" || return 1
_bashc_source_file "$GIT_EXTENTION_FOLDER_LOCATION/gitFunctions.sh" || return 1
_bashc_source_file "$GIT_EXTENTION_FOLDER_LOCATION/ilyaFunctions.sh" || return 1
