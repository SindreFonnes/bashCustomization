#!/bin/bash

source $SHELL_EXTENTION_FOLDER_LOCATION/shellFunctions.sh;

find_aliases_and_functions () {
	local all_git_extention_files=($(get_all_files_bellow_directory $GIT_EXTENTION_FOLDER_LOCATION));

	oIFS=$IFS;
	IFS=$'\n';

	local functions=();
	local aliases=();

	local full_git_extention_code=();
	
	for entry in ${all_git_extention_files[@]}; do
		if [[ $entry == *"gitShow"* || $entry == *"gitAddCommitPushTag"* ]]; then
			continue;
		fi

		for line in $(cat $GIT_EXTENTION_FOLDER_LOCATION/$entry); do
			full_git_extention_code+=("$line");
		done;
	done;

	IFS=$oIFS;

	for line in "${full_git_extention_code[@]}"; do
		if [[ $line == *") {"* ]]; then
			functions+=(${line%%"()"*});
		elif [[ $line == *"alias"* ]]; then
			aliases+=("${line#"alias"}");
		fi
	done

	echo "Git functions:";
	for entry in "${functions[@]}"; do
		echo "	$entry";
	done

	echo "";

	echo "Git aliases";
	for ((i = 0; i < ${#aliases[@]}; i++ )); do
		echo "	${aliases[$i]%%";"*}";
	done
}

find_aliases_and_functions;
