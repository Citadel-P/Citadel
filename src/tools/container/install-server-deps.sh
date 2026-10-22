#!/bin/sh
# Runtime dependencies for the Alpine-based Core image.
set -eu

case "${TARGETARCH:-$(uname -m)}" in
  amd64|x86_64)
    archive=shoutrrr_linux_amd64_0.22.1.tar.gz
    checksum=c6621f7a34b631705e5e94d33056d6368901495835ad07166034b644a1d46d86
    ;;
  arm64|aarch64)
    archive=shoutrrr_linux_arm64v8_0.22.1.tar.gz
    checksum=f5198eacbc481552e169bb5d67cc9652d19c5494f06720832b0d85fc5eb368b4
    ;;
  *) echo "Unsupported architecture: ${TARGETARCH:-$(uname -m)}" >&2; exit 1 ;;
esac

# Apply fixes to packages inherited from the pinned Deno image too.
apk upgrade --no-cache
apk add --no-cache \
  bash ca-certificates git openssh-client postgresql18-client restic setpriv tini
apk add --no-cache --virtual .fetch-deps curl

archive_path=$(mktemp)
trap 'rm -f "$archive_path"' EXIT HUP INT TERM
curl --fail --location --silent --show-error \
  "https://github.com/nicholas-fedor/shoutrrr/releases/download/v0.22.1/$archive" \
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
