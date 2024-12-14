#!/bin/bash

input=($1);

if [[ $input == "" ]]; then
	echo "You need to actually specify what you want to run";
	exit 1;
fi

run_install_script () {
    chmod +x "$1" &&
    $1;
}

script_names=( \
	"installNerdFont" \
	"installStuff" \
	"nvimSetup" \
	"setupZsh" \
	"updateOs" \
	"configureGit" \
	"fix_docker_insuficient_permissions_wsl" \
	"generateSSLCert" \
	"updateDiscord" \
)

case "${input}" in
	"${script_names[0]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[0]}.sh"
		;;
	"${script_names[1]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[1]}.sh"
		;;
	"${script_names[2]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[2]}.sh"
		;;
	"${script_names[3]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[3]}.sh"
		;;
	"${script_names[4]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[4]}.sh"
		;;
	"${script_names[5]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[5]}.sh"
		;;
	"${script_names[6]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[6]}.sh"
		;;
	"${script_names[7]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[7]}.sh"
		;;
	"${script_names[8]}")
		run_install_script "${GENERAL_SCRIPTS_FOLDER_LOCATION}/${script_names[8]}.sh"
		;;
	"help")
		echo "Here are all the script options";
		for cmd in ${script_names[@]}
		do
			echo $cmd;
		done
		;;
	*)
		echo "Invalid option"
		;;
esac;

exit 0;