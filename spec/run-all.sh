#!/usr/bin/env bash

set -e

if [[ "$AUREUM_EXEC" != /* ]]
then
  echo "Please specify the absolute path to the aureum executable (AUREUM_EXEC=<absolute path>)"
  exit 1
fi


"$AUREUM_EXEC" basic01/test.toml
"$AUREUM_EXEC" basic02/test.toml
"$AUREUM_EXEC" basic03/test.toml
"$AUREUM_EXEC" basic04/test.toml
AUREUM_MY_CUSTOM_ENV_VAR="Hello world" "$AUREUM_EXEC" basic05/test.toml
"$AUREUM_EXEC" basic06/test.toml

"$AUREUM_EXEC" aureum01/test.toml
