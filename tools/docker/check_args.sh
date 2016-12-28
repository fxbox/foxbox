#!/bin/bash

die () {
    TARGETS=`find ./* -maxdepth 0 -type d -exec basename {} \; | sed s/builds//`
    echo "Usage: |$0 TARGET| where TARGET can be: $TARGETS"
    exit 1
}

[ "$#" -eq 1 ] || die

export TARGET=$1