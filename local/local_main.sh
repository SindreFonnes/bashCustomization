# Functions
add_local_variable () {
	if [[ $3 -eq "false" ]]; then
		echo -e "$1=\"$2\"" >> $local_dir/local_variables.sh;
		return;
	fi

	echo -e "local_$1=\"$2\"" >> $local_dir/local_variables.sh;
}

add_local_alias () {
	if [[ $3 -eq "false" ]]; then
		echo -e "alias $1=\"$2\";" >> $local_dir/local_aliases.sh;
		return;
	fi

	echo -e "alias local_$1=\"$2\";" >> $local_dir/local_aliases.sh;
}

# Aliases for managing local
alias editLocalVariable="$standard_editor $local_dir/local_variables.sh";
alias editLocalAliases="$standard_editor $local_dir/local_aliases.sh";

alias alv="add_local_variable";
alias ala="add_local_alias";
alias listLocalA="cat $local_dir/local_aliases.sh";
alias listLocalV="cat $local_dir/local_variables.sh";

# Creating the local files if the do not allready exist
if ! [ -f "$local_dir/local_variables.sh" ]; then
    touch "$local_dir/local_variables.sh";
fi

. "$local_dir/local_variables.sh"

if ! [ -f "$local_dir/local_aliases.sh" ]; then
    touch "$local_dir/local_aliases.sh";
fi

. "$local_dir/local_aliases.sh"