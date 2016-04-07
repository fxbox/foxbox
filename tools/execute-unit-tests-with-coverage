#!/bin/bash

# Warning: kcov is a Linux only tool

set -ex

CURRENT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

PROJECT_HOME="$CURRENT_PATH/.."
PROJECT_NAME=$(sed --quiet 's/^name *= *"\(.*\)"$/\1/p' $PROJECT_HOME/Cargo.toml)
PROJECT_BINARY_LOCATION="$PROJECT_HOME/target/debug"

KCOV_VERSION="30"
KCOV_TEMP="$PROJECT_HOME/target/kcov"
KCOV_COMPILE_HOME="$KCOV_TEMP/kcov-$KCOV_VERSION"
KCOV_BINARY="$KCOV_TEMP/kcov"

get_prebuilt() {
    local ubuntu_version="$1"
    curl --location --output "$KCOV_BINARY" \
      "https://github.com/JohanLorenzo/kcov/releases/download/v$KCOV_VERSION/kcov_$ubuntu_version"
    chmod +x "$KCOV_BINARY"
}

get_and_compile_kcov_locally() {
  curl --location --output "$KCOV_TEMP/kcov.tar.gz" \
    "https://github.com/SimonKagstrom/kcov/archive/v$KCOV_VERSION.tar.gz"
  tar xvf "$KCOV_TEMP/kcov.tar.gz" --directory="$KCOV_TEMP"
  cd "$KCOV_COMPILE_HOME"
  cmake .
  make
  cp "src/kcov" "$KCOV_BINARY"
  cd -
}

get_kcov() {
    mkdir -p "$KCOV_TEMP"

    local ubuntu_version=$(lsb_release --codename --short)
    if [[ "$ubuntu_version" == 'precise' || "$ubuntu_version" == 'trusty' ]] ; then
        get_prebuilt "$ubuntu_version"
    else
        get_and_compile_kcov_locally
    fi
}

compile_foxbox_with_dead_code() {
  RUSTFLAGS="-C link-dead-code" cargo test --no-run
}

run_tests_and_coverage() {
  PROJECT_UNIT_TEST_BINARY=$(find "$PROJECT_BINARY_LOCATION" -maxdepth 1 -executable -name "$PROJECT_NAME"-\*)
  RUST_BACKTRACE=1 "$KCOV_BINARY" \
    --exclude-path="${CARGO_HOME:=~/.cargo},\
                    $PROJECT_HOME/src/stubs,\
                    $PROJECT_HOME/target" \
    --coveralls-id="${TRAVIS_JOB_ID:=no-job-id}" \
    "$PROJECT_HOME/target/coverage-report/" \
    "$PROJECT_UNIT_TEST_BINARY"
}


if ! [ -f "$KCOV_BINARY" ] ; then
  get_kcov
fi

compile_foxbox_with_dead_code
run_tests_and_coverage
