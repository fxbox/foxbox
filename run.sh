#!/bin/sh

PATH=target/debug:"$PATH" cargo run --bin foxbox "$@"
