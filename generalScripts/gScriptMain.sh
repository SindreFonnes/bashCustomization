if [[ $bashC != "" ]]; then
    export GENERAL_SCRIPTS_FOLDER_LOCATION=$bashC/generalScripts;
else
    export GENERAL_SCRIPTS_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

_bashc_source_file "$GENERAL_SCRIPTS_FOLDER_LOCATION/gScriptAliases.sh" || return 1

run_general_script () {
	"$GENERAL_SCRIPTS_FOLDER_LOCATION/gScriptRun.sh" "$@";
}
