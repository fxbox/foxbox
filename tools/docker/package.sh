#!/bin/bash

. ./check_args.sh

set -x -e

DEST_DIR=builds/${TARGET}/dist
RUST_TARGET=`cat ${TARGET}/rust_target`
GCC_TRIPLE=`cat ${TARGET}/gcc_triple`

function copy_and_strip() {
  cp ../../target/${RUST_TARGET}/release/$1 ${DEST_DIR}
  docker run --name foxbox_${TARGET}_strip -v `pwd`:/home/user/dev/source foxbox-${TARGET} ${GCC_TRIPLE}-strip ${DEST_DIR}/$1
  docker rm foxbox_${TARGET}_strip
}

mkdir -p ${DEST_DIR}/lib
mkdir -p ${DEST_DIR}/open-zwave/config

# Copy stripped executables.
copy_and_strip dnschallenge
copy_and_strip foxbox

# Package the libpagekite dynamic library.
cp `find ../../target/${RUST_TARGET}/release/ -name libpagekite.so.1` ${DEST_DIR}/lib

# Copy open-zwave configuration files.
# TODO: don't rely on a prebuilt copy in support/
cp -R support/open-zwave/config ${DEST_DIR}

cp support/launch.sh ${DEST_DIR}
cp -R ../../static ${DEST_DIR}

tar -cjf builds/${TARGET}/foxbox-${TARGET}-`date +%Y-%m-%d`.tar.bz2 -C ${DEST_DIR} .

rm -rf ${DEST_DIR}