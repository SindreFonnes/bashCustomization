if [[ $bashC != "" ]]; then
    export RUST_EXTENTION_FOLDER_LOCATION=$bashC/programExtensions/rust;
else
    export RUST_EXTENTION_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

source $RUST_EXTENTION_FOLDER_LOCATION/rustAliases.sh;
# Not yet needed
# source $RUST_EXTENTION_FOLDER_LOCATION/rustFunctions.sh;
