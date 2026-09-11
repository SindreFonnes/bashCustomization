#!/bin/sh

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
[ "${1:-}" = "-y" ] && shift
exec "$script_dir/../runBashcInstaller.sh" neovim "$@"
