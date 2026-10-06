#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)"
env_file="$repo_root/deploy/.env.development"

configure() (
  # Keep one environment for the task runner and CodeLLDB. Never overwrite edits.
  # Compound debugging may prepare the API and UI concurrently.
  umask 077
  mkdir -p -- "$repo_root/deploy"
  exec 9> "$env_file.lock"
  flock 9
  if [[ -f "$env_file" ]]; then return; fi
  # In a devcontainer, retain its socket, database and runtime volume settings.
  cat > "$env_file" <<EOF
DATABASE_URL=${DATABASE_URL:-postgres://citadel:citadel@127.0.0.1:15432/citadel}
Transport__Mode=${Transport__Mode:-Disabled}
Transport__PublicUrl=${Transport__PublicUrl:-http://localhost:8000}
# Local Docker Desktop only; remote Agents need Core's reachable gRPC hostname.
EdgeAgent__PublicGrpcUrl=${EdgeAgent__PublicGrpcUrl:-http://host.docker.internal:8001}
# Set to 0.0.0.0 to test Agents connecting from another machine.
CITADEL_DEV_EDGE_BIND_ADDRESS=${CITADEL_DEV_EDGE_BIND_ADDRESS:-127.0.0.1}
Cors__0=${Cors__0:-http://localhost:5173}
Cors__1=${Cors__1:-http://127.0.0.1:5173}
Jwt__Key=${Jwt__Key:-citadel-development-jwt-key-at-least-32-bytes}
Secrets__EncryptionKey=${Secrets__EncryptionKey:-AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA=}
CITADEL_RUST_DOCKER_SOCKET=${CITADEL_RUST_DOCKER_SOCKET:-/var/run/docker.sock}
DOCKER_HOST=unix://${CITADEL_RUST_DOCKER_SOCKET:-/var/run/docker.sock}
CITADEL_DATA_ROOT=${CITADEL_DATA_ROOT:-${XDG_DATA_HOME:-$HOME/.local/share}/citadel-wsl}
SQLX_OFFLINE=true
RUST_BACKTRACE=1
RUST_LOG=info
LogFormat=text
VITE_API_BASE_URL=http://localhost:8000
EOF
)

load_environment() {
  if [[ ! -f "$env_file" ]]; then
    echo 'Run the Citadel: Prepare development environment task first.' >&2
    exit 1
  fi
  # Plain KEY=value lines, also understood by CodeLLDB. Do not execute shell code.
  while IFS= read -r line || [[ -n "$line" ]]; do
    line="${line%$'\r'}"
    [[ -z "$line" || "$line" == \#* ]] && continue
    if [[ ! "$line" =~ ^[a-zA-Z_][a-zA-Z_0-9]*= ]]; then
      echo "Invalid environment entry in $env_file (expected KEY=value)." >&2
      exit 1
    fi
    export "$line"
  done < "$env_file"
}

prepare() {
  configure
  load_environment
  for tool in cargo node npm docker; do
    command -v "$tool" >/dev/null || { echo "Install Linux $tool; see docs/DEVELOPMENT.md." >&2; exit 1; }
  done
  [[ "$(node -p 'process.platform')" == linux ]] || {
    echo 'Use Linux Node.js inside WSL, not Node.js from the Windows PATH.' >&2; exit 1;
  }
  [[ -S "$CITADEL_RUST_DOCKER_SOCKET" && -r "$CITADEL_RUST_DOCKER_SOCKET" && -w "$CITADEL_RUST_DOCKER_SOCKET" ]] || {
    echo 'Docker socket is unavailable. Check Docker Desktop WSL integration and socket permissions.' >&2; exit 1;
  }
  [[ "$DOCKER_HOST" == "unix://$CITADEL_RUST_DOCKER_SOCKET" ]] || {
    echo 'DOCKER_HOST and CITADEL_RUST_DOCKER_SOCKET must select the same Docker socket.' >&2; exit 1;
  }
  # Explicitly use the same socket as the API, not a different CLI context.
  local docker_os
  docker_os="$(docker --host "unix://$CITADEL_RUST_DOCKER_SOCKET" info --format '{{.OperatingSystem}}')"
  if [[ ! -f /.dockerenv ]] && grep -qi microsoft /proc/sys/kernel/osrelease; then
    if [[ "$docker_os" != 'Docker Desktop' && "${CITADEL_DEV_ALLOW_NATIVE_DOCKER:-false}" != true ]]; then
      echo 'WSL is connected to a separate Docker Engine, not Docker Desktop.' >&2
      echo 'Enable Desktop WSL integration, or deliberately set CITADEL_DEV_ALLOW_NATIVE_DOCKER=true in deploy/.env.development.' >&2
      exit 1
    fi
    if [[ "$DATABASE_URL" == postgres://citadel:citadel@127.0.0.1:15432/citadel ]]; then
      docker --host "unix://$CITADEL_RUST_DOCKER_SOCKET" compose \
        --project-name citadel-wsl \
        -f "$repo_root/.devcontainer/compose.yaml" \
        -f "$repo_root/.devcontainer/compose.wsl.yaml" up -d --wait --wait-timeout 90 postgres
    fi
  fi
  mkdir -p -m 700 -- "$CITADEL_DATA_ROOT"
  bash "$repo_root/.devcontainer/clean-build-cache.sh"
}

development_compose() {
  docker --host "unix://$CITADEL_RUST_DOCKER_SOCKET" compose \
    --project-name citadel-wsl \
    -f "$repo_root/.devcontainer/compose.yaml" \
    -f "$repo_root/.devcontainer/compose.wsl.yaml" \
    -f "$repo_root/deploy/compose.development.yml" "$@"
}

configure_compose() {
  configure
  load_environment
  if [[ -f /.dockerenv ]]; then
    echo 'Run the Compose task from the WSL checkout. Inside a Dev Container, use Citadel: Debug application or Citadel: Run Rust API.' >&2
    exit 1
  fi
  if [[ "$DATABASE_URL" != postgres://citadel:citadel@127.0.0.1:15432/citadel ]]; then
    echo 'The Compose task uses the citadel-wsl database. For a custom DATABASE_URL, use Citadel: Run Rust API or the debugger.' >&2
    exit 1
  fi
  export CITADEL_DEV_UID="$(id -u)" CITADEL_DEV_GID="$(id -g)"
  export CITADEL_DEV_DOCKER_GID="$(stat -c %g "$CITADEL_RUST_DOCKER_SOCKET")"
  local target_dir
  target_dir="$(cd "$repo_root" && cargo metadata --locked --offline --no-deps --format-version 1 | \
    node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>process.stdout.write(JSON.parse(s).target_directory))')"
  case "${CITADEL_COMPOSE_PROFILE:-release}" in
    release) CITADEL_DEV_BUILD_BINARY="$target_dir/release/citadel-server" ;;
    dev) CITADEL_DEV_BUILD_BINARY="$target_dir/debug/citadel-server" ;;
    *) echo 'CITADEL_COMPOSE_PROFILE must be release or dev.' >&2; exit 2 ;;
  esac
  # Docker may restart Core after `cargo clean`. Keep its executable outside the
  # disposable target directory so a missing bind source cannot become a directory.
  export CITADEL_DEV_BINARY="$CITADEL_DATA_ROOT/dev-bin/citadel-server"
}

compose_up() {
  configure_compose
  prepare
  # Reuse the WSL Cargo cache instead of compiling a second tree in Docker.
  local build_args=()
  if [[ "${CITADEL_COMPOSE_PROFILE:-release}" == release ]]; then build_args+=(--release); fi
  CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_DEV_INCREMENTAL=false bash "$repo_root/src/tools/build/build.sh" "${build_args[@]}"
  mkdir -p -m 700 -- "$(dirname -- "$CITADEL_DEV_BINARY")"
  install -m 755 -- "$CITADEL_DEV_BUILD_BINARY" "$CITADEL_DEV_BINARY.new"
  mv -f -- "$CITADEL_DEV_BINARY.new" "$CITADEL_DEV_BINARY"
  development_compose build core
  development_compose up -d --wait --wait-timeout 90 postgres
  # Recreate Core to remount the newly installed executable,
  # without restarting PostgreSQL or discarding either service's data.
  if ! development_compose up -d --no-deps --force-recreate --wait --wait-timeout 90 core; then
    echo 'Core failed to start. Recent Core logs:' >&2
    development_compose logs --no-color --tail 60 core >&2 || true
    echo 'If the logs report AppliedChecksum, reconcile the existing database as described in docs/DEVELOPMENT.md (Database schema changes).' >&2
    return 1
  fi
  echo 'Core and PostgreSQL are running in Docker Compose project citadel-wsl.'
}

case "${1:-}" in
  configure) configure ;;
  prepare) prepare ;;
  compose-up) compose_up ;;
  compose-stop) configure_compose; development_compose stop core postgres ;;
  exec)
    shift
    load_environment
    python3 "$repo_root/src/tools/build/version.py" install
    exec python3 "$repo_root/src/tools/build/version.py" exec -- "$@"
    ;;
  *) echo 'Usage: bash src/tools/dev/dev.sh {configure|prepare|compose-up|compose-stop|exec COMMAND...}' >&2; exit 2 ;;
esac
