#!/usr/bin/env bash
set -euo pipefail

export CITADEL_ACCEPTANCE_CORE_IMAGE="${1:?Usage: test-agent-acceptance.sh CORE_IMAGE AGENT_IMAGE}"
export CITADEL_ACCEPTANCE_AGENT_IMAGE="${2:?Usage: test-agent-acceptance.sh CORE_IMAGE AGENT_IMAGE}"
export CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_DEV_INCREMENTAL=false
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.."
cargo test --locked -p citadel-agent --test acceptance -- --ignored --nocapture --test-threads=1
