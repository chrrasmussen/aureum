#!/usr/bin/env bash

set -e

export AUREUM_EXEC="$PWD/target/debug/aureum"
export AUREUM_TEST_HELLO_WORLD="Hello world" # Required by `basic05`

ARGS=${*:-spec.au.toml}

cargo run -- $ARGS
