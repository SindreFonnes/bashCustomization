if [[ $bashC != "" ]]; then
    export KUBERNETES_EXTENTION_FOLDER_LOCATION=$bashC/programExtensions/kubernetes;
else
    export KUBERNETES_EXTENTION_FOLDER_LOCATION=$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd );
fi

source $KUBERNETES_EXTENTION_FOLDER_LOCATION/kubernetesFunctions.sh;
source $KUBERNETES_EXTENTION_FOLDER_LOCATION/kubernetesAliases.sh;
