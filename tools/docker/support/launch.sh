#!/bin/bash

export LD_LIBRARY_PATH=`pwd`/lib:$LD_LIBRARY_PATH
export PATH=`pwd`:$PATH

set -x -e

./foxbox -s foxbox -t knilxof.org:443 $@