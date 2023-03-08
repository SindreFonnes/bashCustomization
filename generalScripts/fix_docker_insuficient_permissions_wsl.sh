#!/bin/bash

set -eo pipefail;

sudo groupadd docker;
sudo usermod -aG docker $USER;

echo "All done, restart your shell (properly) for the desired effect."

exit 0;