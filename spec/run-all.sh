#!/usr/bin/env bash

set -e

if [[ "$AUREUM_EXEC" != /* ]]
then
  echo "Please specify the absolute path to the aureum executable (AUREUM_EXEC=<absolute path>)"
  exit 1
fi


"$AUREUM_EXEC" basic01/test.au.toml
"$AUREUM_EXEC" basic02/test.au.toml
"$AUREUM_EXEC" basic03/test.au.toml
"$AUREUM_EXEC" basic04/test.au.toml
AUREUM_MY_CUSTOM_ENV_VAR="Hello world" "$AUREUM_EXEC" basic05/test.au.toml
"$AUREUM_EXEC" basic06/test.au.toml

"$AUREUM_EXEC" aureum01/test.au.toml
"$AUREUM_EXEC" aureum02/test.au.toml
