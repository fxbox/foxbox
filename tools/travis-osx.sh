#!/bin/bash

set -e

CURRENT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

export DEP_OPENSSL_INCLUDE=/usr/local/opt/openssl/include/
export OPENSSL_LIB_DIR=/usr/local/opt/openssl/lib
# TODO: Remove once we figure out a better fix https://github.com/fxbox/foxbox/issues/414
export SQLITE3_LIB_DIR=/usr/local/opt/sqlite/lib

install_dependencies() {
    brew update
    brew install libupnp openssl sqlite
}

build() {
    cargo build
}

lint() {
    echo "lint: jshint is not installed on Mac. Skipping..."
}

set_up_tests() {
    echo "set_up_tests: no set up required. Skipping..."
}

run_tests() {
    echo "run_tests: no selenium installed. Skipping..."
    echo "run_tests: no npm installed. Skipping integration tests..."
    echo "run_tests: kcov is not supported on Mac. Running only the tests..."
    "$CURRENT_PATH/execute-all-rust-tests.sh"
}
