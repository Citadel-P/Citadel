#!/usr/bin/env bash
set -euo pipefail

image="${1:?Usage: test-agent-image.sh IMAGE [EXPECTED_VERSION]}"
expected_version="${2:-}"
fixture="citadel-agent-image-test-$$-$RANDOM"
socket="${CITADEL_AGENT_TEST_DOCKER_SOCKET:-/var/run/docker.sock}"
tls_dir=$(mktemp -d)
containers=()

cleanup() {
  result=$?
  trap - EXIT
  for container in "${containers[@]}"; do
    if (( result != 0 )); then
      docker logs "$container" 2>/dev/null || true
      docker inspect --format '{{json .State.Health}}' "$container" 2>/dev/null || true
    fi
    docker rm -fv "$container" >/dev/null 2>&1 || true
  done
  docker image rm "$fixture:build" >/dev/null 2>&1 || true
  rm -rf "$tls_dir"
  exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

version=$(docker run --rm "$image" --version)
if [[ -n "$expected_version" ]]; then
  [[ "$version" == "citadel-agent $expected_version" ]]
fi
printf '%s\n' "$version"

# Generate a public key and TLS certificate for these disposable listeners only.
openssl genpkey -algorithm ED25519 -out "$tls_dir/hub.key" 2>/dev/null
hub_key=$(openssl pkey -in "$tls_dir/hub.key" -pubout -outform DER | tail -c 32 | base64 -w0)
openssl req -x509 -newkey rsa:2048 -nodes -days 1 -subj /CN=agent.example.test \
  -addext subjectAltName=DNS:agent.example.test \
  -addext basicConstraints=critical,CA:FALSE \
  -keyout "$tls_dir/key.pem" -out "$tls_dir/cert.pem" >/dev/null 2>&1
printf 'disposable-image-test' > "$tls_dir/bootstrap"

for profile in direct tls edge edge-build-agent swarm-node; do
  name="$fixture-$profile"
  containers+=("$name")
  args=(--env CITADEL_AGENT_PORT=9107 --env "HUB_PUBLIC_KEY=$hub_key")
  case "$profile" in
    tls)
      args+=(--mount "type=bind,src=$tls_dir,dst=/tls,readonly"
        --env CITADEL_AGENT_TLS_MODE=Direct
        --env CITADEL_AGENT_TLS_CERTIFICATE_PATH=/tls/cert.pem
        --env CITADEL_AGENT_TLS_PRIVATE_KEY_PATH=/tls/key.pem)
      ;;
    edge|edge-build-agent|swarm-node)
      args+=(--env CITADEL_AGENT_MODE=edge --env CITADEL_CORE_URL=http://127.0.0.1:1
        --env CITADEL_EDGE_ENROLLMENT_TOKEN=disposable-image-test)
      if [[ "$profile" != edge ]]; then args+=(--env "CITADEL_EDGE_AGENT_PROFILE=$profile"); fi
      if [[ "$profile" == swarm-node ]]; then
        args+=(--mount "type=bind,src=$tls_dir,dst=/tls,readonly"
          --env CITADEL_EDGE_BOOTSTRAP_FILE=/tls/bootstrap
          --env CITADEL_PLATFORM_ID=00000000-0000-7000-8000-000000000001
          --env CITADEL_SWARM_SERVICE_ID=fixture-service --env CITADEL_SWARM_TASK_ID=fixture-task
          --env CITADEL_SWARM_NODE_ID=fixture-node --env CITADEL_SWARM_NODE_HOSTNAME=fixture-host
          --env CITADEL_SWARM_CLUSTER_ID=fixture-cluster)
      fi
      ;;
  esac
  docker run -d --name "$name" --network none --tmpfs /app/data \
    --health-interval 1s --health-start-period 1s "${args[@]}" "$image" >/dev/null
  for _ in {1..30}; do
    if [[ "$(docker inspect --format '{{.State.Health.Status}}' "$name")" == healthy ]]; then break; fi
    [[ "$(docker inspect --format '{{.State.Running}}' "$name")" == true ]]
    sleep 1
  done
  [[ "$(docker inspect --format '{{.State.Health.Status}}' "$name")" == healthy ]]
  docker stop --time 12 "$name" >/dev/null
  [[ "$(docker inspect --format '{{.State.ExitCode}}' "$name")" == 0 ]]
done

name="$fixture-tools"
containers+=("$name")
docker run --name "$name" -i --entrypoint sh \
  --mount "type=bind,src=$socket,dst=/var/run/docker.sock" \
  --env "TEST_BUILD_IMAGE=$fixture:build" "$image" -s <<'SH'
set -eu
git --version
git init -q /tmp/git-library
printf 'git submodule fixture\n' > /tmp/git-library/payload
git -C /tmp/git-library add payload
git -C /tmp/git-library -c user.name=fixture -c user.email=fixture@example.test commit -qm fixture
git init -q /tmp/git-source
git -C /tmp/git-source -c protocol.file.allow=always submodule add -q /tmp/git-library library
git -C /tmp/git-source -c user.name=fixture -c user.email=fixture@example.test commit -qm fixture
git clone -q /tmp/git-source /tmp/git-checkout
git -C /tmp/git-checkout -c protocol.file.allow=always submodule update --init --recursive
cmp /tmp/git-library/payload /tmp/git-checkout/library/payload
ssh -V
bash --version >/dev/null
test -s /etc/ssl/certs/ca-certificates.crt
test "$(readlink /app/Citadel.Agent.VolumeHelper)" = /usr/local/bin/citadel-volume-helper
for program in deno pg_dump shoutrrr; do
  if command -v "$program" >/dev/null; then
    echo "Unexpected Core runtime tool: $program" >&2
    exit 1
  fi
done
docker version
docker compose version
docker buildx version
printf 'services:\n  fixture:\n    image: scratch\n' | docker compose -p agent-test -f - config --quiet
mkdir /tmp/build-fixture
printf 'agent runtime fixture\n' > /tmp/build-fixture/payload
printf 'FROM scratch\nCOPY payload /payload\n' > /tmp/build-fixture/Dockerfile
DOCKER_BUILDKIT=1 docker build --tag "$TEST_BUILD_IMAGE" /tmp/build-fixture
citadel-volume-helper volume-helper list --root /tmp/build-fixture --path / \
  --max-entries 1000 --max-payload-bytes 1048576 | grep -q payload
/app/Citadel.Agent.VolumeHelper volume-helper stream-file --root /tmp/build-fixture --path /payload > /tmp/download
cmp /tmp/download /tmp/build-fixture/payload
export RESTIC_REPOSITORY=/tmp/restic-repository RESTIC_PASSWORD=disposable-image-test
restic init >/dev/null
restic backup /tmp/build-fixture/payload >/dev/null
restic restore latest --target /tmp/restored >/dev/null
cmp /tmp/build-fixture/payload /tmp/restored/tmp/build-fixture/payload
SH

echo 'Agent version, HTTP/TLS/Edge health, shutdown, Git submodules, Docker build, Compose, volume helper and Restic smoke checks passed.'
