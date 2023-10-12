if [[ $bashC != "" ]]; then
    export GIT_EXTENTION_FOLDER_LOCATION=$bashC/programExtensions/git;
else
    export GIT_EXTENTION_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

source $GIT_EXTENTION_FOLDER_LOCATION/gitAliases.sh;
source $GIT_EXTENTION_FOLDER_LOCATION/gitFunctions.sh;
source $GIT_EXTENTION_FOLDER_LOCATION/ilyaFunctions.sh;
