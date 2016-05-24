#/bin/bash

set -ev

CURRENT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$CURRENT_PATH/travis-linux-common.sh"


build() {
    echo "Nothing to do"
}

lint() {
    echo "Nothing to do"
}

set_up_tests() {
    echo "Nothing to do"
}

run_tests() {
    # TODO: Currently unit tests are executed twice. We need a way to filter out
    # tests with `cargo test` depending on where they are defined in the tree
    "$CURRENT_PATH/execute-all-rust-tests.sh"
}
