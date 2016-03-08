#!/bin/bash
// Usage: ./update.sh my-link-box 192.168.0.42 52.36.71.23

echo "$1.useraddress.net a.$1.useraddress.net b.$1.useraddress.net remote.$1.useraddress.net" > ./domains.txt
./letsencrypt.sh --cron --challenge dns-01 --hook ./deploy-challenge.sh
curl -i -X POST -d "{\"type\":\"A\",\"value\":\"$2\"}" http://ns.useraddress.net:5300/v1/dns/net/useraddress/$1
curl -i -X POST -d "{\"type\":\"A\",\"value\":\"$2\"}" http://ns.useraddress.net:5300/v1/dns/net/useraddress/$1/a
curl -i -X POST -d "{\"type\":\"A\",\"value\":\"$2\"}" http://ns.useraddress.net:5300/v1/dns/net/useraddress/$1/b
curl -i -X POST -d "{\"type\":\"A\",\"value\":\"$3\"}" http://ns.useraddress.net:5300/v1/dns/net/useraddress/$1/remote
