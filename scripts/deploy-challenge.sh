#!/bin/bash
echo $1 $2 $3 $4 $5
URL=`node ./getUrl.js $2`
echo curl -i -X POST -d "{\"type\":\"TXT\",\"value\":\"$4\"}" $URL
curl -i -X POST -d "{\"type\":\"TXT\",\"value\":\"$4\"}" $URL
