# FoxBox

[![Build Status](https://travis-ci.org/fxbox/foxbox.svg?branch=master)](https://travis-ci.org/fxbox/foxbox)
[![Coverage Status](https://coveralls.io/repos/github/fxbox/foxbox/badge.svg?branch=master)](https://coveralls.io/github/fxbox/foxbox?branch=master)
[![License](https://img.shields.io/badge/license-MPL2-blue.svg)](https://raw.githubusercontent.com/fxbox/foxbox/master/LICENSE)


## Technologies

### Rust

We're using Rust for the daemon/server.

Currently a fairly recent nightly is required. To determine which version of rust is being used, check the [.travis.yml](https://github.com/fxbox/foxbox/blob/master/.travis.yml) file.

Look for these 2 lines near the top of the file:
```yaml
rust:
    - nightly-YYYY-MM-DD
```

It's recommended that you use [`multirust`](https://github.com/brson/multirust) to install and switch between versions of Rust. You should then be able to then use:
```
cd /your/path/to/foxbox     # Required, otherwise you might replace rustc for another project
multirust override nightly-YYYY-MM-DD   # Replace with the correct date you found
```
After that, you should be all set in regard to compiling the project.

#### :warning: Warning

Sometimes, there might be a 1-day-difference between the date shown in `.travis.yml` and the one reported by `rustc`. For example [nightly-2016-04-06](https://static.rust-lang.org/dist/2016-04-06/) corresponds to:
```bash
$ rustc -V
rustc 1.9.0-nightly (241a9d0dd 2016-04-05)
```

#### Build requirements

| Dependency   | Debian/Raspian        | Fedora          | Arch               | OS X (Homebrew) |
| ------------ | --------------------- | --------------- | ------------------ | --------------- |
| `libupnp`    | `libupnp-dev`         | `libupnp-devel` | `extra/libupnp`    | `libupnp`       |
| `libssl`     | `libssl-dev`          | `openssl-devel` | via `base-devel`   | `openssl`       |
| `libavahi`   | `libavahi-client-dev` | `avahi-devel`   | `extra/avahi`      | `n.a.`          |
| `libsqlite3` | `libsqlite3-dev`      | `sqlite-devel`  | `core/sqlite`      | `sqlite`        |
| `libespeak`  | `libsespeak-dev`      | `espeak-devel`  | `community/espeak` | `?`             |
| `libdbus`    | `?`                   | `dbus-devel`    | `core/libdbus`     | `?`             |

### Node

We're using Node to run Selenium tests. Currently v4.x LTS. We plan to stay on
stable LTS releases. It's recommended that you use
[`nvm`](https://github.com/creationix/nvm) to install and switch between
versions of Node.


## Target hardware

We're using the Raspberry Pi 2 as a prototyping target (ARMv7). The target
operating system is the latest Raspbian which is based on Debian 8.0 Jessie.


## Contributing

Note: We're in an iterative prototyping phase of the project. Things are moving
really fast so it may be easier to contribute when the dust starts to settle.
You've been warned. :shipit:

### Forks and feature branches

You should fork the main repo and create pull requests against feature branches
of your fork. If you need some guidance with this see:

 - https://guides.github.com/introduction/flow/
 - http://scottchacon.com/2011/08/31/github-flow.html


## Setup

```bash
$ git clone git@github.com:<username>/foxbox.git
$ cd foxbox
```

## Build time options
### Disable authentication
You may want to disable endpoints authentication to ease your development process. You can do that by removing `authentication` from the `default` feature in the `Cargo.toml` file.

```conf
[features]
default = []
authentication = []
```

## Running the daemon

Currently you would likely want to start the daemon like this:

```bash
cargo run -- --disable-tls
```

You will be disabling [TLS](https://wiki.mozilla.org/Connected_Devices/Projects/Project_Link/TLS) support. We hope to have out-of-the-box TLS support ready pretty soon, but for now disabling it is the easiest way to run foxbox.

### More options

To see every available options, run: `cargo run -- --help`. Here below are detailed some specific configurations.

#### Enable tunneling support

If you want to access your foxbox from outside of the network where it is running, you'll need to enable [tunneling](https://wiki.mozilla.org/Connected_Devices/Projects/Project_Link/Tunneling) support. To do that you need to specify the address of the tunneling server that you want to use, the shared secret for this server (if any) and the remote name that you want to use to access to your foxbox from outside of your foxbox' local network.

```bash
cargo run -- -r http://knilxof.org:4242 -t knilxof.org:443 -s secret --remote-name yourname.knilxof.org --disable-tls
```

In the example above, `knilxof.org:443` is the location of our tunneling dev server, which has a not-that-secret-anymore value that you'll need to ask for on [IRC](https://wiki.mozilla.org/Connected_Devices/Projects/Project_Link#IRC). You are supposed to substitute `<yourname>` by the subdomain of your choice, but take into account that you'll need to keep the domain name of the tunneling server, in this case `.knilxof.org`. Starting the daemon with the command line options above you should be able to access your foxbox through `http://yourname.knilxof.org`.

#### Custom Philips Hue nUPNP server

```
$ cargo run -- -c "philips_hue;nupnp_url;http://localhost:8002/"
```

#### Common issues

##### "Failed set host name"

Since [3e758cc](https://github.com/fxbox/foxbox/commit/3e758cc83a0511f5b6a206d6ee10df308f04456d), foxbox tries to set a custom host name for the Avahi daemon. This is a Linux-only feature. Depending on your configuration, you may get this error:
```
Running `target/debug/foxbox --disable-tls`
thread '<main>' panicked at 'Failed set host name: Access denied (code -20)', /home/YOUR_LOGIN/.multirust/toolchains/nightly-2016-04-10/cargo/git/checkouts/multicast-dns-d51af2fa146824d6/a6e4bcc/src/adapters/avahi/adapter.rs:346
```

To get rid of it, without running the box with `sudo`, add your user to the group that allows to administrate the network. On Ubuntu this group is called `netdev`, on Arch Linux, it's `network`.

If your distribution is different, take a look at the avahi configuration file (`/etc/dbus-1/system.d/avahi-dbus.conf`), and find the group that has these rights:
```xml
<policy group="GROUP_NAME_TO_LOOK_FOR">
  <allow send_destination="org.freedesktop.Avahi"/>
  <allow receive_sender="org.freedesktop.Avahi"/>
</policy>
```

Then, add yourself to the group, login again and run foxbox. You might need to wait about 1 minute to be able to run foxbox with the corrects rights.


## Interacting with the daemon

Once you have your foxbox up and running you can try our [demo application](https://github.com/fxbox/app) by browsing to [https://fxbox.github.io/app](https://fxbox.github.io/app).

Alternatively, you can use the foxbox' current [REST API](https://wiki.mozilla.org/Connected_Devices/Projects/Project_Link/Taxonomy#Current_REST_API)

## Rust tests

```bash
$ cargo test
```

## Selenium tests

You'll need to make sure you install the dependencies via:

```bash
$ npm install
```

Then you can run the Selenium tests via:

```bash
$ cargo run -- --disable-tls
$ npm run test-selenium
```

## Cross compiling to ARM

There is no one solution for this. The process will be different depending on
your operating system. You may be able to build on a RPi, but the larger the
application gets, the slower and more painful this will be (not recommended).

### Linux

@fabricedesre has created a script to help compile a toolchain. So far it's
only been tested on Ubuntu but there's nothing ubuntu specific so that should
work just fine on any Linux.

 - https://github.com/fabricedesre/rustpi2

For an extensive write-up about cross compiling Rust programs see:

 - https://github.com/japaric/rust-cross


### Mac OS X

Cross compiling on Mac hasn't been documented. A PR is welcomed. :wink:


## Notes for Mac OS X

You'll need some dependencies installed to build.

``` bash
$ brew install openssl libupnp sqlite
```

This is required to build/link the openssl crate and foxbox using homebrew's openssl:

``` bash
$ export DEP_OPENSSL_INCLUDE=/usr/local/opt/openssl/include/
$ export OPENSSL_LIB_DIR=/usr/local/opt/openssl/lib
$ export EXTRA_LDFLAGS=-L/usr/local/opt/openssl/lib
```

Previous versions of these instructions described setting ```OPENSSL_INCLUDE_DIR```.
Make sure it is unset. In fact, an obsolete value may have been cached by cargo
which is fixed by ```cargo clean```.
