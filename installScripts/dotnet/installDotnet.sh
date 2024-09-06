#!/bin/bash

set -eo pipefail;

name="Dotnet";
commandAlias="dotnet";

source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

if ! script_check_if_allready_installed "$commandAlias" "$name"; then
	exit 1;
fi

if is_mac_os; then
    script_does_not_support_os "$name";
    exit 1;
fi

INSTALL_DISTRO="Ubuntu";
INSTALL_DISTRO_VERSION="22.04";

check_param_for_string () {
	if [[ "$1" == *"${@:2}"* ]]; then
        return 0;
    fi
	return 1;
}

OS_RELEASE=$(cat /etc/os-release)

install_for_ubuntu () {
    #HTML_FRIENDLY=$(echo $INSTALL_DISTRO | tr '[:upper:]' '[:lower:]');

    #wget https://packages.microsoft.com/config/$HTML_FRIENDLY/$INSTALL_DISTRO_VERSION/packages-microsoft-prod.deb -O packages-microsoft-prod.deb;
    #sudo dpkg -i packages-microsoft-prod.deb;
    #rm packages-microsoft-prod.deb;

    # Install ASP.NET Core runtime
    sudo apt-get update; \
    sudo apt-get install apt-transport-https -y && \
    sudo apt-get update && \
  	sudo apt-get install -y dotnet-sdk-8.0 && \

    script_success_message "$name";
    exit 0;
}

if check_param_for_string "$OS_RELEASE" "$INSTALL_DISTRO $INSTALL_DISTRO_VERSION"; then
    install_for_ubuntu;
fi

echo "Installscript eiher cannot be run on your system";
    
if check_param_for_string "$OS_RELEASE" "NAME=\"$INSTALL_DISTRO\""; then
    echo "You are running an updated/outdated version of the os compared to what the install script is targeting";
else
    echo "You are running a different distro from what the installscript is targeting."
fi
    
echo "The installscript is targeting $INSTALL_DISTRO version $INSTALL_DISTRO_VERSION.";
echo "Either update the system or the install script if you want to install .NET this way";

exit 1;
