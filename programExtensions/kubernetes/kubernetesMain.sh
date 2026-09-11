if [[ $bashC != "" ]]; then
    export KUBERNETES_EXTENTION_FOLDER_LOCATION=$bashC/programExtensions/kubernetes;
else
    export KUBERNETES_EXTENTION_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

_bashc_source_file "$KUBERNETES_EXTENTION_FOLDER_LOCATION/kubernetesFunctions.sh" || return 1
_bashc_source_file "$KUBERNETES_EXTENTION_FOLDER_LOCATION/kubernetesAliases.sh" || return 1
