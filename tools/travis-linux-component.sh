#!/bin/bash

set -ev
CURRENT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_PATH="$CURRENT_PATH/.."

# GLOBAL ENV $COMPONENT

install_dependencies() {
    echo "install_dependencies: nothing to do. Skipping..."
}

build() {
    echo "build: nothing to do. Skipping..."
}

run_tests() {
    cd "$PROJECT_PATH/components/$COMPONENT"
    cargo test
    cd -
}
