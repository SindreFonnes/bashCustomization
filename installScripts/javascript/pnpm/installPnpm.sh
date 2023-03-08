#!/bin/bash

set -eo pipefail;

curl -fsSL https://get.pnpm.io/install.sh | sh - &&
echo "Finished installing pnpm";