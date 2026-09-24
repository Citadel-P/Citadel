#!/usr/bin/env bash
set -euo pipefail

# The test runner reaches only this disposable daemon, never the host Docker socket.
image="${1:?Usage: test-agent-compatibility.sh AGENT_IMAGE [TEST_EXECUTABLE]}"
binary="${2:-}"
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.."
if [[ -z "$binary" ]]; then
  binary=$(cd rust && CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_DEV_INCREMENTAL=false \
    cargo test --locked -p citadel-agent --test compatibility --test live_docker --test edge_intake --no-run --message-format=json |
    python3 -c 'import json,sys; rows=[json.loads(line) for line in sys.stdin]; print("\n".join(r["executable"] for r in rows if r.get("reason")=="compiler-artifact" and r.get("target",{}).get("name") in ("compatibility", "live_docker", "edge_intake") and r.get("executable")))')
fi
mapfile -t binaries <<< "$binary"
fixture="citadel-agent-compat-$$-$RANDOM"
directory=$(mktemp -d)
dind=docker@sha256:3f3c01aaaebf7cce837356b688b7c059a4749f10bd7660dec7c58fc454a283f0
registry=registry@sha256:325b4b29b041e82803abeb703e201655e4e23ab83264ec1a7c9ddb0a5b14a6e0
postgres=postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44

cleanup() {
  result=$?
  trap - EXIT
  for role in runner registry postgres worker daemon; do
    if (( result != 0 )); then docker logs "$fixture-$role" 2>/dev/null || true; fi
    docker rm -fv "$fixture-$role" >/dev/null 2>&1 || true
  done
  docker network rm "$fixture" >/dev/null 2>&1 || true
  rm -rf "$directory"
  exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# Public fixture credential, used only by the isolated registry.
# shellcheck disable=SC2016
printf '%s\n' 'fixture:$2b$12$gV/Jq68qxL253/dpxBDXe.jaFRerpaU2P9p5BaxSgIhu3sv/.QdEu' > "$directory/htpasswd"
docker network create "$fixture" >/dev/null
docker run -d --name "$fixture-registry" --network "$fixture" --network-alias registry \
  --mount "type=bind,src=$directory,dst=/auth,readonly" \
  -e REGISTRY_AUTH=htpasswd -e REGISTRY_AUTH_HTPASSWD_REALM=fixture \
  -e REGISTRY_AUTH_HTPASSWD_PATH=/auth/htpasswd "$registry" >/dev/null
docker run -d --name "$fixture-postgres" --network "$fixture" --network-alias postgres \
  --tmpfs /var/lib/postgresql -e POSTGRES_USER=citadel_test -e POSTGRES_PASSWORD=fixture \
  -e POSTGRES_DB=citadel_agent_test "$postgres" >/dev/null
docker run -d --privileged --name "$fixture-daemon" --hostname "$fixture-daemon" \
  --network "$fixture" --network-alias daemon -e DOCKER_TLS_CERTDIR= "$dind" \
  --tls=false --host=tcp://0.0.0.0:2375 --host=unix:///var/run/docker.sock \
  --insecure-registry=registry:5000 >/dev/null
docker run -d --privileged --name "$fixture-worker" --hostname "$fixture-worker" \
  --network "$fixture" --network-alias worker -e DOCKER_TLS_CERTDIR= "$dind" \
  --tls=false --host=tcp://0.0.0.0:2375 --host=unix:///var/run/docker.sock \
  --insecure-registry=registry:5000 >/dev/null
for _ in {1..60}; do
  if docker exec "$fixture-daemon" docker info >/dev/null 2>&1 && \
    docker exec "$fixture-worker" docker info >/dev/null 2>&1 && \
    docker exec "$fixture-postgres" pg_isready -U citadel_test >/dev/null 2>&1; then break; fi
  sleep 1
done
docker exec "$fixture-daemon" docker info >/dev/null
docker exec "$fixture-postgres" pg_isready -U citadel_test >/dev/null
docker exec "$fixture-daemon" docker swarm init >/dev/null
token=$(docker exec "$fixture-daemon" docker swarm join-token -q worker)
docker exec "$fixture-worker" docker swarm join --token "$token" daemon:2377 >/dev/null
docker exec "$fixture-daemon" docker pull alpine:3.24 >/dev/null
docker exec "$fixture-daemon" docker pull redis:latest >/dev/null
docker exec "$fixture-daemon" docker tag alpine:3.24 compatibility-base:local
docker exec "$fixture-daemon" docker save compatibility-base:local | \
  docker exec -i "$fixture-worker" docker load >/dev/null

for binary in "${binaries[@]}"; do
  binary=$(realpath "$binary")
  timeout --signal=TERM 10m docker run --name "$fixture-runner" --network "$fixture" --no-healthcheck \
    --mount "type=bind,src=$binary,dst=/app/compatibility-tests,readonly" \
    --tmpfs /app/data --env DOCKER_HOST=tcp://daemon:2375 \
    --env CITADEL_COMPAT_WORKER_HOST=tcp://worker:2375 \
    --env CITADEL_AGENT_TEST_DATABASE_URL=postgres://citadel_test:fixture@postgres:5432/citadel_agent_test \
    --entrypoint /app/compatibility-tests "$image" --ignored --nocapture --test-threads=1
  docker rm -fv "$fixture-runner" >/dev/null
done
