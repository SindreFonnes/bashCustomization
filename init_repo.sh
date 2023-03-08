#!/bin/bash

set -o pipefail;

cat ~/.bashrc | grep "bashCustomization" &> /dev/null

if [[ $? -eq 0 ]]; then
    echo "The extention is allready added to the bashrc";
	exit 0;
fi

echo -e "if [ -f ~/bashCustomization/main.sh ]; then\n. ~/bashCustomization/main.sh\nfi\n" >> ~/.bashrc;

# Adding git configuration
chmod +x ~/bashCustomization/gitCustomizations/configureGit.sh;
~/bashCustomization/gitCustomizations/configureGit.sh;

# Adding sorting folders
mkdir ~/p;

p_home=~/p;

mkdir $p_home/javascript;
mkdir $p_home/rust;
mkdir $p_home/go;
mkdir $p_home/dotnet;
mkdir $p_home/notes;

echo "$OSTYPE" | grep "darwin" &> /dev/null;

if [[ $? -eq 0 ]]; then
	echo "Update install scripts to support mac you lazy bum. Exiting init script...";
	exit 0;
fi

# Install various packages
chmod +x ~/bashCustomization/generalScripts/installStuff.sh;
~/bashCustomization/generalScripts/installStuff.sh;

# Run all the install scripts
. ~/bashCustomization/installScripts/installMain.sh;
use_install_script all;

echo "Added the customization to bashrc, use the command \"source ~/.bashrc\" to reload the shell and load the customizations"
echo "You should also probably clone down .app_data into ~/";
