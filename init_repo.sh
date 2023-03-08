#!/bin/bash

set -o pipefail;

bashrc=$(cat "$HOME/.bashrc");

if [[ $bashrc == *"bashCustomization"* ]]; then
    echo "The extention is allready added to the bashrc";
	exit 0;
fi

echo -e "if [ -f ~/bashCustomization/main.sh ]; then\n. ~/bashCustomization/main.sh\nfi\n" >> $HOME/.bashrc;

bashC="$HOME/bashCustomization";

# Adding git configuration
chmod +x $bashC/gitCustomizations/configureGit.sh;
$bashC/gitCustomizations/configureGit.sh;

# Adding sorting folders
mkdir $HOME/p;

p_home=$HOME/p;

mkdir $p_home/javascript;
mkdir $p_home/rust;
mkdir $p_home/go;
mkdir $p_home/dotnet;
mkdir $p_home/notes;

if [[ "$OSTYPE" == *"darwin"* ]]; then
	echo "Update install scripts to support mac you lazy bum. Exiting init script...";
	exit 0;
fi

# Install various packages
chmod +x $bashC/generalScripts/installStuff.sh;
$bashC/generalScripts/installStuff.sh;

# Run all the install scripts
source $bashC/installScripts/installMain.sh;
use_install_script all;

echo "Added the customization to bashrc, use the command \"source ~/.bashrc\" to reload the shell and load the customizations";
