#!/usr/bin/env bash

set -e

AUREUM_EXEC="$PWD/target/debug/aureum" cargo run -- spec/all.au.toml
