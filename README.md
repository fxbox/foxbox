FoxBox
======

[![Build Status](https://travis-ci.org/fxbox/foxbox.svg?branch=master)](https://travis-ci.org/fxbox/foxbox)
[![License](https://img.shields.io/badge/license-MPL2-blue.svg)](https://raw.githubusercontent.com/fxbox/foxbox/master/LICENSE)


## Target

Target hardware for prototyping is the Raspberry Pi 2. It's ARMv7 with Neon, but building without Neon support should do for now. Target OS is the latest Raspbian which is based on Debian 8.0 Jessie.


## Toolchain

Rust 1.8+ is required. We're building with rust nightly (1.8.0 as of 2016-02-04).
(what's the revision from git for this build ```rustc -V``` should say?)

Cross Compilation Toolchain

If you prefer to compile on your dev system, this is the way to go. There's an experimental toolchain at 
https://people.mozilla.org/~fdesre/rust-rpi2.tar.gz which Fabrice compiled on Ubuntu 15.10. It is built against a specific version of 
libstdc++.so.6 (GLIBCXX_3.4.21), so it may or may not work on other distributions. Ubuntu 15.04 is reportedly no good.

 * Download and and unpack toolchain linked above in $toolchain
 * Add $toolchain/x-tools/bin and $toolchain/bin to your PATH
 * Add $toolchain/lib to your LD_LIBRARY_PATH

To build a rust file:

``` bash
$ rustc --target=armv7-unknown-linux-gnueabihf -C linker=armv7-unknown-linux-gnueabihf-g++ hello.rs
```

## Building and running locally on Linux

This should work straight-forward. Install a rust nightly, clone the repo, and then cargo run.


## Building and running locally on OS X

``` bash
$ brew install openssl ... ... ... # TODO: other build dependencies?
$ brew install multirust
$ multirust update
$ multirust default nightly
```

This is required to build the openssl crate using homebrew's openssl:

``` bash
$ export DEP_OPENSSL_INCLUDE=/usr/local/opt/openssl/include/
$ export OPENSSL_INCLUDE_DIR=/usr/local/Cellar/openssl/1.0.2f/include/
```

This then builds and runs the project locally:

``` bash
$ git clone https://github.com/fxbox/foxbox
$ cd foxbox
$ cargo build
$ cargo run
```


## Building on Raspbian

Rustc doesn't build natively on the Raspberry Pi, yet, because the rust team is not offering ARM binaries for staging at this point. However, there are working ARMv7 binary builds at https://github.com/warricksothr/RustBuild . Grab and install a nightly build of rust, rustlib, and cargo from there. After that, clone and cargo run the Foxbox repo.
