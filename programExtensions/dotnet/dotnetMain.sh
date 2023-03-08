if [[ $bashC != "" ]]; then
    export DOTNET_EXTENTION_FOLDER_LOCATION=$bashC/programExtensions/dotnet;
else
    export DOTNET_EXTENTION_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

source $DOTNET_EXTENTION_FOLDER_LOCATION/dotnetAliases.sh;
# Not yet needed
# source $DOTNET_EXTENTION_FOLDER_LOCATION/dotnetFunctions.sh;
