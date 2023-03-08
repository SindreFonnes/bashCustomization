#!/bin/bash

set -eo pipefail;

if [ -f ~/.vim/.vimrc ]; then
	echo "There allready exists a vimrc file. Exiting...."
	exit 0;
fi

vimrc_path=~/.vim/

mkdir ~/.vim

touch ~/.vim/.vimrc

echo "set nocompatible" >> "$vimrc_path/.vimrc"
echo "filetype plugin on" >> "$vimrc_path/.vimrc"
echo "syntax on" >> "$vimrc_path/.vimrc"

# Install vimplug
sh -c 'curl -fLo "${XDG_DATA_HOME:-$HOME/.local/share}"/nvim/site/autoload/plug.vim --create-dirs \
       https://raw.githubusercontent.com/junegunn/vim-plug/master/plug.vim'

# Add some config to neovims settings
DOT_CONFIG_LOCATION=~/.config;
NVIM_CONFIG_DIR_LOCATION=$DOT_CONFIG_LOCATION/nvim;
INIT_VIM_LOCATION=$NVIM_CONFIG_DIR_LOCATION/init.vim;

if ! [ -d $DOT_CONFIG_LOCATION ]; then
	mkdir $DOT_CONFIG_LOCATION;
fi

if ! [ -d ~/.config/nvim ]; then
	mkdir $NVIM_CONFIG_DIR_LOCATION;
fi

if ! [ -f $INIT_VIM_LOCATION ]; then
	touch $INIT_VIM_LOCATION;
fi

CONFIG_STRING="\" Plugins will be downloaded under the specified directory.\ncall plug#begin(has('nvim') ? stdpath('data') . '/plugged' : '~/.vim/plugged')\n\n\" Declare the list of plugins.\nPlug 'tpope/vim-sensible'\nPlug 'junegunn/seoul256.vim'\nPlug 'mg979/vim-visual-multi', {'branch': 'master'}\n\n\" List ends here. Plugins become visible to Vim after this call.\ncall plug#end()";

cat $INIT_VIM_LOCATION | grep -q "\" Plugins will be downloaded";

if [[ $? -eq 0 ]]; then
	echo -e $CONFIG_STRING >> $INIT_VIM_LOCATION;

	nvim --headless +"source $INIT_VIM_LOCATION" +qall;
	nvim --headless +PlugInstall +qall;
fi
