#!/bin/bash

set -eo pipefail;

wget -O temp.deb "https://discord.com/api/download/stable?platform=linux&format=deb" && sudo apt install ./temp.deb && rm temp.deb

