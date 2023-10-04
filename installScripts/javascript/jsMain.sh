#!/bin/bash

set -eo pipefail;

run_js_install_script () {
	chmod +x "$1" &&
	$1;
}

JSMAIN_LOCATION=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd );

NVM_INSTALL_LOCATION=$JSMAIN_LOCATION/nvm/installNvm.sh;
PNPM_INSTALL_LOCATION=$JSMAIN_LOCATION/pnpm/installPnpm.sh;
YARN_INSTALL_LOCATION=$JSMAIN_LOCATION/yarn/installYarn.sh;
BUN_INSTALL_LOCATION=$JSMAIN_LOCATION/bun/installBun.sh;
options=("nvm" "pnpm" "yarn" "bun");

determine_install_script_to_use () {
	case "$1" in
		"${options[0]}" | "1")
			run_js_install_script $NVM_INSTALL_LOCATION;;
		"${options[1]}" | "2")
			run_js_install_script $PNPM_INSTALL_LOCATION;;
		"${options[2]}" | "3")
			run_js_install_script $YARN_INSTALL_LOCATION;;
		"${options[3]}" | "4")
			run_js_install_script $BUN_INSTALL_LOCATION;;
		"all")
			run_js_install_script $NVM_INSTALL_LOCATION;
			run_js_install_script $PNPM_INSTALL_LOCATION;
			run_js_install_script $YARN_INSTALL_LOCATION;;
		*)
			echo "Not a valid option $REPLY";;
	esac
	exit 0;
}

if [[ $# -ne 0 && $1 != "" && $1 != "unfilled_value" ]]; then
	determine_install_script_to_use $1
fi

echo "What js package manager do you want to install?";

select choice in "${options[@]}"; do
	determine_install_script_to_use $REPLY;
done
