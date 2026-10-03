#!/usr/bin/env sh
set -eu

docker_version="29.0.0"
compose_version="v2.40.3"
tools_dir="$(mktemp -d)"
trap 'rm -rf "$tools_dir"' EXIT

curl --fail --silent --show-error --location \
  "https://download.docker.com/linux/static/stable/x86_64/docker-${docker_version}.tgz" \
  | tar --extract --gzip --directory "$tools_dir"
mkdir -p "$tools_dir/config/cli-plugins"
curl --fail --silent --show-error --location \
  "https://github.com/docker/compose/releases/download/${compose_version}/docker-compose-linux-x86_64" \
  --output "$tools_dir/config/cli-plugins/docker-compose"
chmod +x "$tools_dir/config/cli-plugins/docker-compose"

export PATH="$tools_dir/docker:$PATH"
export DOCKER_CONFIG="$tools_dir/config"
docker version >/dev/null
docker compose version >/dev/null

cargo test --locked -p citadel-adapters --test stack_runtime_local \
  local_compose_stack_apply_creates_owned_runtime_and_cleans_it \
  -- --ignored --test-threads=1

if [ "${CITADEL_RUN_SWARM_LIFECYCLE:-false}" = "true" ]; then
  swarm_state="$(docker info --format '{{.Swarm.LocalNodeState}}')"
  if [ "$swarm_state" != "active" ]; then
    echo "CITADEL_RUN_SWARM_LIFECYCLE=true requires an already active Swarm manager." >&2
    exit 1
  fi
  cargo test --locked -p citadel-adapters --test stack_runtime_local \
    local_swarm_stack_apply_and_delete_use_the_native_stack_lifecycle \
    -- --ignored --test-threads=1
fi
