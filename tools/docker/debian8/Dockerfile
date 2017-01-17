FROM debian:jessie
MAINTAINER Fabrice Desré <fabrice@desre.org>

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
  libasound2 \
  libssl-dev \
  libespeak-dev \
  libupnp6-dev \
  libudev-dev \
  libavahi-client-dev \
  libsqlite3-dev \
  libev-dev

RUN apt-get clean

ENV SHELL=/bin/bash

RUN useradd -m -d /home/user -p user user

USER user
WORKDIR /home/user

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly-2017-01-12
ENV PATH=/home/user/.cargo/bin:/home/user/bin:$PATH

ENV LD_LIBRARY_PATH=/home/user/lib:$LD_LIBRARY_PATH

RUN mkdir -p dev/source
RUN mkdir dev/.cargo
RUN mkdir /home/user/bin

ADD cargodebian8 /home/user/bin

WORKDIR /home/user/dev/source

