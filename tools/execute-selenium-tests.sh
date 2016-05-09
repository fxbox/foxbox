#!/bin/bash

set -ex

CURRENT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_HOME="$CURRENT_PATH/.."
FOXBOX_BINARY="$PROJECT_HOME/target/debug/foxbox"
DEBUG_PROFILE="$HOME/.local/share/foxbox-tests"

# TODO: Put this part in NodeJS instead
rm -rf "${DEBUG_PROFILE}"

# build and launch foxbox daemon
pushd "${PROJECT_HOME}"
cargo build

# run tests
"${PROJECT_HOME}/node_modules/.bin/mocha" "${PROJECT_HOME}/test/selenium/*_test.js"
