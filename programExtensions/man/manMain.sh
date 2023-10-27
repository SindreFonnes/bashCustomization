if [[ $bashC != "" ]]; then
    export MAN_EXTENTION_FOLDER_LOCATION=$bashC/programExtensions/man;
else
    export MAN_EXTENTION_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

source $MAN_EXTENTION_FOLDER_LOCATION/manFunctions.sh;
source $MAN_EXTENTION_FOLDER_LOCATION/manAliases.sh;