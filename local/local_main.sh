# Functions
_bashc_local_shell_quote () {
	printf "'%s'" "$(printf '%s' "$1" | sed "s/'/'\\\\''/g")"
}

add_local_variable () {
	local variable_name=$1
	case "$variable_name" in
		''|[!a-zA-Z_]*|*[!a-zA-Z0-9_]*)
			printf 'bashc: invalid local variable name: %s\n' "$variable_name" >&2
			return 1
			;;
	esac

	if [[ ${3:-false} != "false" ]]; then
		variable_name="local_$variable_name"
	fi

	printf '%s=%s\n' "$variable_name" "$(_bashc_local_shell_quote "$2")" >> "$local_dir/local_variables.sh"
}

add_local_alias () {
	local alias_name=$1
	case "$alias_name" in
		''|*[!a-zA-Z0-9_.-]*)
			printf 'bashc: invalid local alias name: %s\n' "$alias_name" >&2
			return 1
			;;
	esac

	if [[ ${3:-false} != "false" ]]; then
		alias_name="local_$alias_name"
	fi

	printf 'alias %s=%s\n' "$alias_name" "$(_bashc_local_shell_quote "$2")" >> "$local_dir/local_aliases.sh"
}

# Aliases for managing local
alias editLocalVariable="$standard_editor $local_dir/local_variables.sh";
alias editLocalAliases="$standard_editor $local_dir/local_aliases.sh";

alias alv="add_local_variable";
alias ala="add_local_alias";
alias listLocalA="cat $local_dir/local_aliases.sh";
alias listLocalV="cat $local_dir/local_variables.sh";

# Creating the local files if they do not already exist (owner-only permissions)
if ! [ -f "$local_dir/local_variables.sh" ]; then
	touch "$local_dir/local_variables.sh";
fi
chmod 600 "$local_dir/local_variables.sh";

_bashc_source_file "$local_dir/local_variables.sh" || return 1

if ! [ -f "$local_dir/local_aliases.sh" ]; then
	touch "$local_dir/local_aliases.sh";
fi
chmod 600 "$local_dir/local_aliases.sh";

_bashc_source_file "$local_dir/local_aliases.sh" || return 1
