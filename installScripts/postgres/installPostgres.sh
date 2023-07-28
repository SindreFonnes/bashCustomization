#!/bin/bash

set -eo pipefail;

name="Postgres";
commandAlias="psql";

source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

if ! script_check_if_allready_installed "$commandAlias" "$name"; then
    exit 1;
fi

install_for_mac () {
    brew update &&
    brew install postgresql;

    script_success_message "$name";
    exit 0;
}

install_for_linux () {
    sudo apt-get update &&
    sudo apt-get install postgresql postgresql-contrib -y;

    script_success_message "$name";
    exit 0;
}

if is_mac_os; then
    install_for_mac;
fi

install_for_linux;
