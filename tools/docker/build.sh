#!/bin/bash

. ./check_args.sh

set -x

docker run --name foxbox_${TARGET}_builder -v `pwd`/../..:/home/user/dev/source -v $HOME/.cargo:/home/user/.cargo foxbox-${TARGET} cargo${TARGET} build --release
docker rm foxbox_${TARGET}_builder
