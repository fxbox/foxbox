#!/bin/bash

# Update this when doing a rustup.
EXPECTED_HASH="2782e8f8fcefdce77c5e0dd0846c15c4c5103d84"
EXPECTED_DATE="2017-01-12"

CURRENT_HASH=`rustc --version -v|grep commit-hash|cut -f 2 -d ' '`

install_rustc() {
    echo "Checking for rustup"
    RUSTUP=`which rustup`
    if [[ "$RUSTUP" == "" ]]; then
        echo "Installing rustup"
        curl https://sh.rustup.rs -sSf | sh
        RUSTUP=`which rustup`
    fi
    $RUSTUP install nightly-$EXPECTED_DATE
    $RUSTUP override set nightly-$EXPECTED_DATE
}

prompt_rust_install() {
    while true; do
        read -p "Do you wish to install the correct Rustc version? [y/n] " yn
        case $yn in
            [Yy]* ) install_rustc; break;;
            [Nn]* ) exit;;
            * ) echo "Please answer yes or no.";;
        esac
    done
}

if [ "$CURRENT_HASH" != "$EXPECTED_HASH" ]; then
    echo "You need Rustc nightly from $EXPECTED_DATE to build."
    echo "Found $CURRENT_HASH but expected $EXPECTED_HASH"
    prompt_rust_install
fi

echo "Building..."
set -e -x
cargo build
