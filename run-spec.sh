#!/usr/bin/env bash

set -e

cargo run -- spec/basic01/test.toml
cargo run -- spec/basic02/test.toml
cargo run -- spec/basic03/test.toml
cargo run -- spec/basic04/test.toml
AUREUM_MY_CUSTOM_ENV_VAR="Hello world" cargo run -- spec/basic05/test.toml

AUREUM_EXEC="$PWD/target/debug/aureum" cargo run -- spec/aureum01/test.toml
