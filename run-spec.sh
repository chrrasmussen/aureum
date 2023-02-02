#!/usr/bin/env bash

set -e

export AUREUM_TEST_EXEC="$PWD/target/debug/aureum"
export AUREUM_TEST_HELLO_WORLD="Hello world" # Required by `basic/read-env-var.au.toml`

cargo run -- "${@:-spec.au.toml}"
