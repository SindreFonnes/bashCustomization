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

# Ensures brew is installed on macOS and returns the brew prefix
# Usage: BREW_PREFIX=$(ensure_brew_installed)
ensure_brew_installed () {
    if [[ "$OSTYPE" != *"darwin"* ]]; then
        echo "ERROR: ensure_brew_installed() should only be called on macOS" >&2
        return 1;
    fi
    
    local BREW_PREFIX="$(/usr/bin/env brew --prefix 2>/dev/null || true)"
    
    if [[ -z "${BREW_PREFIX}" ]]; then
        echo "Homebrew not found. Installing Homebrew..." >&2
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        
        # Evaluate shellenv to make brew available in current session
        if [[ -x /opt/homebrew/bin/brew ]]; then
            eval "$(/opt/homebrew/bin/brew shellenv)"
        elif [[ -x /usr/local/bin/brew ]]; then
            eval "$(/usr/local/bin/brew shellenv)"
        fi
        
        BREW_PREFIX="$(/usr/bin/env brew --prefix 2>/dev/null || true)"
        
        if [[ -z "${BREW_PREFIX}" ]]; then
            echo "ERROR: Failed to install or locate Homebrew" >&2
            return 1;
        fi
    fi
    
    echo "${BREW_PREFIX}"
    return 0;
}

is_mac_os () {
    if [[ "$OSTYPE" == *"darwin"* ]]; then
        # Ensure brew is installed but don't need the prefix
        ensure_brew_installed >/dev/null
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

is_linux_os () {
    if [[ "$OSTYPE" == *"darwin"* ]]; then
        return 1;
    fi
    
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
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

# Detects if system is Ubuntu/Debian-based
is_ubuntu_debian () {
    if [[ ! -f /etc/os-release ]]; then
        return 1;
    fi
    
    . /etc/os-release
    if [[ "$ID" == "ubuntu" ]] || [[ "$ID_LIKE" == *"debian"* ]] || [[ "$ID" == "debian" ]]; then
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
