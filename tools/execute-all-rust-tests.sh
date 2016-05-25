#!/bin/bash

# This runs unit tests as well as tests living under $crate/tests

set -ex
CURRENT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_PATH="$CURRENT_PATH/.."
COMPONENTS_CRATES="$(find "$PROJECT_PATH/components/" -mindepth 1 -maxdepth 1 -type d)"
CRATES="$PROJECT_PATH $COMPONENTS_CRATES"

for crate in $CRATES; do
    cd $crate
    echo "Running tests for crate at $crate"
    cargo test
    cd -
done
