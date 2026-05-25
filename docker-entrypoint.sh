#!/bin/sh
set -eu

mkdir -p /app/data /app/logs

if [ "$(id -u)" = "0" ]; then
  chown -R 10001:10001 /app/data /app/logs
  exec gosu tiphia "$@"
fi

exec "$@"