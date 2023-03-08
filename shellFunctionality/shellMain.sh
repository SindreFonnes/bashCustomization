if [[ $bashC != "" ]]; then
    export SHELL_EXTENTION_FOLDER_LOCATION=$bashC/shellFunctionality;
else
    export SHELL_EXTENTION_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

source $SHELL_EXTENTION_FOLDER_LOCATION/shellFunctions.sh;
source $SHELL_EXTENTION_FOLDER_LOCATION/shellAliases.sh;


