#!/bin/sh
set -e

apk add --no-cache bash git ca-certificates docker-cli

apk add --no-cache --virtual .build-deps curl

ARCH=$(uname -m)
[ "$ARCH" = "x86_64" ] && COMPOSE_ARCH="x86_64" || COMPOSE_ARCH="aarch64"

mkdir -p /usr/lib/docker/cli-plugins

curl -SL "https://github.com/docker/compose/releases/latest/download/docker-compose-linux-$COMPOSE_ARCH" \
     -o /usr/lib/docker/cli-plugins/docker-compose
chmod +x /usr/lib/docker/cli-plugins/docker-compose

ln -sf /usr/lib/docker/cli-plugins/docker-compose /usr/bin/docker-compose
apk del .build-deps
rm -rf /var/cache/apk/* /tmp/*

curl -sS https://starship.rs/install.sh | sh -s -- --yes --bin-dir /usr/local/bin
echo 'export STARSHIP_CONFIG=/starship.toml' >> /root/.bashrc
echo 'eval "$(starship init bash)"' >> /root/.bashrc