# --== Some colors ==-- #

GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
DIM_GRAY='\033[0;90m'
NC='\033[0m'

# --== Utility functions ==-- #

get-latest-stage-tag() {
    get-latest-tag "\-stage"
}

get-latest-prod-tag() {
    get-latest-tag "\-prod"
}

get-latest-tag() {
    git fetch --quiet
    git tag --sort=-version:refname |\
    head -n 50 |\
    grep $1 |\
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

get-git-commit-sha-by-tag() {
    git rev-list -n 1 tags/$1
}

# --== Releasing repositories to stage/prod environments ==-- #

release-stage() {
    diff-tag-full stage
    release-continue-prompt || return
    release-stage-force
}

release-stage-force() {
    gh release create $(get-next-stage-tag) -n ""
    git fetch --quiet
}

release-prod() {
    diff-tag-full prod stage
    release-continue-prompt || return
    release-prod-force
}

release-prod-force() {
    LATEST_STAGE_TAG=$(get-latest-stage-tag-or-default)
    LATEST_PROD_TAG=$(echo $LATEST_STAGE_TAG | awk -F '.' '{printf "%s.%d.%d-prod", $1, $2, $3}')
    STAGE_TAG_SHA=$(get-git-commit-sha-by-tag $LATEST_STAGE_TAG)
    gh release create $LATEST_PROD_TAG --target $STAGE_TAG_SHA -n ""
    git fetch --quiet
}

release-all() {
    diff-tag-full prod
    release-continue-prompt || return
    release-all-force
}

release-all-force() {    
    release-stage-force
    release-prod-force
}

release-continue-prompt() {
    echo -e "Do you wish to continue? [yes]"
    read -r answer
    if [[ $answer != "yes" ]]; then
        echo "Aborting release"
        return 1
    fi
}

# --== Diff to show unreleased items ==-- #

diff-tag() {

    local from=$1
    local to=${2:-origin/main}

    if [[ $from == "prod" || $from == "" ]]; then
        from=$(get-latest-prod-tag)
    elif [[ $from == "stage" ]]; then
        from=$(get-latest-stage-tag)
    fi

    if [[ $to == "stage" ]]; then
        to=$(get-latest-stage-tag)
    fi

    echo "$from..$to"
    git --no-pager log $from..$to --pretty=tformat:"%s"
}

diff-tag-full() {
    #!/usr/bin/env bash
    local JIRA_CASES_PATTERN="^([A-Z]+-[0-9]+)\."
    local jira_cases_in_commits=()
    local all_commits=()

    # Read all commits and parse jira cases
    local GIT_COMMITS_DIFF_WITH_HEADER=$(diff-tag $1 $2)
    local GIT_COMMITS_DIFF_HEADER=$(echo "$GIT_COMMITS_DIFF_WITH_HEADER" | head -n 1)
    local GIT_COMMITS_DIFF=$(echo "$GIT_COMMITS_DIFF_WITH_HEADER" | tail -n +2)

    while IFS= read -r line; do
        if [[ $line == "" ]]; then
            continue
        fi
        
        if [[ $line =~ $JIRA_CASES_PATTERN ]]; then

            key="${BASH_REMATCH[1]}"
            if [[ $key == "" ]]; then # in zsh, the match is in $match[1]
                key=$match[1]
            fi
            
            local item_exists=false
            for jira_case in "${jira_cases_in_commits[@]}"; do
                if [[ $jira_case == $key ]]; then
                    item_exists=true
                    break;
                fi
            done

            if [[ $item_exists == "false" ]]; then
                jira_cases_in_commits+=("$key")
            fi
        fi

        all_commits+=("$line")

    done <<< "${GIT_COMMITS_DIFF}"

    # Output all commits and highlight the ones with JIRA cases
    if [[ ${#all_commits[@]} == 0 ]]; then
        echo -e "\n${GREEN}No commits found (${GIT_COMMITS_DIFF_HEADER}).${NC}\n"
        return
    fi

    if hash jira 2>/dev/null; then
        JIRA_INSTALLED=true
    else
        JIRA_INSTALLED=false
    fi

    # Query JIRA for the cases
    if [[ "$JIRA_INSTALLED" == "true" && ${#jira_cases_in_commits[@]} > 0 ]]; then
        JIRA_CASES_STRING=""
        for key in "${jira_cases_in_commits[@]}"; do
            if [[ $JIRA_CASES_STRING ]]; then
                JIRA_CASES_STRING+=", "
            fi
            JIRA_CASES_STRING+="$key"
        done

        local found_jira_cases=$(jira issue list -q "project IS NOT EMPTY AND issuekey IN ($JIRA_CASES_STRING)" --plain)        
        
        # Get the keys from the JIRA cases
        local found_jira_keys=()
        for key in $(printf '%s\n' "$found_jira_cases" | tail -n +2 | awk '{print $2}'); do
            found_jira_keys+=("$key")
        done

    else
        local found_jira_cases=""
    fi        

    # Output all commits and highlight the ones with JIRA cases
    echo -e "\n${GREEN}Commits (${GIT_COMMITS_DIFF_HEADER}):${NC}"
    for line in "${all_commits[@]}"; do
        matched=false
        for key in "${found_jira_keys[@]}"; do
            if [[ "$line" == *"$key"* ]]; then
                matched=true
            break
            fi
        done

        if [[ $matched == "true" ]]; then
            echo -e "${DIM_GRAY}$line${NC}"
        else
            echo -e "${YELLOW}$line${NC}"
        fi
    done

    # Output the JIRA cases if they exist
    if [[ $found_jira_cases ]]; then
        echo -e "\n${GREEN}Found JIRA cases:${NC}"
        while IFS= read -r line; do
            if [[ "$line" =~ ^TYPE[[:space:]]+KEY ]]; then
                echo "$line"
                continue
            fi

            READY_FOR_PRODUCTION_PATTERN="(Ready[[:space:]]for[[:space:]]prod|Ready[[:space:]]for[[:space:]]production)$"

            # If the status is NOT one of the two “ready for prod” variants...
            if [[ ! "$line" =~ $READY_FOR_PRODUCTION_PATTERN ]]; then
                # …print it in yellow
                echo -e "${YELLOW}${line}${NC}"
            else
                echo -e "${DIM_GRAY}${line}${NC}"
            fi
        done <<< $found_jira_cases
    else
        if [[ "$JIRA_INSTALLED" == "true" ]]; then
            echo -e "\n${GREEN}No JIRA cases found.${NC}"
        else
            echo -e "\n${YELLOW}Please install jira-cli to get more information about the JIRA cases. Follow installation and initialization instructions here: https://github.com/ankitpokhrel/jira-cli${NC}"
        fi            
    fi    

    echo
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

alias release-diff='diff-tag-full'

# TODO - Seems logs are unavailable during run unfortunately, it should hopefully be fixed some time soon
# some of the issues maybe fixed soon, see https://github.com/actions/runner/issues/886#issuecomment-1669631425, there is some big rewrite of logging stuff