#!/usr/bin/env bash
set -euo pipefail

# Run from the Rust workspace so its toolchain and Cargo settings apply.
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
export SQLX_OFFLINE=true

cargo build --locked -p citadel-server --bins "$@"
cargo run --locked -p xtask -- openapi
