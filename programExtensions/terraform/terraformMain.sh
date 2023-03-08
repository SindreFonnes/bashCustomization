if [[ $bashC != "" ]]; then
    export TERRAFORM_EXTENTION_FOLDER_LOCATION=$bashC/programExtensions/terraform;
else
    export TERRAFORM_EXTENTION_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

source $TERRAFORM_EXTENTION_FOLDER_LOCATION/terraformAliases.sh;
# Not yet needed
# source $TERRAFORM_EXTENTION_FOLDER_LOCATION/terraformFunctions.sh;
