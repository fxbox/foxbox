#!/bin/bash

set -ev
CURRENT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_PATH="$CURRENT_PATH/.."
source "$CURRENT_PATH/travis-linux-common.source.sh"

build() {
    echo "build: nothing to do. Skipping..."
}

run_tests() {
    "$CURRENT_PATH/execute-unit-tests-with-coverage.sh"
}
