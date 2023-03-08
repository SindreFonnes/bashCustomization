kubernetes_apply_recursive () {
    kubectl apply -f "$1" --recursive
}
