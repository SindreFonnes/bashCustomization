#!/bin/bash

set -eo pipefail;

sudo apt update;

# Ufw, a simplified interaction with the linux firewall;
sudo apt install ufw -y;

sudo ufw limit 22/tcp;
sudo ufw allow 80/tcp;
sudo ufw allow 443/tcp;
sudo ufw default deny incoming;
sudo ufw default allow outgoing;
sudo ufw enable;

# Show ufw status with:
# sudo ufw status

# Ufw One liner;
# sudo apt install ufw && sudo ufw limit 22/tcp && sudo ufw allow 80/tcp && sudo ufw allow 443/tcp && sudo ufw default deny incoming && sudo ufw default allow outgoing && sudo ufw enable

# fail2ban, prevents connection after x amount of attempts
sudo apt install fail2ban -y;

conf_template_location=$bashC/generalScripts/firewall/jail.local;

cat $conf_template_location | sudo tee -a /etc/fail2ban/jail.local;

sudo systemctl enable fail2ban;
sudo systemctl start fail2ban;

exit 0;