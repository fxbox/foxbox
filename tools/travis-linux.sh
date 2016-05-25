#/bin/bash

set -ev

CURRENT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$CURRENT_PATH/travis-linux-common.sh"


build() {
    cargo build
}

lint() {
    echo 'nothing to do'
}

set_up_tests() {
    echo 'nothing to do'
}

_e2e_tests() {
    git clone https://github.com/fxbox/app.git
    cd app
    npm install
    npm install -g gulp
    gulp test-e2e
}

run_tests() {
    _e2e_tests
}
