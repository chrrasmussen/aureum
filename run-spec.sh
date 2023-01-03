#!/usr/bin/env bash

set -e

cargo run -- spec/basic01/test.toml
cargo run -- spec/basic02/test.toml
