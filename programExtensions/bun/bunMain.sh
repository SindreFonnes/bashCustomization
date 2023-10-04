if [[ $bashC != "" ]]; then
    export MY_BUN_EXTENTION_FOLDER_LOCATION=$bashC/programExtensions/bun;
else
    export MY_BUN_EXTENTION_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

source $MY_BUN_EXTENTION_FOLDER_LOCATION/bunAliases.sh;
# Not yet needed
# source $MY_BUN_EXTENTION_FOLDER_LOCATION/yarnFunctions.sh;
