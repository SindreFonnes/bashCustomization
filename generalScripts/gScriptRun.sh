#!/bin/bash

input=${1:-};
status=0;

if [[ $input == "" ]]; then
	echo "You need to actually specify what you want to run";
	exit 1;
fi

run_install_script () {
    chmod +x "$1" &&
    $1;
}

run_supported_install () {
	if ! command -v bashc >/dev/null 2>&1; then
		printf 'bashc: the Rust binary is required for supported installs; run init.sh first\n' >&2
		return 1
	fi
	bashc install "$1"
}

script_names=( \
	"installNerdFont" \
	"installStuff" \
	"nvimSetup" \
	"setupZsh" \
	"updateOs" \
	"configureGit" \
	"setupGpgSigning" \
	"fix_docker_insuficient_permissions_wsl" \
	"generateSSLCert" \
	"updateDiscord" \
	"launchSteam" \
)

case "${input}" in
	"${script_names[0]}")
		run_supported_install nerd-font || status=$?
		;;
	"${script_names[1]}")
		run_supported_install base || status=$?
		;;
	"${script_names[2]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[2]}.sh" || status=$?
		;;
	"${script_names[3]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[3]}.sh" || status=$?
		;;
	"${script_names[4]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[4]}.sh" || status=$?
		;;
	"${script_names[5]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[5]}.sh" || status=$?
		;;
	"${script_names[6]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[6]}.sh" || status=$?
		;;
	"${script_names[7]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[7]}.sh" || status=$?
		;;
	"${script_names[8]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[8]}.sh" || status=$?
		;;
	"${script_names[9]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[9]}.sh" || status=$?
		;;
	"${script_names[10]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[10]}.sh" || status=$?
		;;
	"help")
		echo "Here are all the script options";
		for cmd in "${script_names[@]}"
		do
			echo "$cmd";
		done
		;;
	*)
		echo "Invalid option" >&2
		status=1
		;;
esac;

exit "$status";
