# --== Some colors ==-- #

GREEN='\033[0;32m'
NC='\033[0m'

# --== Releasing repositories to stage/prod environments ==-- #

get-latest-stage-tag() {
    git fetch
    git tag --sort=-version:refname |\
    head -n 50 |\
    grep "\-stage" |\
    head -n 1
}

get-latest-stage-tag-or-default() {
    get-latest-stage-tag | grep --color=never . || echo "v1.0.0-stage"
}

get-next-stage-tag() {
    get-latest-stage-tag-or-default |\
    awk -F '.' '{printf "%s.%d.%d-stage", $1, $2, $3+1}'
}

get-next-prod-tag() {
    get-latest-stage-tag-or-default |\
    awk -F '.' '{printf "%s.%d.%d-prod", $1, $2, $3}'
}

#release-stage() {
#    gh release create $(get-next-stage-tag) -n ""
#    git fetch
#}

release-stage() {
    increment_type="patch" # Default to patch releases

    # Process flags
    while [[ "$#" -gt 0 ]]; do 
        case $1 in
            --patch) increment_type="patch";;
            --minor) increment_type="minor";;
            --major) increment_type="major";;
            *) echo "Unknown parameter: $1" >&2; exit 1;;
        esac
        shift
    done

    # Function to increment a version part 
    increment_version_part() {
        current_ver=$(get-latest-stage-tag-or-default)
        awk -F '.' -v part=$1 -v type=$increment_type '
            {
                if (type == "patch") $3++
                else if (type == "minor") { $2++; $3=0 } 
                else if (type == "major") { $1++; $2=0; $3=0 }
                sub(/^v/, "", $1) 
                printf "v%s.%d.%d-stage\n", $1, $2, $3 # Prefix with "v"
            }
        ' <<< $current_ver
    }

    next_stage_tag=$(increment_version_part)
    echo $next_stage_tag
    gh release create $next_stage_tag -n ""
    git fetch
}

release-prod() {
  LATEST_STAGE_TAG=$(get-latest-stage-tag-or-default)
  LATEST_PROD_TAG=$(echo $LATEST_STAGE_TAG | awk -F '.' '{printf "%s.%d.%d-prod", $1, $2, $3}')
  STAGE_TAG_SHA=$(git rev-list -n 1 tags/$LATEST_STAGE_TAG)  
  gh release create $LATEST_PROD_TAG --target $STAGE_TAG_SHA -n ""
}

release-all() {
    release-stage
    release-prod
}

# --== Approving/rejecting issues for manual approval ==-- #

get-latest-approve-issue() {
    gh issue list --json=number,title --jq='.[] | select(.title | startswith("Request for approval")) | .number' |\
    head -1
}

release-approve() {
  ISSUE=$(get-latest-approve-issue)
  if [[ $ISSUE ]]; then
    gh issue comment $ISSUE -b "approved"
  else
    echo "No issue found to approve"
  fi
}

release-reject() {
  ISSUE=$(get-latest-approve-issue)
  if [[ $ISSUE ]]; then
    gh issue comment $ISSUE -b "rejected"
  else
    echo "No issue found to reject"
  fi
}

# --== Getting the latest terraform plan for prod ==-- #

get-latest-workflow-run()
{
    gh run list --limit 5 --json=databaseId,headBranch --jq='.[] | select(.headBranch | endswith("'${1}'")) | .databaseId' | head -1
}

get-latest-workflow-run-url() {
    gh run view $(get-latest-workflow-run ${1}) --json=url --jq='.url'
}

get-latest-workflow-run-job() {
    gh run view $(get-latest-workflow-run ${1}) --json=jobs --jq='.jobs.[0].databaseId'
}

get-latest-workflow-run-job-url() {
    gh run view $(get-latest-workflow-run ${1}) --json=jobs --jq='.jobs.[0].url'
}

get-latest-workflow-run-job-tag() {
    gh run view $(get-latest-workflow-run ${1}) --json=headBranch --jq='.headBranch'
}

get-latest-workflow-run-job-log() {
    gh run view --log --job=$(get-latest-workflow-run-job ${1})
}

get-latest-workflow-run-job-log() {
    gh run view --log --job=$(get-latest-workflow-run-job ${1})
}

get-latest-terraform-plan() {  
    echo -e "\nPrinting terraform plan for tag ${GREEN}$(get-latest-workflow-run-job-tag ${1})${NC}\n"
    LATEST_TERRAFORM_LOG=$(get-latest-workflow-run-job-log ${1})
    
    LATEST_TERRAFORM_PLAN=$(\
        echo "$LATEST_TERRAFORM_LOG" |\
        awk '/Terraform will perform the following actions/,/Plan:/{print; if (/Plan:/) nextfile}' | # Find the first occurence of the printed terraform plan \
        awk -F'\t' '{print $3}' | # Remove extra github actions metadata in the beginning \
        sed 's/^[^ ]* //'         # Remove timestamp` \
    )

    if [[ $LATEST_TERRAFORM_PLAN ]]; then
        echo "$LATEST_TERRAFORM_PLAN"
    else
        echo "$LATEST_TERRAFORM_LOG" |\
        awk '/No changes/{print; if (/No changes/) nextfile}' | # Find the first occurence of No changes \
        awk -F'\t' '{print $3}' | # Remove extra github actions metadata in the beginning \
        sed 's/^[^ ]* //'         # Remove timestamp` \
    fi
}

release-plan() {
    get-latest-terraform-plan ${1}
}

release-view() {
    gh run view $(get-latest-workflow-run ${1})
}

if [[ -n "$WSL_DISTRO_NAME" ]]; then
    alias open-in-browser='explorer.exe' # This trick can be used on windows to open urls in the browser
else
    alias open-in-browser='open' # This trick can be used on windows to open urls in the browser
fi

release-open() {
    open-in-browser $(get-latest-workflow-run-job-url ${1})
}

release-watch() {
      gh run watch $(get-latest-workflow-run ${1})
}

release-rerun() {
    RUN_ID=$(get-latest-workflow-run ${1})
    echo -e "\nRe-running workflow for tag ${GREEN}$(get-latest-workflow-run-job-tag ${1})${NC} (Run $RUN_ID)\n"
    gh run rerun $RUN_ID
}

# Aliases for different environments

alias release-plan-dev='release-plan "main"'
alias release-plan-stage='release-plan "-stage"'
alias release-plan-prod='release-plan "-prod"'

alias release-view-dev='release-view "main"'
alias release-view-stage='release-view "-stage"'
alias release-view-prod='release-view "-prod"'

alias release-open-dev='release-open "main"'
alias release-open-stage='release-open "-stage"'
alias release-open-prod='release-open "-prod"'

alias release-watch-dev='release-watch "main"'
alias release-watch-stage='release-watch "-stage"'
alias release-watch-prod='release-watch "-prod"'

alias release-rerun-dev='release-rerun "main"'
alias release-rerun-stage='release-rerun "-stage"'
alias release-rerun-prod='release-rerun "-prod"'


# TODO - Seems logs are unavailable during run unfortunately, it should hopefully be fixed some time soon
# some of the issues maybe fixed soon, see https://github.com/actions/runner/issues/886#issuecomment-1669631425, there is some big rewrite of logging stuff