#!/usr/bin/env bash
set -euo pipefail

sudo chown -R "$(id -u):$(id -g)" \
  /usr/local/cargo/registry \
  /usr/local/cargo/git \
  /home/vscode/.cache/citadel-target \
  /home/vscode/.npm \
  /workspace/src/Citadel.FrontEnd/node_modules

cargo fetch --locked --manifest-path /workspace/rust/Cargo.toml
npm --prefix /workspace/src/Citadel.FrontEnd ci
