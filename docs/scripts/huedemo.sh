#!/bin/bash

# This is a quick&dirty demo of how to talk to the Philips HUe adapter.
# You must disable authenticaion before using this script.

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


for g in `getters | grep available`
do
	echo -n $g:
	get $g
	echo
done

function all_on() {
	for g in `setters | grep power`
	do
		echo -n $g:
		set $g '"OnOff":"On"'
		echo
	done
}

function all_off() {
	for g in `setters | grep power`
	do
		echo -n $g:
		set $g '"OnOff":"Off"'
		echo
	done
}

for x in `seq 3`
do
	all_on
	sleep 1
	all_off
	sleep 1
done


