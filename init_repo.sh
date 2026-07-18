#!/bin/sh

# Compatibility entry point for the pre-Rust bootstrap name. Keep all setup on
# the verified init.sh path instead of executing the historical installer
# scripts retained under installScripts/ and generalScripts/.
script_root=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
exec "$script_root/init.sh" "$@"
