#!/bin/bash

set -e

_install_arm_packages() {
    sudo tee -a /etc/apt/sources.list <<EOF
deb [arch=armhf,arm64] http://ports.ubuntu.com/ trusty main
deb [arch=armhf,arm64] http://ports.ubuntu.com/ trusty-updates main
deb [arch=armhf,arm64] http://ports.ubuntu.com/ trusty universe
deb [arch=armhf,arm64] http://ports.ubuntu.com/ trusty-updates universe
EOF

    sudo dpkg --add-architecture armhf
    # XXX Network issues might make the package discovery fail. If it impacts
    # one of our packages, we will notice it at installation time
    set +e
    sudo apt-get update
    set -e

    sudo apt-get install -y --no-install-recommends build-essential curl file \
        g++-arm-linux-gnueabihf

    sudo apt-get install -y --no-install-recommends libasound2:armhf \
        libssl-dev:armhf libespeak-dev:armhf libupnp6-dev:armhf \
        libudev-dev:armhf libavahi-client-dev:armhf libsqlite3-dev:armhf
}

_set_up_environment() {
    # open-zwave wants -cc and -c++ but no package seems to provid them.
    sudo cp /usr/bin/arm-linux-gnueabihf-gcc /usr/bin/arm-linux-gnueabihf-cc
    sudo cp /usr/bin/arm-linux-gnueabihf-g++ /usr/bin/arm-linux-gnueabihf-c++

    # For rust-crypto
    export CC=arm-linux-gnueabihf-gcc

    # For open-zwave
    export CROSS_COMPILE=arm-linux-gnueabihf-

    tee -a $HOME/.cargo/config << EOF
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
EOF
}

_install_rust_for_arm() {
    # Remove preinstalled rust
    rm -rf ~/rust

    sh ~/rust-installer/rustup.sh --prefix=~/rust --spec="$TRAVIS_RUST_VERSION"\
        -y --disable-sudo --with-target=armv7-unknown-linux-gnueabihf
}

install_dependencies() {
    _install_arm_packages
    _set_up_environment
    _install_rust_for_arm
}

build() {
    # Both target should be validated
    echo "build: Debug compilation"
    cargo build --target=armv7-unknown-linux-gnueabihf
    echo "build: Release compilation"
    cargo build --target=armv7-unknown-linux-gnueabihf --release
}

lint() {
    echo "lint: No linting needed for cross-compilation. Skipping..."
}

set_up_tests() {
    echo "set_up_tests: no set up required. Skipping..."
}

run_tests() {
    echo "run_tests: No specific tests exist for cross-compilation. Skipping..."
}
