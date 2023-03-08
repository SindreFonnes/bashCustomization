#!/bin/bash

set -eo pipefail;

# Source of package, and described use:
# https://stackoverflow.com/questions/20392243/run-c-sharp-code-on-linux-terminal

# Package home website:
# https://www.mono-project.com/

# Note! Seems to only support upto .NET 5 as of time of writing.
# Compatability docs:
# https://www.mono-project.com/docs/about-mono/compatibility/

# Github repo
# https://github.com/mono/mono

sudo apt update;
sudo apt install mono-complete;

echo "Finished installing mono-complete. You can now use \"mcs\" to compile c# files, and \"mono 'exefilename.exe'\" to run the executable";
exit 0;