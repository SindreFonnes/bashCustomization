#!/bin/bash

source $GIT_EXTENTION_FOLDER_LOCATION/functions/gitCommonFunctions.sh;

is_greater_than_current_version () {
	local current_version=$(cat ./package.json | sed -n "/\"version\"/p" | tr -dc '[0-9.]');
	local current_version_array=($(echo $current_version | tr . " "))

	local new_version_array=($(echo $1 | tr . " "));

	if [[ 
		${current_version_array[0]} -lt ${new_version_array[0]}
	]] || [[  
		! ${current_version_array[0]} -gt ${new_version_array[0]} && 
		${current_version_array[1]} -lt ${new_version_array[1]} 
	]] || [[
		! ${current_version_array[0]} -gt ${new_version_array[0]} && 
		! ${current_version_array[1]} -gt ${new_version_array[1]} &&
		${current_version_array[2]} -lt ${new_version_array[2]} 
	]]; then
		return 0;
	else
		return 1;
	fi
}

change_package_json_version () {
	if ! [[ -f "./package.json" ]]; then
		echo "There is no package.json file in this folder";
		echo "exiting...";
		return 1;
	fi

	if [[ $1 == "" ]]; then
		echo "You did not provide a version number";
		echo "exiting...";
		return 1;
	fi

	local new_version_number="$1";

	is_greater_than_current_version $new_version_number;

	if [[ $? -ne 0 ]]; then
		echo "You must specify a newer version than the current one";
		echo "Exiting...";
		return 1;
	fi

	local currVersionLine=$(cat ./package.json | sed -n "/\"version\"/p");

	local newVersionLine="    \"version\": \"$new_version_number\",";

	if [[ "$OSTYPE" == *"darwin"* ]]; then
		sed -i "" "s/$currVersionLine/$newVersionLine/" ./package.json
	else
		sed -i "s/$currVersionLine/$newVersionLine/" ./package.json
	fi
}

git_add_commit_push_tag () {
	if [[ $1 == "" ]]; then
		echo "You must include a tag"
		echo "Exiting...";
		return 1;
	fi

	if [[ $2 == "" ]]; then
		echo "You must include a commit message"
		echo "Exiting...";
		return 1;
	fi

	local version_number="";

	if [[ $1 != *"."* ]]; then
		local current_version=$(cat ./package.json | sed -n "/\"version\"/p" | tr -dc '[0-9.]');
		local current_version_array=($(echo $current_version | tr . " "));

		if [[ $1 == "major" ]]; then
			current_version_array[0]=$((${current_version_array[0]} + 1));
			current_version_array[1]="0";
			current_version_array[2]="0";
		elif [[ $1 == "minor" ]]; then
			current_version_array[1]=$((${current_version_array[1]} + 1));
			current_version_array[2]="0";
		elif [[ $1 == "patch" ]]; then
			current_version_array[2]=$((${current_version_array[2]} + 1));
		else
			echo "Must specify either \"major\", \"minor\" or \"patch\"";
			exit 0;
		fi

		version_number="${current_version_array[0]}.${current_version_array[1]}.${current_version_array[2]}";
	fi

	if [[ version_number == "" ]]; then
		version_number="$1";
	fi

	local commitMessage="${@:2}";

	if [[ -f "./package.json" ]]; then
		change_package_json_version "$version_number";
	fi

	if [[ $? -ne 0 ]]; then
		return 1;
	fi

	git_add_commit_push $commitMessage &&
	git_tag_commit $version_number;
}

git_add_commit_push_tag ${@}
