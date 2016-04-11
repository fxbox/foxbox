#!/bin/bash

set -e

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
    RUST_BACKTRACE=1 cargo test
}
