#!/bin/bash

set -eo pipefail;

sek=60
echo "$sek Seconds Wait!"
while [ $sek -ge 1 ]
do
   echo -ne "One Moment please $sek ... \r"
   sleep 1
   sek=$[$sek-1]
done
echo
echo "ready!"
