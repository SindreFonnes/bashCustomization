if [[ $bashC != "" ]]; then
    export MY_YARN_EXTENTION_FOLDER_LOCATION=$bashC/programExtensions/yarn;
else
    export MY_YARN_EXTENTION_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

source $MY_YARN_EXTENTION_FOLDER_LOCATION/yarnAliases.sh;
# Not yet needed
# source $MY_YARN_EXTENTION_FOLDER_LOCATION/yarnFunctions.sh;
