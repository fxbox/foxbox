#!/bin/bash

# This is a quick&dirty demo of how to talk to the Philips HUe adapter.
# You must disable authenticaion before using this script.

BRIGHTNESS=0.5
URL="http://localhost:3000"
API="$URL/api/v1"
SERVICES="$API/services"
GET="$API/channels/get"
SET="$API/channels/set"

function channels() {
	curl -s -X GET "$SERVICES" \
		| tr '"' \\012 \
		| grep "etter:" \
		| grep philips_hue \
		| sort -u
}

function getters() {
	channels | grep ^getter:
}

function setters() {
	channels | grep ^setter:
}

function get() {
	curl -s -X PUT "$GET" -d "{\"id\":\"$1\"}"
}

function set() {
	curl -s -X PUT "$SET" -d "{\"select\":{\"id\":\"$1\"},\"value\":{$2}}"
}

function all_on() {
	for s in `setters | grep power`
	do
		echo -n $s:
		set $s '"OnOff":"On"'
		echo
	done
}

function all_off() {
	for s in `setters | grep power`
	do
		echo -n $s:
		set $s '"OnOff":"Off"'
		echo
	done
}

function all_color() {
	for s in `setters | grep color`
	do
		echo -n $s:
		set $s '"Color":{"h":'${1-0}',"s":'${2-1}',"v":'${3-1}'}'
		echo
	done
}

function clean_up() {
	all_off &>/dev/null
	all_color 0 0 $BRIGHTNESS &>/dev/null
	exit
}

case "$1" in
	"list")
		channels
		;;
	"on")
		all_on &>/dev/null
		;;
	"off")
		all_off &>/dev/null
		;;
	"color")
		all_on &>/dev/null
		all_color $2 $3 $4 &>/dev/null
		;;
	"disco")
		trap clean_up SIGHUP SIGINT SIGTERM
		all_on &>/dev/null
		while true
		do
			all_color 0 1 $BRIGHTNESS &>/dev/null
			sleep 1
			all_color 60 1 $BRIGHTNESS &>/dev/null
			sleep 1
			all_color 120 1 $BRIGHTNESS &>/dev/null
			sleep 1
			all_color 180 1 $BRIGHTNESS &>/dev/null
			sleep 1
			all_color 240 1 $BRIGHTNESS &>/dev/null
			sleep 1
			all_color 300 1 $BRIGHTNESS &>/dev/null
			sleep 1
		done
		;;
	*)
		echo "usage: $0 list|on|off|color|disco"
		;;
esac

