#!/bin/sh
# Runtime dependencies for the Alpine-based Core image.
set -eu

case "${TARGETARCH:-$(uname -m)}" in
  amd64|x86_64)
    archive=shoutrrr_linux_amd64_0.19.0.tar.gz
    checksum=e857441d23747486a4404b1fd1e6a82d1fbe8e0397fabddf412955f15c2e3e57
    ;;
  arm64|aarch64)
    archive=shoutrrr_linux_arm64v8_0.19.0.tar.gz
    checksum=67321d97a7ebd19f6f5d7bbeabb6337256882716eee28e8204828fe314c9e215
    ;;
  *) echo "Unsupported architecture: ${TARGETARCH:-$(uname -m)}" >&2; exit 1 ;;
esac

apk add --no-cache \
  bash ca-certificates git openssh-client postgresql18-client restic setpriv tini
apk add --no-cache --virtual .fetch-deps curl

archive_path=$(mktemp)
trap 'rm -f "$archive_path"' EXIT HUP INT TERM
curl --fail --location --silent --show-error \
  "https://github.com/nicholas-fedor/shoutrrr/releases/download/v0.19.0/$archive" \
  --output "$archive_path"
printf '%s  %s\n' "$checksum" "$archive_path" | sha256sum -c -
tar -xzf "$archive_path" -C /usr/local/bin shoutrrr
chmod 0755 /usr/local/bin/shoutrrr

# curl is used only to install Shoutrrr; keep the runtime image lean.
apk del --no-cache .fetch-deps

git --version
docker --version
docker compose version
docker buildx version
deno --version
shoutrrr --version
pg_dump --version
restic version
