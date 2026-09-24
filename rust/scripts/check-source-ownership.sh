#!/usr/bin/env bash
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

# Historical reports are evidence, not build inputs. Scan executable sources,
# manifests and scripts, including untracked files during local development.
if rg --hidden -n -i \
  'src[\\/]+Citadel[.]Contracts|Citadel-P/Citadel[.]Contracts|github[.]com/Citadel-P/Citadel[.]Agent|[.]csproj' \
  build crates xtask scripts Cargo.toml Dockerfile* \
  --glob '!*.md'; then
  echo 'Rust build inputs must be owned by this repository.' >&2
  exit 1
else
  status=$?
  if [[ "$status" != 1 ]]; then
    exit "$status"
  fi
fi
