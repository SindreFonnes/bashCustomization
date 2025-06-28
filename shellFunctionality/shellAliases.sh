# Behaviour
if [[ $IS_MAC != "true" ]]; then
    alias rm="/usr/bin/safe-rm";
fi
alias c="clear";

alias grepOnly="grep_specific_filetype_in_subfolders";
alias showSize="find_entity_size";
alias updateNotes="git_pull_repo $notes_home";
alias installStuff="$bashC/generalScripts/installStuff.sh";

### Restart/reload shell;
alias restart="restart_shell";
alias reload="load_shell_extentionfiles";

### Shell upgrades
alias upgrade="update_packages";
alias updateShell="git_pull_repo $bashC";

### Ip stuff
alias getLocalIp="ip r";
alias whatismyip='curl ipecho.net/plain';

# Edit
alias c.="code .";
alias editBash="$standard_editor $bashC/main.sh";
alias editBashrc="$standard_editor ~/.bashrc";
alias editAlias="$standard_editor $bashC/aliases.sh";

## Open vscode for location
alias codeBash="code $bashC";
alias codeNotes="code $notes_home";

# Navigation
## Locations
alias gotoSettings="cdd $bashC";
alias bashC="cdd $bashC";

alias p="cdd $p_home";
alias scripts="cdd $scripts_home";
alias notes="cdd $notes_home";

### Project folder navigation
alias rust="cdd $p_home/rust";
alias javascript="cdd $p_home/javascript";
alias pgo="cdd $p_home/go";
alias pdotnet="cdd $p_home/dotnet";

## Behaviour
alias cdd="pushd_wrapper";
alias goback="popd_wrapper";
alias gob="goback";
alias cdh="dirs -l -v";

alias externalIp="curl ipecho.net/plain";

# Windows only aliases
if [[ $IS_WSL == true ]]; then
    alias p-win="cdd $p_win_home";
    alias windows="cdd $win_main_drive_path";
fi

alias cb="output_to_clipboad";
