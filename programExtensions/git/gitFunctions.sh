GIT_FUNCTION_FOLDER=$GIT_EXTENTION_FOLDER_LOCATION/functions;

source $GIT_FUNCTION_FOLDER/gitCommonFunctions.sh;

git_add_commit_push_tag () {
	$GIT_FUNCTION_FOLDER/gitAddCommitPushTag.sh ${@};
}
