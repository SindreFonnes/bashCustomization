#!/bin/bash

set -eo pipefail;

PATH_TO_SCRIPT="/home/sindre/bashCustomization/test2.sh";

scp $PATH_TO_SCRIPT root@10.0.0.28:/root/testScript.sh;

ssh root@10.0.0.28 << EOF
  sh /root/testScript.sh;
EOF

sleep 5;

scp root@10.0.0.28:/root/test ./testResult;

cat ./testResult;

exit 0;



TEMP=$(getopt -o h -l filename:,verbose -- "$@")

echo ${TEMP[@]}

exit 0;

while getopt ":h-:" opt; do
  case $opt in
    h)
      echo "Usage: my_script.sh [--filename filename] [--verbose]"
      exit 0
      ;;
    filename)
      filename=$OPTARG
      ;;
    verbose)
      verbose=1
      ;;
    \?)
      echo "Invalid option: -$OPTARG" >&2
      exit 1
      ;;
    :)
      echo "Option -$OPTARG requires an argument." >&2
      exit 1
      ;;
  esac
done

echo "Filename: $filename"
echo "Verbose mode: $verbose" 

exit 0;

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