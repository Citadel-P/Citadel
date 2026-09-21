#!/bin/sh
# Runtime dependencies for the Debian-based Rust Core image.
set -eu

case "${TARGETARCH:-$(dpkg --print-architecture)}" in
  amd64)
    archive=shoutrrr_linux_amd64_0.19.0.tar.gz
    checksum=e857441d23747486a4404b1fd1e6a82d1fbe8e0397fabddf412955f15c2e3e57
    ;;
  arm64)
    archive=shoutrrr_linux_arm64v8_0.19.0.tar.gz
    checksum=67321d97a7ebd19f6f5d7bbeabb6337256882716eee28e8204828fe314c9e215
    ;;
  *) echo "Unsupported architecture: ${TARGETARCH:-$(dpkg --print-architecture)}" >&2; exit 1 ;;
esac

export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install --yes --no-install-recommends \
  ca-certificates git openssh-client postgresql-client restic curl util-linux

archive_path=$(mktemp)
trap 'rm -f "$archive_path"' EXIT HUP INT TERM
curl --fail --location --silent --show-error \
  "https://github.com/nicholas-fedor/shoutrrr/releases/download/v0.19.0/$archive" \
  --output "$archive_path"
printf '%s  %s\n' "$checksum" "$archive_path" | sha256sum --check --strict
tar -xzf "$archive_path" -C /usr/local/bin shoutrrr
chmod 0755 /usr/local/bin/shoutrrr

# curl is used only to install Shoutrrr; keep the runtime image lean.
apt-get purge --yes curl
apt-get clean
rm -rf /var/lib/apt/lists/*

git --version
docker --version
docker compose version
docker buildx version
deno --version
shoutrrr --version
pg_dump --version
restic version
