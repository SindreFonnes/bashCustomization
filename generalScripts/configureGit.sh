#!/bin/bash

set -eo pipefail;

echo "What is your email?";
read email;

echo "What is your full name?"
read name;

echo "Do you want to use rebase on pull? (y/n, or enter for yes)"
read rebase;

if [[ $rebase == "y" || $rebase == "Y" || $rebase == "" ]]; then
	rebase="true";
else
	rebase="false";
fi

echo "Do you want to use vim as the default editor? (y/n, or enter for yes)"
read answer;

if [[ $answer == "y" || $answer == "Y" || $answer == "" ]]; then
	editor="vim";
else
	echo "Specify desired editor (for example: nano)"
	read editor;
fi

echo "Specify default branch name (press enter for main to be default branch)"
read branch;

if [[ $branch == "" ]]; then
	branch="main";
fi

git config --global user.email "$email"
git config --global user.name "$name"
git config --global pull.rebase "$rebase"
git config --global core.editor "$editor"
git config --global init.defaultBranch "$branch"

echo "Configured git";

exit 0;