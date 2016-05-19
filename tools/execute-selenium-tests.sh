#!/bin/bash

set -ex

CURRENT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_HOME="$CURRENT_PATH/.."

# build and launch foxbox daemon
pushd "${PROJECT_HOME}"
cargo build

# run tests
"${PROJECT_HOME}/node_modules/.bin/mocha" "${PROJECT_HOME}/test/selenium/app_test.js"
