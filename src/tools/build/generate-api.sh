#!/usr/bin/env bash
set -euo pipefail

# One generator owns the SDK, OpenAPI documents and frontend models. Serialize
# debugger/UI startup so two npm generators cannot clear each other's outputs.
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.."
mkdir -p .artifacts
exec 9> .artifacts/api-generation.lock
flock 9
export SQLX_OFFLINE=true
cargo run --locked -p xtask -- openapi
