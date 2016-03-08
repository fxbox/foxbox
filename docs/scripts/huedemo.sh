#!/bin/bash

# You must disable authenticaion before using this script.
# See https://github.com/fxbox/foxbox#disable-authentication

HOST="localhost:3000"
VAL=0.2

huecmd() {
	id=$1
	cmd=$2
	echo -n "$cmd -> "
	curl -s -X PUT "http://$HOST/services/$id/state" -d "$cmd"
	echo
}

cmd=$1

case $cmd in
	status)
		echo "Talking to FoxBox at http://$HOST/services"
		curl -s -X GET http://localhost:3000/services/list \
			| tr '{' \\012 \
			| cut -d, -f1 \
			| tr -d \" \
			| grep ^id \
			| cut -d: -f2 \
			| while read id ; do
				echo -n "$id: "
				curl -X GET http://localhost:3000/services/$id/state
				echo
			  done
		;;
	disco)
		echo "Talking to FoxBox at http://$HOST/services"
		id=$2
		huecmd $id '{"on": true}'
		while true ; do 
			huecmd $id '{"hue": 0.0, "sat": 1.0, "val": '$VAL'}'
			sleep 2
			huecmd $id '{"hue": 120, "sat": 1.0, "val": '$VAL'}'
			sleep 2
			huecmd $id '{"hue": 240, "sat": 1.0, "val": '$VAL'}'
			sleep 2
		done
		;;
	off)
		echo "Talking to FoxBox at http://$HOST/services"
		id=$2
		huecmd $id '{"on": false}'
		;;
	*)
		echo "usage: $0 <cmd> [<id>]"
		echo "  cmd: status      - list status of all available lights"
		echo "       disco <id>  - well, ... disco!"
		echo "       off <id>    - turn off the lights!"
esac

