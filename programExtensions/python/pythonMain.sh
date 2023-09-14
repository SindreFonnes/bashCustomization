if [[ $bashC != "" ]]; then
    export PYTHON_EXTENTION_FOLDER_LOCATION=$bashC/programExtensions/python;
else
    export PYTHON_EXTENTION_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

source $PYTHON_EXTENTION_FOLDER_LOCATION/pythonAliases.sh;

