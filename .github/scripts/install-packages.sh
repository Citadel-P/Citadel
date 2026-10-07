#!/usr/bin/env bash
set -euo pipefail

if (( $# == 0 )); then
  echo "Usage: $0 PACKAGE [PACKAGE ...]" >&2
  exit 2
fi

missing=()
for package in "$@"; do
  if [[ "$(dpkg-query -W -f='${Status}' "$package" 2>/dev/null || true)" != "install ok installed" ]]; then
    missing+=("$package")
  fi
done

if (( ${#missing[@]} == 0 )); then
  echo "All requested packages are already installed; skipping APT."
  exit 0
fi

# A stalled mirror must not consume the entire build's timeout. Run timeout
# under sudo so it can terminate APT and its privileged child processes.
run_apt() {
  if sudo -n timeout --kill-after=10s 180s env DEBIAN_FRONTEND=noninteractive apt-get \
    -o Acquire::Retries=2 \
    -o Acquire::http::Timeout=20 \
    -o Acquire::https::Timeout=20 \
    -o Acquire::Languages=none \
    -o DPkg::Lock::Timeout=30 "$@"; then
    return 0
  else
    status=$?
    echo "APT $1 failed (exit $status; each operation is limited to 180 seconds)." >&2
    return "$status"
  fi
}

echo "Installing missing packages: ${missing[*]}"
run_apt update -o APT::Update::Error-Mode=any
run_apt install -y --no-install-recommends "${missing[@]}"
