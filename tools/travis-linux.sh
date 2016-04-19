#/bin/bash

set -e

CURRENT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

install_dependencies() {
    # Missing dependencies only. The regular ones are in done with the APT addon
    # defined in .travis.yml
    sudo apt-get -qq update
    # TODO: Move to apt addon once https://github.com/travis-ci/apt-package-whitelist/issues/1983 lands
    sudo apt-get install -y avahi-daemon libavahi-client-dev libavahi-common-dev libdbus-1-dev
}

build() {
    cargo build
}

lint() {
    jshint static/main/js/*.js static/setup/js/*.js test/selenium/*.js test/integration/lib/*.js test/integration/test/*.js
}

_set_up_unit_tests() {
    sudo usermod -a -G netdev,dialout "$USER"
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

set_up_tests() {
    _set_up_unit_tests
    _set_up_selenium_tests
}

run_tests() {
    npm run test-selenium
    npm run test-integration-travis
    # Note: Cargo recompiles every dependency with dead code. That's why
    # this step is currently the last
    "$CURRENT_PATH/execute-unit-tests-with-coverage.sh"
}
