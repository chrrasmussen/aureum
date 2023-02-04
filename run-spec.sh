#!/usr/bin/env bash

set -e

# GitHub's Windows runner has multiple Bash shells: https://github.com/actions/runner-images/blob/751fe08d9840d2273fb1986980c5f18f3a920e64/images/win/Windows2022-Readme.md#shells
export AUREUM_TEST_BASH="${SHELL:-bash}" # Use the same shell that is executing this file
export AUREUM_TEST_EXEC="$PWD/target/debug/aureum"
export AUREUM_TEST_HELLO_WORLD="Hello world" # Required by `basic/read-env-var.au.toml`

cargo run -- "${@:-spec.au.toml}"
