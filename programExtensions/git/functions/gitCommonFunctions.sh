source $SHELL_EXTENTION_FOLDER_LOCATION/shellFunctions.sh;

git_commit () {
	local inputs=($@);
	local parsedInput=${inputs[@]};
	git commit -m "$parsedInput";
}

git_add_commit () {
	local inputs=($@);
	local parsedInput=${inputs[@]};

	git add -A &&
	git commit -m "$parsedInput";
}

git_add_commit_push () {
	local inputs=($@);
	start_or_install_keychain &&
	git_add_commit ${inputs[@]} &&
	git push;
}

git_pull () {
	start_or_install_keychain &&
	git pull;
}

git_clone () {
	git clone $@;
}

git_checkout () {
	git fetch --all &&
	git checkout "$1";
}

git_checkout_new_branch () {
	git fetch --all &&
	git checkout -b "$1";
}

git_push () {
	start_or_install_keychain &&
	git push;
}

git_diff_bashc () {
	git_diff_func () {
		git diff;
	}
	execute_command_in_folder_and_go_back git_diff_func $bashC
}

git_get_latest_commit () {
	git log --pretty=oneline | head -n 1 | cat;
}

# Deprecated
# Overengineered and git_tag_commit seems to be doing the job just fine
git_tag_latest_commit () {
	git rev-parse --is-inside-work-tree &> /dev/null;

	if [[ $? -ne 0 ]]; then
		echo "You are not inside a git repo...";
		return;
	fi

	echo "This is the latest tag:";
	git tag | tail -n 1 | cat;

	echo "Which version would you like to tag?";

	local newTag;
	read newTag;
	
	local latestCommit=$(git log --pretty=oneline | head -n 1);
	local array=($latestCommit);

	echo "This will be your tag:"
	echo "v$newTag";
	echo "";
	echo "It will be added to the commmit:";
	echo $latestCommit;
	echo "";
	echo "Are you happy with this?(y/n)";

	local happy
	read happy;

	if [[ $happy != "y" ]]; then
		echo "Exiting...";
		return;
	fi

	git tag -a "v$newTag" ${array[0]} &&
	git push origin "v$newTag";
}

git_tag_commit () {
	git rev-parse --is-inside-work-tree &> /dev/null;

	if [[ $? -ne 0 ]]; then
		echo "You are not inside a git repo...";
		return;
	fi

	if [[ $1 == "" ]]; then
		echo "You must provide a verion number";
		return;
	fi

	git tag "v$1" &&
	git push origin "v$1";
}

git_delete_tag () {
	if [[ $1 == "" ]]; then
		echo "You must provide an argument. Exiting...";
		return;
	fi

	git tag -d $1;
	git push --delete origin $1;
}

git_commit_and_push_project () {
	local inputs=($@);
	local destination="$1";
	local commitMessage="${inputs[@]:1}";
	git_func () {
		git_add_commit_push "$commitMessage"
	}
	execute_command_in_folder_and_go_back git_func $destination;
}

git_pull_repo () {
	execute_command_in_folder_and_go_back git_pull $1
}
