#!/bin/bash

check_install_arg () {
    local input=($@);

    if [[ $2 == " " ]]; then
        run_install_script "$1";
    else
        run_install_script_with_args "$1" "$2";
    fi
}

run_install_script () {
    chmod +x "$1" &&
    $1;
}

run_install_script_with_args () {
    chmod +x "$1" &&
    $1 "$2";
}

determine_install_script_to_use () {
		local AZURE_INSTALL_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/azure/installAzureCli.sh;
		local BREW_INSTALL_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/brew/installBrew.sh;
		local DOCKER_INSTALL_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/docker/installDocker.sh;
        local DOTNET_INSTALL_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/dotnet/installDotnet.sh;
		local GITHUB_INSTALL_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/github/installGithubCli.sh;
        local GO_INSTALL_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/go/installGo.sh;
        local JAVA_INSTALL_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/java/installJava.sh;
        local JAVASCRIPT_INSTALL_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/javascript/jsMain.sh;
        local RUST_INSTALL_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/rust/installRust.sh;
		local TERRAFORM_INSTALL_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/terraform/installTerraform.sh;
        local NEOVIM_INSTALL_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/neovim/installNeovim.sh;
        local POSTGRES_INSTALL_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/postgres/installPostgres.sh;
		local KUBECTL_INSTALL_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/kubectl/installKubectl.sh;
        local OBSIDIAN_INSTALL_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/obsidian/installObsidian.sh;

        local input=($@);

        # Argument 1 and 2 is reserved for passing the selection arguments

        case "${input[0]}" in
            "${input[2]}" | "1")
                run_install_script "$GO_INSTALL_LOCATION"
                ;;
            "${input[3]}" | "2")
                run_install_script "$DOTNET_INSTALL_LOCATION"
                ;;
            "${input[4]}" | "3")
                run_install_script "$RUST_INSTALL_LOCATION"
                ;;
            "${input[5]}" | "4" | "js" | "javascript")
                check_install_arg "$JAVASCRIPT_INSTALL_LOCATION" "${input[1]}"
                ;;
            "${input[6]}" | "5")
                run_install_script "$JAVA_INSTALL_LOCATION"
                ;;
            "${input[7]}" | "6")
				run_install_script "$AZURE_INSTALL_LOCATION"
				;;

			"${input[8]}" | "7")
				run_install_script "$GITHUB_INSTALL_LOCATION"
				;;

			"${input[9]}" | "8")
				run_install_script "$TERRAFORM_INSTALL_LOCATION"
				;;
			
			"${input[10]}" | "9")
				run_install_script "$BREW_INSTALL_LOCATION"
				;;

			"${input[11]}" | "10")
				run_install_script "$DOCKER_INSTALL_LOCATION"
				;;

            "${input[12]}" | "nvim" | "11")
                check_install_arg "$NEOVIM_INSTALL_LOCATION" "${input[1]}"
                ;;

			"${input[13]}" | "12")
                run_install_script "$POSTGRES_INSTALL_LOCATION"
                ;;

            "${input[14]}" | "13" | "k8s")
                run_install_script "$KUBECTL_INSTALL_LOCATION"
                ;;

            "${input[15]}" | "14")
                run_install_script "$OBSIDIAN_INSTALL_LOCATION"
                ;;

            "${input[16]}" | "15")
                echo "Running all scripts sequentialy..." &&
                run_install_script "$GO_INSTALL_LOCATION" &&

                run_install_script "$DOTNET_INSTALL_LOCATION" &&

                run_install_script "$RUST_INSTALL_LOCATION" &&
                run_install_script "$NEOVIM_INSTALL_LOCATION" &&
                
                # Node has 3 things to install
                run_install_script "$JAVASCRIPT_INSTALL_LOCATION" "nvm" &&
                run_install_script "$JAVASCRIPT_INSTALL_LOCATION" "pnpm" &&
                run_install_script "$JAVASCRIPT_INSTALL_LOCATION" "yarn" &&
                run_install_script "$JAVASCRIPT_INSTALL_LOCATION" "bun" &&

                run_install_script "$JAVA_INSTALL_LOCATION" &&
				run_install_script "$AZURE_INSTALL_LOCATION" &&
				run_install_script "$GITHUB_INSTALL_LOCATION" &&
				run_install_script "$TERRAFORM_INSTALL_LOCATION" &&
				run_install_script "$BREW_INSTALL_LOCATION" &&
				run_install_script "$DOCKER_INSTALL_LOCATION" &&                
                run_install_script "$KUBECTL_INSTALL_LOCATION" &&
                run_install_script "$OBSIDIAN_INSTALL_LOCATION" &&
                echo "Installed everything (except postgres)";
                ;;
            *)
                echo "Not an available install option ${input[0]}";
                ;;
        esac;

        return 0;
}

use_install_script () {
    local install_options=("go" "dotnet" "rust" "node" "java" "azure" "github" "terraform" "brew" "docker" "neovim" "postgres" "kubernetes" "obsidian" "all")

    if [[ $# -ne 0 ]]; then
        if [[ $# > 1 ]]; then
            determine_install_script_to_use "$1" "$2" "${install_options[@]}";
        else
            determine_install_script_to_use "$1" "unfilled_value" "${install_options[@]}";
        fi
        
        return 0;
    fi

    echo "What do you want to install?";
    
    select choice in "${install_options[@]}"; do
        determine_install_script_to_use "$REPLY" "unfilled_value" "${install_options[@]}" &&
        if [[ $? -eq 0 ]]; then
            break;
        fi
    done;
}

use_install_script $1 $2;
