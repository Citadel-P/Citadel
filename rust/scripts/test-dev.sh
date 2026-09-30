#!/usr/bin/env bash
set -euo pipefail

# Isolated fixtures and mock tools only; never start containers or run Cargo.
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fixture="$(mktemp -d)"
trap 'rm -rf -- "$fixture"' EXIT
mkdir -p "$fixture/rust/scripts" "$fixture/.devcontainer"
cp "$script_dir/dev.sh" "$fixture/rust/scripts/dev.sh"
printf '#!/usr/bin/env bash\nexit 0\n' > "$fixture/.devcontainer/clean-build-cache.sh"
dev="$fixture/rust/scripts/dev.sh"
env_file="$fixture/rust/.env.development"
unset DATABASE_URL CITADEL_RUST_DOCKER_SOCKET CITADEL_DATA_ROOT Secrets__EncryptionKey
export XDG_DATA_HOME="$fixture/data with spaces"
bash "$dev" configure
[[ "$(stat -c %a "$env_file")" == 600 ]]
[[ "$(bash "$dev" exec printenv CITADEL_DATA_ROOT)" == "$XDG_DATA_HOME/citadel-wsl" ]]
[[ "$(bash "$dev" exec printenv DATABASE_URL)" == postgres://citadel:citadel@127.0.0.1:15432/citadel ]]
[[ "$(bash "$dev" exec printenv Secrets__EncryptionKey)" == AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA= ]]
printf 'CUSTOM_VALUE=literal $(not-a-command)\r\n' >> "$env_file"
bash "$dev" configure
[[ "$(bash "$dev" exec printenv CUSTOM_VALUE)" == 'literal $(not-a-command)' ]]
printf 'invalid entry\n' >> "$env_file"
if bash "$dev" exec true >/dev/null 2>&1; then
  echo 'Invalid environment entry was accepted.' >&2; exit 1
fi
rm -- "$env_file"

# Inherited devcontainer settings must survive configuration generation.
DATABASE_URL=postgres://citadel:citadel@postgres:5432/citadel \
  CITADEL_RUST_DOCKER_SOCKET=/var/run/docker-host.sock \
  CITADEL_DATA_ROOT=/home/vscode/.local/share/citadel bash "$dev" configure
[[ "$(bash "$dev" exec printenv DATABASE_URL)" == postgres://citadel:citadel@postgres:5432/citadel ]]
[[ "$(bash "$dev" exec printenv CITADEL_RUST_DOCKER_SOCKET)" == /var/run/docker-host.sock ]]
[[ "$(bash "$dev" exec printenv DOCKER_HOST)" == unix:///var/run/docker-host.sock ]]
[[ "$(bash "$dev" exec printenv CITADEL_DATA_ROOT)" == /home/vscode/.local/share/citadel ]]

cargo() { echo 'Unexpected Cargo invocation' >&2; return 1; }
npm() { echo 'Unexpected npm invocation' >&2; return 1; }
node() { echo win32; }
docker() { echo 'Unexpected Docker invocation' >&2; return 1; }
export -f cargo npm node docker
if output="$(bash "$dev" prepare 2>&1)"; then exit 1; fi
[[ "$output" == *'Use Linux Node.js'* ]]

node() { echo linux; }
export -f node
printf 'CITADEL_RUST_DOCKER_SOCKET=%s/missing.sock\n' "$fixture" >> "$env_file"
if output="$(bash "$dev" prepare 2>&1)"; then exit 1; fi
[[ "$output" == *'Docker socket is unavailable'* ]]

# Exercise the WSL daemon guard on WSL with its existing socket. The Docker
# command is mocked, so no real daemon request or database mutation occurs.
if [[ ! -f /.dockerenv && -S /var/run/docker.sock ]] && grep -qi microsoft /proc/sys/kernel/osrelease; then
  export DEV_TEST_DOCKER_LOG="$fixture/docker.log"
  docker() {
    if [[ "$*" == *' info '* ]]; then echo Ubuntu; else printf '%s\n' "$*" >> "$DEV_TEST_DOCKER_LOG"; fi
  }
  export -f docker
  printf 'CITADEL_RUST_DOCKER_SOCKET=/var/run/docker.sock\nDOCKER_HOST=unix:///var/run/docker.sock\n' >> "$env_file"
  if output="$(bash "$dev" prepare 2>&1)"; then exit 1; fi
  [[ "$output" == *'separate Docker Engine'* && ! -f "$DEV_TEST_DOCKER_LOG" ]]
  printf 'CITADEL_DEV_ALLOW_NATIVE_DOCKER=true\nCITADEL_DATA_ROOT=%s/runtime\nDATABASE_URL=postgres://citadel:citadel@127.0.0.1:15432/citadel\n' "$fixture" >> "$env_file"
  bash "$dev" prepare
  [[ "$(cat "$DEV_TEST_DOCKER_LOG")" == *'--project-name citadel-wsl'*'up -d --wait --wait-timeout 90 postgres' ]]
  [[ "$(stat -c %a "$fixture/runtime")" == 700 ]]
  : > "$DEV_TEST_DOCKER_LOG"
  printf 'DATABASE_URL=postgres://custom/database\n' >> "$env_file"
  bash "$dev" prepare
  [[ ! -s "$DEV_TEST_DOCKER_LOG" ]]
fi

# Compose must mount a staged executable that survives Cargo cache cleanup.
if [[ ! -f /.dockerenv && -S /var/run/docker.sock ]]; then
  export DEV_TEST_TARGET_DIR="$fixture/cargo target"
  export DEV_TEST_DOCKER_LOG="$fixture/docker.log"
  printf 'DATABASE_URL=postgres://citadel:citadel@127.0.0.1:15432/citadel\nCITADEL_RUST_DOCKER_SOCKET=/var/run/docker.sock\nDOCKER_HOST=unix:///var/run/docker.sock\nCITADEL_DATA_ROOT=%s/runtime\n' "$fixture" > "$env_file"
  cargo() { printf '{"target_directory":"%s"}\n' "$DEV_TEST_TARGET_DIR"; }
  node() {
    if [[ "${1:-}" == -p ]]; then echo linux; else command node "$@"; fi
  }
  docker() {
    if [[ "$*" == *' info '* ]]; then echo 'Docker Desktop'; return; fi
    if [[ "$*" == *' build core' ]]; then
      [[ -f "$CITADEL_DEV_BINARY" && -x "$CITADEL_DEV_BINARY" ]] || return 1
      [[ "$CITADEL_DEV_BINARY" != "$DEV_TEST_TARGET_DIR"/* ]] || return 1
    fi
    printf '%s\n' "$*" >> "$DEV_TEST_DOCKER_LOG"
  }
  export -f cargo node docker
  cat > "$fixture/rust/scripts/build.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "${DEV_TEST_FAIL_BUILD:-false}" != true ]] || exit 1
profile=debug
[[ "${1:-}" != --release ]] || profile=release
mkdir -p -- "$DEV_TEST_TARGET_DIR/$profile"
printf '#!/bin/sh\necho %s\n' "$profile" > "$DEV_TEST_TARGET_DIR/$profile/citadel-server"
EOF
  bash "$dev" compose-up
  staged="$fixture/runtime/dev-bin/citadel-server"
  [[ "$(bash "$staged")" == release ]]
  rm -rf -- "$DEV_TEST_TARGET_DIR"
  [[ -x "$staged" && "$(bash "$staged")" == release ]]
  CITADEL_COMPOSE_PROFILE=dev bash "$dev" compose-up
  [[ "$(bash "$staged")" == debug ]]
  : > "$DEV_TEST_DOCKER_LOG"
  if DEV_TEST_FAIL_BUILD=true bash "$dev" compose-up; then
    echo 'Compose continued after a failed build.' >&2; exit 1
  fi
  [[ "$(bash "$staged")" == debug ]]
  [[ "$(cat "$DEV_TEST_DOCKER_LOG")" != *'--force-recreate'* ]]
fi
echo 'Development environment tests passed.'
