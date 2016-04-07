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
After that, you should be all set in regars to compiling the project.

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

## Running the daemon

```bash
$ cargo run
```

To run with custom local host name (eg. foxbox.local):

```bash
$ cargo run -- -l foxbox
```

__NOTE:__ currently changing of host name is done via ```avahi-daemon``` and therefore supported only on Linux platform. To be able to change local host machine name user must be either included into ```netdev``` group or allow any other suitable user group to manage host name by adding the following policy to ```/etc/dbus-1/system.d/avahi-dbus.conf```:
```xml
<policy group="any_suitable_group_name">
  <allow send_destination="org.freedesktop.Avahi"/>
  <allow receive_sender="org.freedesktop.Avahi"/>
</policy>
```

Alternatively you can build the app without running it via:

```bash
$ cargo build
```

Foxbox also takes a number of command line parameters:

```bash
-v, --verbose : Toggle verbose output.
-l, --local-name <hostname> : Set local hostname. Linux only. Requires to be a member of the netdev group.
-p, --port <port>  : Set port to listen on for http connections. [default: 3000]
-w, --wsport <wsport> : Set port to listen on for websocket. [default: 4000]
-d, --profile <path> : Set profile path to store user data.
-r, --register <url> : URL of registration endpoint [default: http://localhost:4242]
-t, --tunnel <tunnel> : Set the tunnel endpoint hostname. If omitted, the tunnel is disabled.
-s, --tunnel-secret <secret> : Set the tunnel shared secret. [default: secret]
-c, --config <namespace;key;value> :  Set configuration override
-h, --help : Print this help menu.
--disable-tls : Run as a plain HTTP server, disabling encryption.
--dns-domain <domain> : Set the top level domain for public DNS. If omitted, the tunnel is disabled
--dns-api <url> : Set the DNS API endpoint
--remote-name: external domain of foxbox

```

Example:
```bash
# To start foxbox with the IP tunneling, HTTP only:
$ cargo run -- -r http://someserver.org:4242 -t someserver.org:443 -s secret --remote-name foxbox.someserver.org --disable-tls
# To change the philips hue nupnp server location to http://localhost:8002
$ cargo run -- -c "philips_hue;nupnp_url;http://localhost:8002/"
```

## Build time options
### Disable authentication
You may want to disable endpoints authentication to ease your development process. You can do that by removing `authentication` from the `default` feature in the `Cargo.toml` file.

```conf
[features]
default = []
authentication = []
```

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

This is required to build the openssl crate using homebrew's openssl:

``` bash
$ export DEP_OPENSSL_INCLUDE=/usr/local/opt/openssl/include/
```

Previous versions of these instructions described setting ```OPENSSL_INCLUDE_DIR```.
Make sure it is unset. In fact, an obsolete value may have been cached by cargo
which is fixed by ```cargo clean```.
