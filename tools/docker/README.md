Cross-compiling FoxBox
======================

# Adding a new target

Each target lives in its own directory (see `rpi2` for instance), where the
scripts expect to find the following:
- A `Dockerfile` to create the docker image. This image must provide all the
tools needed to cross-compile for the target.
- A `gcc_triple` file containing the gcc triple definition for this target.
- A `rust_target` file containing the Rust target definition.

# Usage

Three scripts are provided to manage builds:

## Creating Docker images
Use `./docker.sh TARGET_NAME` to create a local Docker image name `foxbox-TARGET`.

## Building
Run `./build.sh TARGET_NAME` to build a release version.

## Packaging
Run `./package.sh TARGET_NAME` to create a package in `builds/TARGET_NAME/foxbox-TARGET_NAME-DATE.tar.bz2`.