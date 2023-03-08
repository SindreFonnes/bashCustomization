#!/bin/bash

set -eo pipefail;

name="Go";
commandAlias="go"

source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

echo "Running go install script...";

ARCH="amd64" # system archicture

is_greater_than_current_version () {
	local current_version=$(go version | grep -o -a go[0-9]*.[0-9]*.[0-9] | tr -dc "[0-9].");
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

check_if_is_installed () {
	# Checks if the 'go' command is present
	if command -v "$commandAlias" &> /dev/null; then
		if is_greater_than_current_version "$1"; then
			echo "Go is allready installed, but there is a new version available ($1)";
			echo "Do you want to uppgrade to the newest version? (y/n)";
			
			read answer;

			if [[ $answer == "y" || $answer == "Y" ]]; then
				sudo rm -rf /usr/local/go;
				return 0;
			fi
			exit 0;
		fi

		echo "Go version $VERSION is already installed. Please remove it if you want to reinstall it."
		echo "You can remove it with the command 'rm -rf /usr/local/go'";
		exit 0;
	fi
}

install_for_linux () {
	NEWEST_VERSION=$(\
		curl -sL https://go.dev/dl/ | \
		grep -A 5 -w go[0-9]*.[0-9]*.[0-9]*.src | \
		grep -zoP "(?<=<td).*(?=</td>)" | \
		grep -o -a -m 1 go[0-9]*.[0-9]*.[0-9]* | \
		grep -m 1 [0-9]*.[0-9]*.[0-9]* | \
		tr -dc '[0-9].' \
	);

	check_if_is_installed "$NEWEST_VERSION";

	if [[ -f "./go${NEWEST_VERSION}.linux-${ARCH}.tar.gz" ]]; then
		echo "Go tar is allready downloaded";
	else
		echo "Downloading go";
		curl -O -L "https://golang.org/dl/go${NEWEST_VERSION}.linux-${ARCH}.tar.gz"
	fi

	# First downloads go dl site. Then grep fetches the column with our version and architecture. Then we fetch the relevant checksum from the string we now have. Then remove null characters from the string.
	CHECKSUM=$(\
		curl -sL https://golang.org/dl/ | \
		grep -A 5 -w "go${NEWEST_VERSION}.linux-${ARCH}.tar.gz" | \
		grep -z -o -P "(?<=<tt>).*(?=</tt>)" | tr '\0' '\n' \
	)

	CHECK_RESULT=$(\
		echo "$CHECKSUM *go${NEWEST_VERSION}.linux-${ARCH}.tar.gz" | \
		shasum -a 256 --check | grep "OK" \
	)

	if [[ $CHECK_RESULT == *"OK"* ]]; then
		echo "Checksum OK";
		tar -xf "go${NEWEST_VERSION}.linux-${ARCH}.tar.gz";
		sudo chown -R root:root ./go;
		sudo mv -v go /usr/local;
		rm -rf "go${NEWEST_VERSION}.linux-${ARCH}.tar.gz";
		sudo rm -rf "go";
		echo "" >> ~/.bashrc;
		echo "#Go" >> ~/.bashrc;
		echo "export PATH=\$PATH:/usr/local/go/bin" >> ~/.bashrc;
		echo "" >> ~/.bashrc;
		script_success_message "$name";
		exit 0;
	else
		echo "Checksum NOT OK" &&
		rm -rf "go${NEWEST_VERSION}.linux-${ARCH}.tar.gz";
		exit 1;
	fi
}

if is_mac_os; then
	brew update &&
	brew install go &&
	script_success_message "$name";
	exit 0;
fi

if [[ $OSTYPE == *"linux"* ]]; then
	install_for_linux;
fi

script_does_not_support_os "$name";

exit 1;