#!/usr/bin/env bash
set -euo pipefail

# Run from the Rust workspace so its toolchain and Cargo settings apply.
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.."
export SQLX_OFFLINE=true

if [[ -z "${CITADEL_VERSION:-}" && -z "${CITADEL_INFORMATIONAL_VERSION:-}" ]]; then
  python3 src/tools/build/version.py install
  exec python3 src/tools/build/version.py exec -- bash src/tools/build/build.sh "$@"
fi

cargo build --locked -p citadel-server --bins "$@"
cargo run --locked -p xtask -- openapi
