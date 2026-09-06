#!/usr/bin/env bash
set -euo pipefail

# Run in the devcontainer. Mock every command that measures/cleans the cache;
# never build, create a large fixture, or delete actual compiler artifacts.
[[ -d /home/vscode/.cache/citadel-target ]] || { printf 'Run this test in the Citadel devcontainer.\n' >&2; exit 1; }
cleanup_script="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/clean-build-cache.sh"
cargo() { printf 'MOCK_CARGO: %s\n' "$*"; }
du() { printf '%s\t/cache\n' "$TEST_CACHE_KIB"; }
df() { printf 'Filesystem 1024-blocks Used Available Capacity Mounted\nfixture 20000000 0 %s 0%% /cache\n' "$TEST_FREE_KIB"; }
pgrep() { [[ "$TEST_ACTIVE_BUILD" == 1 ]]; }
readlink() { printf '%s\n' "$TEST_REAL_PATH"; }
export -f cargo du df pgrep readlink
export CARGO_TARGET_DIR=/home/vscode/.cache/citadel-target
export TEST_REAL_PATH="$CARGO_TARGET_DIR" TEST_ACTIVE_BUILD=0
export TEST_CACHE_KIB=1024 TEST_FREE_KIB=20000000

assert_no_clean() {
  local result
  result=$(bash "$cleanup_script")
  [[ "$result" != *MOCK_CARGO* ]] || { printf 'Unexpected cleanup: %s\n' "$result" >&2; exit 1; }
}
assert_clean() {
  local result
  result=$(bash "$cleanup_script")
  [[ "$result" == *'--profile dev'* && "$result" == *'--profile release'* && "$result" == *'--doc'* ]]
  [[ "$result" != *'--target-dir'* ]]
  [[ $(printf '%s\n' "$result" | grep -c MOCK_CARGO) == 3 ]]
}

assert_no_clean
TEST_CACHE_KIB=$((8 * 1024 * 1024))
assert_clean
TEST_ACTIVE_BUILD=1
assert_no_clean
TEST_ACTIVE_BUILD=0 TEST_CACHE_KIB=1024 TEST_FREE_KIB=1024
assert_clean
CARGO_TARGET_DIR=/workspace
assert_no_clean
CARGO_TARGET_DIR=/home/vscode/.cache/citadel-target TEST_REAL_PATH=/unexpected
if bash "$cleanup_script" >/dev/null 2>&1; then
  printf 'Symlink target was not rejected.\n' >&2
  exit 1
fi
printf 'Devcontainer build-cache tests passed.\n'
