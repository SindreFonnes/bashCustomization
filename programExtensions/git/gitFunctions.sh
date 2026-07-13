GIT_FUNCTION_FOLDER=$GIT_EXTENTION_FOLDER_LOCATION/functions;

_bashc_source_file "$GIT_FUNCTION_FOLDER/gitCommonFunctions.sh" || return 1

git_add_commit_push_tag () {
	$GIT_FUNCTION_FOLDER/gitAddCommitPushTag.sh ${@};
}
