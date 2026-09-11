#!/bin/sh

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)

selection=${1:---interactive}
if [ "$#" -gt 0 ]; then
    shift
fi

case "$selection" in
    1) tool=go ;;
    2) tool=dotnet ;;
    3) tool=rust ;;
    4|js|node) tool=javascript ;;
    5) tool=java ;;
    6) tool=azure ;;
    7) tool=github ;;
    8) tool=terraform ;;
    9) tool=brew ;;
    10) tool=docker ;;
    11|nvim) tool=neovim ;;
    12) tool=postgres ;;
    13|k8s|kubernetes) tool=kubectl ;;
    14) tool=obsidian ;;
    15|rg) tool=ripgrep ;;
    16) tool=bat ;;
    17) tool=fd ;;
    18) tool=eza ;;
    19) tool=shellcheck ;;
    20) tool=all ;;
    *) tool=$selection ;;
esac

# The old JavaScript submenu accepted a component name. `bashc` reconciles the
# complete JavaScript toolchain, so consume that legacy selector when present.
if [ "$tool" = javascript ]; then
    case ${1:-} in
        nvm|pnpm|yarn|bun|all) shift ;;
    esac
fi

exec "$script_dir/runBashcInstaller.sh" "$tool" "$@"
