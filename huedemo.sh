#!/bin/bash

cmd=$1

case $cmd in
	status)
		curl -s -X GET http://localhost:3000/services/list.json \
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
		id=$2
		curl -s -X PUT http://localhost:3000/services/$id/state -d '{"on": true}'
		echo
		while true ; do 
			curl -s -X PUT http://localhost:3000/services/$id/state -d '{"hue": 0.0, "sat": 1.0, "val": 1}'
			echo
			sleep 2
			curl -s -X PUT http://localhost:3000/services/$id/state -d '{"hue": 120, "sat": 1.0, "val": 1}'
			echo
			sleep 2
			curl -s -X PUT http://localhost:3000/services/$id/state -d '{"hue": 240, "sat": 1.0, "val": 1}' 
			echo
			sleep 2
		done
		;;
	off)
		curl -s -X PUT http://localhost:3000/services/$2/state -d '{"on": false}'
		;;
	*)
		echo "usage: $0 <cmd> [<id>]"
		echo "  cmd: status      - list status of all available lights"
		echo "       disco <id>  - well, ... disco!"
		echo "       off <id>    - turn off the lights!"
esac


