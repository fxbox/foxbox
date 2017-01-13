FROM ubuntu:14.04
MAINTAINER Fabrice Desré <fabrice@desre.org>

ADD sources.list /etc/apt/

RUN dpkg --add-architecture armhf
RUN apt-get update
RUN apt-get upgrade -y
RUN apt-get install -y \
  build-essential \
  cpp gcc g++ cpp-4.8 gcc-4.8 g++-4.8 \
  autoconf \
  automake \
  curl \
  file \
  libtool \
  shtool \
  python3 \
  g++-arm-linux-gnueabihf \
  libasound2:armhf \
  libssl-dev:armhf \
  libespeak-dev:armhf \
  libupnp6-dev:armhf \
  libudev-dev:armhf \
  libavahi-client-dev:armhf \
  libsqlite3-dev:armhf \
  libev-dev:armhf

RUN apt-get clean

ENV SHELL=/bin/bash

RUN useradd -m -d /home/user -p user user

# open-zwave wants -cc and -c++ but I could not find a package providing them.
RUN ln -s /usr/bin/arm-linux-gnueabihf-gcc /usr/bin/arm-linux-gnueabihf-cc
RUN ln -s /usr/bin/arm-linux-gnueabihf-g++ /usr/bin/arm-linux-gnueabihf-c++

# Toolchain aliasing for servo
RUN ln -s /usr/bin/arm-linux-gnueabihf-ar    /usr/bin/arm-unknown-linux-gnueabihf-ar
RUN ln -s /usr/bin/arm-linux-gnueabihf-gcc   /usr/bin/arm-unknown-linux-gnueabihf-gcc
RUN ln -s /usr/bin/arm-linux-gnueabihf-g++   /usr/bin/arm-unknown-linux-gnueabihf-g++
RUN ln -s /usr/bin/arm-linux-gnueabihf-ld    /usr/bin/arm-unknown-linux-gnueabihf-ld
RUN ln -s /usr/bin/arm-linux-gnueabihf-strip /usr/bin/arm-unknown-linux-gnueabihf-strip

RUN ln -s /usr/bin/arm-linux-gnueabihf-ar    /usr/bin/armv7-unknown-linux-gnueabihf-ar
RUN ln -s /usr/bin/arm-linux-gnueabihf-gcc   /usr/bin/armv7-unknown-linux-gnueabihf-gcc
RUN ln -s /usr/bin/arm-linux-gnueabihf-g++   /usr/bin/armv7-unknown-linux-gnueabihf-g++
RUN ln -s /usr/bin/arm-linux-gnueabihf-ld    /usr/bin/armv7-unknown-linux-gnueabihf-ld
RUN ln -s /usr/bin/arm-linux-gnueabihf-strip /usr/bin/armv7-unknown-linux-gnueabihf-strip

ENV PKG_CONFIG_ALLOW_CROSS=1
ENV PKG_CONFIG_PATH=/usr/lib/arm-linux-gnueabihf/pkgconfig

USER user
WORKDIR /home/user

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly-2017-01-12
ENV PATH=/home/user/.cargo/bin:/home/user/bin:$PATH
RUN rustup target add armv7-unknown-linux-gnueabihf

ENV LD_LIBRARY_PATH=/home/user/lib:$LD_LIBRARY_PATH

# For rust-crypto
ENV CC=arm-linux-gnueabihf-gcc

# For open-zwave
ENV CROSS_COMPILE=arm-linux-gnueabihf-

RUN mkdir -p dev/source
RUN mkdir dev/.cargo
RUN mkdir /home/user/bin

ADD cargoarmhf /home/user/bin
 
ADD armhf-linker /home/user/bin

WORKDIR /home/user/dev/source

