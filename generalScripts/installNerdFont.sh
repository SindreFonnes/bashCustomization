#!/bin/bash

set -eo pipefail

announce_success () {
	echo "Finished installing JetbrainsMono font";
	exit 0;
};

install_for_mac () {
	brew tap homebrew/cask-fonts;

	brew install --cask font-jetbrains-mono-nerd-font;

	fc-cache -fv;

	announce_success;
}

install_for_other () {
	local FONT_LOCATION="$HOME/.local/share/fonts"
	local FONT_ZIP_NAME="JetBrainsMono.zip";

	if ! [ -d $FONT_LOCATION ]; then 
		mkdir $FONT_LOCATION;
	fi

	wget https://github.com/ryanoasis/nerd-fonts/releases/download/v2.3.3/$FONT_ZIP_NAME \
	 -P $FONT_LOCATION;

	local FONT_ZIP_PATH;
	
	unzip $FONT_ZIP_PATH -d $FONT_LOCATION;

	rm $FONT_ZIP_PATH;

	fc-cache -fv;

	announce_success;
}

install_fontconfig_apt () {
	sudo apt install fontconfig;
}

install_fontconfig_mac () {
	brew install fontconfig;
}

if [[ "$OSTYPE" == *"darwin"* ]]; then
    if ! command -v brew &> /dev/null; then
	    echo "Missing brew. Please install before running this script";
		exit 1;
	fi

	if ! command -v fc-cache &> /dev/null; then
		install_fontconfig_mac;
	fi

	install_for_mac;

else
	if ! command -v fc-cache &> /dev/null; then
		if ! command -v apt &> /dev/null; then
			echo "Non-apt and brew systems are currently not supported."
			echo "If you wish to use this script for these unsuported systems, then you will have to add support for them to this script";
			exit 1;
		fi

		install_fontconfig_apt;
	fi

	install_for_other;
fi


echo "Something went wrong";

exit 1;
