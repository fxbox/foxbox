#!/bin/bash
URL_PATH=`echo $2 | perl -lpe '$_ = join "/", reverse split /\./'`
node ./apiCall.js knilxof.org 5300 /v1/dns/$URL_PATH/_acme-challenge "{\"type\":\"TXT\",\"value\":\"$4\"}"
