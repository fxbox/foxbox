install_dependencies() {
    # Missing dependencies only. The regular ones are in done with the APT addon
    # defined in .travis.yml
    sudo apt-get -qq update
    # TODO: Move to apt addon once https://github.com/travis-ci/apt-package-whitelist/issues/1983 lands
    sudo apt-get install -y avahi-daemon libavahi-client-dev libavahi-common-dev libdbus-1-dev
}
