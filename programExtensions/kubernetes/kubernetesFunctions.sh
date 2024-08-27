kubernetes_apply_recursive () {
    kubectl apply -f "$1" --recursive
}

unsetAzureEnvVars() {
    unset AZURE_SUBSCRIPTION_ID
    unset AZURE_CLIENT_ID # kubectl log in would fail if this is set as it starts using this variable
    unset AZURE_CLIENT_SECRET
    unset AZURE_TENANT_ID
}