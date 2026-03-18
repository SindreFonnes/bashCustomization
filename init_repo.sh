#!/bin/bash

set -o pipefail;

bashrc=$(cat "$HOME/.bashrc");

if [[ $bashrc == *"bashCustomization"* ]]; then
    echo "The extention is allready added to the bashrc";
	exit 0;
fi

echo -e "if [ -f ~/bashCustomization/main.sh ]; then\n. ~/bashCustomization/main.sh\nfi\n" >> "$HOME/.bashrc";

bashC="$HOME/bashCustomization";

# Adding git configuration
chmod +x "$bashC/generalScripts/configureGit.sh";
"$bashC/generalScripts/configureGit.sh";

# Adding sorting folders
mkdir -p "$HOME/p";

p_home="$HOME/p";

mkdir -p "$p_home/javascript";
mkdir -p "$p_home/rust";
mkdir -p "$p_home/go";
mkdir -p "$p_home/dotnet";
mkdir -p "$p_home/notes";

# Install various packages
chmod +x "$bashC/generalScripts/installStuff.sh";
"$bashC/generalScripts/installStuff.sh";

echo "Added the customization to bashrc, use the command \"source ~/.bashrc\" to reload the shell and load the customizations";
