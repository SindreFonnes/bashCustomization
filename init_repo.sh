#!/bin/sh

# Compatibility entry point for the pre-Rust bootstrap name. Keep all setup on
# the verified init.sh path instead of duplicating bootstrap logic in the shell
# compatibility launchers.
script_root=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
exec "$script_root/init.sh" "$@"
