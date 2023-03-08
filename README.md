# My bash settings

run

```sh
chmod +x ./generalScripts/init_repo.sh && ./generalScripts/init_repo.sh
```

to be make it append the loading to bashrc.

## Alias readme

A resource
https://www.shell-tips.com/bash/alias/#gsc.tab=0

## For mac users

https://gcollazo.com/making-iterm-2-work-with-normal-mac-osx-keyboard-shortcuts/

## Unatended upgrade

package apt

comment in:
"o=Debian,a=stable"
"o=Debian,a=stable-updates"

comment in and change:
Unattended-Upgrades::AutoFixInterruptingDpkg "true";
Unattended-Upgrades::MinimalSteps: "true";
Unattended-Upgrades::Remove-Unused-Kernel-Packages: "true";
Unattended-Upgrades::Removes-New-Unused-Dependencies: "true";
Unattended-Upgrades::Automatic-Reboot "true";
Unattended-Upgrades::Automatic-Reboot-WithUsers "true";
Unattended-Upgrades::Automatic-Reboot-Time "true";
Unattended-Upgrades::SyslogEnable "true";

## su

Base debian does not have sudo installed by default.
su # changes user to root user
su - # changes user to root, and also loads root user enviroment

cat /etc/sudoers # writes out the config file for sudo command

## Logs in linux

/var/log

tail -f syslog # writes out last 10 lines continously
tail -n 100 -f syslog # writes out last 100 lines continously

## Bash script set

https://ss64.com/bash/set.html

standard set:

```sh
set -eo pipefail
```

## Unset certain elements in an array

https://reactgo.com/bash-remove-first-element-of-array/

## To change default shell, run:

chsh -s $(which your_shell_here)
