#!/bin/bash

set -eo pipefail;

if ! [[ -f $bashC/gitCustomizations/.gitconfig ]]; then
	echo "Can't find the config template file.";
	echo "Exiting....";
	exit 1;
fi

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

# \n = newline \t = horizontal tab
config="[user]\n
\temail = $email\n
\tname = $name\n
\n
[pull]\n
\trebase = $rebase\n
\n
[core]\n
\teditor = $editor\n
\n
[init]\n
\tdefaultBranch = $branch"

echo -e $config > ~/.gitconfig;

echo "Configured git";

exit 0;