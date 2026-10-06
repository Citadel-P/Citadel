#!/usr/bin/env bash
set -euo pipefail
repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)
python3 "$repo_root/src/tools/build/version.py" install
exec python3 "$repo_root/src/tools/build/version.py" exec -- "$@"
