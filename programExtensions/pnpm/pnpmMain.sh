if [[ $bashC != "" ]]; then
    export PNPM_EXTENTION_FOLDER_LOCATION=$bashC/programExtensions/pnpm;
else
    export PNPM_EXTENTION_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

source $PNPM_EXTENTION_FOLDER_LOCATION/pnpmAliases.sh;
# Not yet needed
# source $PNPM_EXTENTION_FOLDER_LOCATION/pnpmFunctions.sh;
