#!/usr/bin/env bash

# Run this script to initialize a Valkey container for testing.

# Uncomment to enable debug mode
#set -x
set -eo pipefail

# What container engine to use (Podman or Docker)
CONTAINER_ENGINE="${CONTAINER_ENGINE:=podman}"

# Comment this if you prefer not to use valkey-cli, or modify to use redis-cli
CLI_TOOL="${CLI_TOOL:=valkey-cli}"

if ! [ -x "$(command -v $CLI_TOOL)" ]; then
        echo >&2 "Error: $CLI_TOOL is not installed."
        exit 1
fi

# Allow to skip Docker if a dockerized Postgres database is already running
if [[ -z "${SKIP_DOCKER}" ]]
then
    $CONTAINER_ENGINE run \
        -p 6379:6379 \
        -d valkey/valkey:latest
fi

# Let the container start up
sleep 3

>&2 echo "Valkey ready to go!"
