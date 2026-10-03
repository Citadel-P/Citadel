#!/usr/bin/env bash
set -euo pipefail

export CITADEL_ACCEPTANCE_CORE_IMAGE="${1:?Usage: test-agent-acceptance.sh CORE_IMAGE AGENT_IMAGE [RELEASED_AGENT@DIGEST]}"
export CITADEL_ACCEPTANCE_AGENT_IMAGE="${2:?Usage: test-agent-acceptance.sh CORE_IMAGE AGENT_IMAGE [RELEASED_AGENT@DIGEST]}"
if [[ -n "${3:-}" ]]; then
  [[ "$3" =~ @sha256:[a-f0-9]{64}$ ]] || { echo "The rollback Agent image must be pinned by digest." >&2; exit 1; }
  export CITADEL_ACCEPTANCE_BASELINE_AGENT_IMAGE="$3"
fi
export CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_DEV_INCREMENTAL=false
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.."
cargo test --locked -p citadel-agent --test acceptance -- --ignored --nocapture --test-threads=1
