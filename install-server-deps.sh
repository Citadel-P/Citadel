#!/bin/sh
set -e

apk add --no-cache curl ca-certificates tar gzip deno restic tzdata

ARCH=$(uname -m)
case "$ARCH" in
  x86_64) API_FILTER="linux_amd64" ;;
  aarch64) API_FILTER="linux_arm64" ;;
  *) echo "Unsupported architecture: $ARCH" && exit 1 ;;
esac

URL=$(curl -s https://api.github.com/repos/nicholas-fedor/shoutrrr/releases/latest \
  | grep -o "https://[^\"]*${API_FILTER}[^\"]*\.tar\.gz" \
  | head -n 1)

echo "Downloading: $URL"

curl -L "$URL" | tar -xz -C /usr/local/bin

chmod +x /usr/local/bin/shoutrrr

deno --version
restic version

apk del curl tar gzip
rm -rf /var/cache/apk/* /tmp/*
