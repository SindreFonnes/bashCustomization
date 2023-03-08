#!/bin/bash

## This script is meant to update ubuntu based distros (or wsl). Other debian based distros might work, but has not been tested
set -eo pipefail;

if ! command -v do-release-upgrade &> /dev/null; then
    sudo apt install update-manager-core;
fi

sudo do-release-upgrade -d;
