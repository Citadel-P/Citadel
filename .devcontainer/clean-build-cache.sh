#!/usr/bin/env bash
set -euo pipefail

# Only the dedicated compiler-output volume is eligible, never runtime data.
cache_dir=/home/vscode/.cache/citadel-target
if [[ "${CARGO_TARGET_DIR:-}" != "$cache_dir" ]]; then
  printf 'Skipping build-cache cleanup: not the Citadel development target.\n'
  exit 0
fi
[[ -d "$cache_dir" ]] || exit 0
if [[ "$(readlink -f -- "$cache_dir")" != "$cache_dir" ]]; then
  printf 'Refusing build-cache cleanup through a symlink.\n' >&2
  exit 1
fi

# Keep ordinary incremental builds fast. This is a pre-build threshold, not a
# filesystem quota: a single build can exceed it until the next invocation.
cache_kib=$(du -sk -- "$cache_dir" | cut -f1)
available_kib=$(df -Pk -- "$cache_dir" | awk 'END {print $4}')
if (( cache_kib < 8 * 1024 * 1024 && available_kib >= 5 * 1024 * 1024 )); then
  exit 0
fi
if pgrep -x cargo >/dev/null || pgrep -x rustc >/dev/null; then
  printf 'Build-cache cleanup deferred: a Rust build is running.\n'
  exit 0
fi
printf 'Clearing Citadel compiler output (%s KiB); source and runtime data are unchanged.\n' "$cache_kib"
# Preserve the volume mountpoint: a full clean tries to remove that directory.
# Test output shares dev's debug directory. Use the validated configured target;
# --target-dir rejects older Docker-created caches without CACHEDIR.TAG.
for profile in dev release; do
  CARGO_TARGET_DIR="$cache_dir" cargo clean --manifest-path /workspace/Cargo.toml --profile "$profile"
done
CARGO_TARGET_DIR="$cache_dir" cargo clean --manifest-path /workspace/Cargo.toml --doc
