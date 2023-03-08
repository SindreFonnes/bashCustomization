#!/bin/bash

script_check_args_exist () {
    if [[ $# < 1 || $1 == "" ]]; then
        return 1;
    fi

    return 0;
}

script_allready_installed () {
    if ! script_check_args_exist ${@}; then
        return 1;
    fi

    echo "${@} is allready installed";
    echo "exiting...";

    return 0;
}

script_does_not_support_os () {
    if ! script_check_args_exist ${@}; then
        return 1;
    fi

    echo "This script does not currently support installing ${@} for your os...";
    echo "If you want to install ${@}, either do it manualy or update this script";
    echo "exiting...";

    return 0;
}

script_success_message () {
    if ! script_check_args_exist ${@}; then
        return 1;
    fi

    echo "Successfully installed ${@}!";

    return 0;
}

script_check_if_allready_installed () {
    if [[ $# < 2 ]]; then
        return 1;
    fi

    name=("${@:2}")

    if ! script_check_args_exist ${name[@]}; then
        return 1;
    fi

    if command -v $1 &> /dev/null; then
        script_allready_installed ${name[@]};
        return 1;
    fi

    return 0;
}

is_mac_os () {
    if [[ "$OSTYPE" == *"darwin"* ]]; then
    	if ! command -v brew &> /dev/null; then
		    run_my_install "brew";
	    fi
        
        return 0;
    fi
    
    return 1;
}

is_wsl_os () {
    if [[ "$OSTYPE" == *"darwin"* ]]; then
        return 1;
    fi
    
    if [[ $(cat /proc/version | tr '[:upper:]' '[:lower:]') == *"wsl"* ]]; then
        return 0;
    fi

    return 1;
}

apt_package_manager_available () {
    if command -v apt &> /dev/null; then
        return 0;
    fi

    return 1;
}

run_my_install () {
    if ! script_check_args_exist ${@}; then
        return 1;
    fi

    $MYINSTALL_SCRIPT_LOCATION $1 $2;
}
