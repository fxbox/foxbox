#!/bin/bash
# Usage: ./update.sh a7427c5ce2175c61a7427c5ce2175c61 192.168.0.42 52.36.71.23
echo "Got called: ./update.sh $1 $2 $3"

echo "Creating DNS records..."

node ./apiCall.js knilxof.org 5300 /v1/dns/net/useraddress/xob/$1 "{\"type\":\"A\",\"value\":\"$2\"}"
node ./apiCall.js knilxof.org 5300 /v1/dns/net/useraddress/xob/$1/a "{\"type\":\"A\",\"value\":\"$2\"}"
node ./apiCall.js knilxof.org 5300 /v1/dns/net/useraddress/xob/$1/b "{\"type\":\"A\",\"value\":\"$2\"}"
node ./apiCall.js knilxof.org 5300 /v1/dns/net/useraddress/xob/$1/remote "{\"type\":\"A\",\"value\":\"$2\"}"

echo "$1.xob.useraddress.net a.$1.xob.useraddress.net b.$1.xob.useraddress.net remote.$1.xob.useraddress.net" > ./domains.txt
echo "Getting SAN cert for: `cat domains.txt`"
./letsencrypt.sh --cron --challenge dns-01 --hook ./deploy-challenge.sh

echo "Setting remote. to use the tunnel"
node ./apiCall.js knilxof.org 5300 /v1/dns/net/useraddress/xob/$1/remote "{\"type\":\"A\",\"value\":\"$3\"}"
