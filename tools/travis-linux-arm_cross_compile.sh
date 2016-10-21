#!/bin/bash

set -ev

BUILD_TARGET='arm-linux-gnueabihf'
RUST_TARGET='armv7-unknown-linux-gnueabihf'

_install_arm_packages() {
    sudo tee -a /etc/apt/sources.list <<EOF
deb [arch=armhf,arm64] http://ports.ubuntu.com/ xenial main
deb [arch=armhf,arm64] http://ports.ubuntu.com/ xenial-updates main
deb [arch=armhf,arm64] http://ports.ubuntu.com/ xenial universe
deb [arch=armhf,arm64] http://ports.ubuntu.com/ xenial-updates universe
EOF

    sudo dpkg --add-architecture armhf
    # XXX Network issues might make the package discovery fail. If it impacts
    # one of our packages, we will notice it at installation time
    set +e
    sudo apt-get update
    set -e

    sudo apt-get install -y --no-install-recommends build-essential curl file \
        "g++-$BUILD_TARGET"

    sudo apt-get install -y --no-install-recommends libasound2:armhf \
        libssl-dev:armhf libespeak-dev:armhf libupnp6-dev:armhf \
        libudev-dev:armhf libavahi-client-dev:armhf libsqlite3-dev:armhf
}

_set_up_cargo_config() {
    mkdir -p "$HOME/.cargo"
    touch "$HOME/.cargo/config"
    tee -a "$HOME/.cargo/config" << EOF
[target.$RUST_TARGET]
linker = "$BUILD_TARGET-gcc"
EOF
}

_set_up_environment() {
    _set_up_cargo_config

    # open-zwave wants -cc and -c++ but no package seems to provid them.
    sudo cp "/usr/bin/$BUILD_TARGET-gcc" "/usr/bin/$BUILD_TARGET-cc"
    sudo cp "/usr/bin/$BUILD_TARGET-g++" "/usr/bin/$BUILD_TARGET-c++"

    # For rust-crypto
    export CC="$BUILD_TARGET-gcc"

    # For open-zwave
    export CROSS_COMPILE="$BUILD_TARGET-"

    export PKG_CONFIG_LIBDIR="/usr/lib/$BUILD_TARGET/pkgconfig"
    export PKG_CONFIG_ALLOW_CROSS=1
}

_install_rust_for_arm() {
    # Remove preinstalled rust
    rm -rf ~/rust

    sh ~/rust-installer/rustup.sh --prefix=~/rust --spec="$TRAVIS_RUST_VERSION"\
        -y --disable-sudo --with-target="$RUST_TARGET"
}

install_dependencies() {
    _install_arm_packages
    _set_up_environment
    _install_rust_for_arm
}

build() {
    # Both target should be validated
    echo "build: Debug compilation"
    cargo build --target="$RUST_TARGET"
    echo "build: Release compilation"
    cargo build --target="$RUST_TARGET" --release
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
