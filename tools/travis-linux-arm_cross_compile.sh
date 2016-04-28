#!/bin/bash

set -ev

# Global env $RELEASE
BUILD_TARGET='arm-linux-gnueabihf'
RUST_TARGET='armv7-unknown-linux-gnueabihf'

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
        "g++-$BUILD_TARGET"

    sudo apt-get install -y --no-install-recommends libasound2:armhf \
        libssl-dev:armhf libespeak-dev:armhf libupnp6-dev:armhf \
        libudev-dev:armhf libavahi-client-dev:armhf libsqlite3-dev:armhf
}

_set_up_environment() {
    # open-zwave wants -cc and -c++ but no package seems to provid them.
    sudo cp "/usr/bin/$BUILD_TARGET-gcc" "/usr/bin/$BUILD_TARGET-cc"
    sudo cp "/usr/bin/$BUILD_TARGET-g++" "/usr/bin/$BUILD_TARGET-c++"

    # For rust-crypto
    export CC="$BUILD_TARGET-gcc"

    # For open-zwave
    export CROSS_COMPILE="$BUILD_TARGET-"

    tee -a $HOME/.cargo/config << EOF
[target.$RUST_TARGET]
linker = "$BUILD_TARGET-gcc"
EOF

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
    if [[ $RELEASE == "true" ]]; then
        FLAGS='--release'
    fi

    cargo build --target="$RUST_TARGET" $FLAGS
}

run_tests() {
    echo "run_tests: No specific tests exist for cross-compilation. Skipping..."
}
