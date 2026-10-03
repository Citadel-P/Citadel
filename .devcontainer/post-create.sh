#!/usr/bin/env bash
set -euo pipefail

runtime_data_root="${CITADEL_DATA_ROOT:?CITADEL_DATA_ROOT must be configured by the development container}"
sudo install -d -m 0700 -o "$(id -u)" -g "$(id -g)" "$runtime_data_root"
if [[ ! -w "$runtime_data_root" || ! -x "$runtime_data_root" ]]; then
  printf 'Citadel data directory is not writable: %s\n' "$runtime_data_root" >&2
  exit 1
fi

sudo chown -R "$(id -u):$(id -g)" \
  /usr/local/cargo/registry \
  /usr/local/cargo/git \
  /home/vscode/.cache/citadel-target \
  /home/vscode/.npm \
  /workspace/src/frontend/node_modules

cargo fetch --locked --manifest-path /workspace/Cargo.toml
npm --prefix /workspace/src/frontend ci
