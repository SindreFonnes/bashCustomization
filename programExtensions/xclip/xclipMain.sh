if [[ $bashC != "" ]]; then
    export XCLIP_EXTENTION_FOLDER_LOCATION=$bashC/programExtensions/xclip;
else
    export XCLIP_EXTENTION_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

source $XCLIP_EXTENTION_FOLDER_LOCATION/xclipAliases.sh;