#/bin/bash

set -e

CURRENT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$CURRENT_PATH/travis-linux-common.source.sh"

_install_js_linter() {
    npm install
}

_lint() {
    _install_js_linter
    jshint static/main/js/*.js static/setup/js/*.js test/selenium/*.js test/integration/lib/*.js test/integration/test/*.js
}

build() {
    cargo build
    _lint
}

_set_up_unit_tests() {
    sudo usermod -a -G netdev "$USER" # "netdev" group membership allows to set custom host name via avahi-daemon.
}

_set_up_selenium_tests() {
    sh -e /etc/init.d/xvfb start
    export DISPLAY=:99.0
    wget http://selenium-release.storage.googleapis.com/2.53/selenium-server-standalone-2.53.0.jar
    java -jar selenium-server-standalone-2.53.0.jar > /dev/null &
    sleep 5
    nvm install 4.2
    nvm use 4.2
    npm install
}

_set_up_tests() {
    _set_up_unit_tests
    _set_up_selenium_tests
}

run_tests() {
    _set_up_tests
    npm run test-selenium
    npm run test-integration-travis
}
