if [[ $bashC != "" ]]; then
    export MYINSTALL_SCRIPT_FOLDER_LOCATION=$bashC/installScripts;
else
    export MYINSTALL_SCRIPT_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

export MYINSTALL_COMMON_FUNCTIONS_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/commonMyinstallFunctions.sh;
export MYINSTALL_SCRIPT_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/installScript.sh

source $MYINSTALL_SCRIPT_FOLDER_LOCATION/installAliases.sh;

run_my_install () {
    $MYINSTALL_SCRIPT_LOCATION $1 $2;
}

export -f run_my_install;
