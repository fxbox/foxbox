#!/bin/bash

die () {
    # TODO: dynamically get the list of available targets.
    echo "Usage: |$0 TARGET| where TARGET can be: rpi2"
    exit 1
}

[ "$#" -eq 1 ] || die

export TARGET=$1