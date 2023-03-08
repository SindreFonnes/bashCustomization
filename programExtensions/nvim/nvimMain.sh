if [[ $bashC != "" ]]; then
    export NVIM_EXTENTION_FOLDER_LOCATION=$bashC/programExtensions/nvim;
else
    export NVIM_EXTENTION_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

source $NVIM_EXTENTION_FOLDER_LOCATION/nvimAliases.sh;