alias kar="kubernetes_apply_recursive";
alias unsetA="unsetAzureEnvVars";
alias k="unsetA && kubectl";
alias kx="kubectx";
# Usage: kredeploy <deployment-name>
# Deployment name example: feed-manager-web-api
alias kredeploy="k rollout restart deployment";