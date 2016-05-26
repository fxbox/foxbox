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

_set_up_selenium_tests() {
    sh -e /etc/init.d/xvfb start
    export DISPLAY=:99.0
    wget http://selenium-release.storage.googleapis.com/2.53/selenium-server-standalone-2.53.0.jar
    java -jar selenium-server-standalone-2.53.0.jar > /dev/null &
    sleep 5
    nvm install 4.2
    nvm use 4.2
}

set_up_tests() {
    _set_up_selenium_tests
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
