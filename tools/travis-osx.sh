#!/bin/bash

set -e

export OPENSSL_LIB_DIR=/usr/local/opt/openssl/lib
export EXTRA_LDFLAGS=-L/usr/local/opt/openssl/lib

install_dependencies() {
    brew update
    brew install libupnp openssl sqlite
}

build() {
    rm -rf target
    cargo clean
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
    RUST_BACKTRACE=1 cargo test
}
