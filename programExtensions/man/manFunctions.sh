function open_man_in_vscode() { 
	if ! command -v code &> /dev/null; then
		echo "vscode is not installed";
		return 0;
	fi

	man "$1" | col -bx | code -;
}