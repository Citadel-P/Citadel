#!/bin/sh
set -eu

# Explicit non-root users (including development) retain their configured groups.
if [ "$(id -u)" = 0 ]; then
  socket=${CITADEL_RUST_DOCKER_SOCKET:-/var/run/docker.sock}
  runtime_groups=0
  if [ -S "$socket" ]; then
    socket_group=$(stat -c '%g' "$socket")
    if [ "$socket_group" != 0 ]; then
      runtime_groups="0,$socket_group"
    fi
  fi

  exec setpriv --reuid=65532 --regid=0 --groups="$runtime_groups" \
    /app/citadel-server "$@"
fi

exec /app/citadel-server "$@"
