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

install_for_apt () {
    sudo apt-get update &&
    sudo apt-get install postgresql postgresql-contrib -y;

    script_success_message "$name";
    exit 0;
}

install_for_pacman () {
    sudo pacman -Syu --noconfirm;
    sudo pacman -S --needed --noconfirm postgresql;
    
    # Initialize the database cluster
    sudo -u postgres initdb -D /var/lib/postgres/data;
    
    # Enable and start postgresql service
    sudo systemctl enable postgresql.service;
    sudo systemctl start postgresql.service;
    
    script_success_message "$name";
    echo "Note: PostgreSQL has been initialized and started.";
    exit 0;
}

if is_mac_os; then
    install_for_mac;
fi

if apt_package_manager_available; then
    install_for_apt;
fi

if pacman_package_manager_available; then
    install_for_pacman;
fi

script_does_not_support_os "$name";

exit 1;
