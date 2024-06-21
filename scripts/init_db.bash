#!/usr/bin/env bash

# Script that automates the initialization process of the DB backend. Overwrite
# the following variables for a deployment scenario. Default values are only
# advised for development scenarios.

set -x
set -eo pipefail

# Check if a custom user has been set, otherwise default to 'mariadb'
DB_USER="${MARIADB_USER:=user}"
# Check if a custom password has been set, otherwise default to 'password'
DB_PASSWORD="${MARIADB_PASSWORD:=password}"
# Check if a custom root password has been set, otherwise default to 'password'
DB_ROOT_PASSWORD="${MARIADB_ROOT_PASSWORD:=password}"
# Check if a custom database name has been set, otherwise default to 'test'
DB_NAME="${MARIADB_DB:=test}"
# Check if a custom port has been set, otherwise default to '3306'
DB_PORT="${MARIADB_PORT:=3306}"
# Check if a custom host has been set, otherwise default to 'localhost'
DB_HOST="${MARIADB_HOST:=127.0.0.1}"

docker run \
    -e MARIADB_USER=${DB_USER} \
    -e MARIADB_ROOT_PASSWORD=${DB_PASSWORD} \
    -e MARIADB_DATABASE=${DB_NAME} \
    -e MARIADB_ROOT_PASSWORD=${DB_ROOT_PASSWORD} \
    -p "${DB_PORT}":3306 \
    -d mariadb


# Timeout to wait until the DB engine is ready to accept requests
sleep 10

export DATABASE_URL=mariadb://$root:${DB_ROOT_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}
sqlx database create
sqlx migrate run


