#!/bin/bash

set -ev

CURRENT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"


install_dependencies() {
    brew update
    brew install openssl libupnp sqlite
    source "$CURRENT_PATH/mac-os-x-setup.source.sh"
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
