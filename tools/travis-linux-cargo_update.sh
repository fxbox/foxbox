#/bin/bash

set -ev

CURRENT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$CURRENT_PATH/travis-linux-common.sh"


build() {
    echo "build: updating Cargo.lock to the most recent version"
    cargo update
    cargo build
}

lint() {
    echo "lint: nothing to do. Skipping..."
}

set_up_tests() {
    echo "set_up_tests: nothing to do. Skipping..."
}

run_tests() {
    echo "run_tests: nothing to do. Skipping..."
}
