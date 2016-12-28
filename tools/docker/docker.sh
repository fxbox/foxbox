#!/bin/bash

. ./check_args.sh

set -x -e

docker build -t foxbox-${TARGET} ${TARGET}
