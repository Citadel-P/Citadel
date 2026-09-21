#!/usr/bin/env bash
set -euo pipefail

# Exercise the packaged binaries against disposable services and data.
image="${1:-citadel-rust:local}"
postgres_image=postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44
fixture="citadel-runtime-test-$$-$RANDOM"
core="$fixture-core"
database="$fixture-db"
build_image="$fixture:build"
socket="${CITADEL_RUST_DOCKER_SOCKET:-/var/run/docker.sock}"

cleanup() {
  result=$?
  trap - EXIT
  if (( result != 0 )); then
    docker logs "$core" 2>/dev/null || true
    docker logs "$database" 2>/dev/null || true
  fi
  for container in "$core" "$database"; do
    if docker container inspect "$container" >/dev/null 2>&1; then
      docker stop --time 5 "$container" >/dev/null || true
      docker container rm --volumes "$container" >/dev/null || true
    fi
  done
  if docker image inspect "$build_image" >/dev/null 2>&1; then
    docker image rm "$build_image" >/dev/null || true
  fi
  docker network rm "$fixture" >/dev/null 2>&1 || true
  exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

docker network create --internal "$fixture" >/dev/null
docker run -d --name "$database" --network "$fixture" --network-alias postgres \
  --tmpfs /var/lib/postgresql \
  -e POSTGRES_USER=citadel -e POSTGRES_DB=citadel \
  -e POSTGRES_PASSWORD=runtime-test-only "$postgres_image" >/dev/null
for attempt in {1..60}; do
  if docker exec "$database" pg_isready -h 127.0.0.1 -U citadel -d citadel >/dev/null 2>&1; then break; fi
  sleep 1
done
docker exec "$database" pg_isready -h 127.0.0.1 -U citadel -d citadel >/dev/null

docker run -d --name "$core" --network "$fixture" \
  -e DATABASE_URL=postgres://citadel:runtime-test-only@postgres:5432/citadel \
  -e Transport__Mode=Disabled -e Transport__PublicUrl=http://localhost:8000 \
  "$image" >/dev/null
for attempt in {1..60}; do
  if docker exec "$core" /usr/local/bin/citadel-entrypoint healthcheck >/dev/null 2>&1; then break; fi
  sleep 1
done
docker exec "$core" /usr/local/bin/citadel-entrypoint healthcheck
docker exec "$core" sh -c 'grep -Eq "^Uid:[[:space:]]+65532[[:space:]]" /proc/1/status'
docker exec --user 65532:0 -i "$core" sh -s <<'SH'
set -eu
wget -qO /tmp/index.html http://127.0.0.1:8000/
cmp /tmp/index.html /app/wwwroot/index.html
export PGPASSWORD=runtime-test-only PGHOST=postgres PGUSER=citadel
pg_dump --format=custom --no-owner --no-privileges -f /tmp/core.dump citadel
createdb restored
pg_restore --exit-on-error --single-transaction --no-owner --no-privileges --dbname=restored /tmp/core.dump
tables=$(psql -d restored -Atc "SELECT count(*) FROM information_schema.tables WHERE table_schema='public'")
[ "$tables" -gt 0 ]
echo 'Core startup, non-root process, health check, frontend, PostgreSQL dump/restore passed.'
SH

docker run --rm -i --user 65532:0 --entrypoint sh "$image" -s <<'SH'
set -eu
GIT_TERMINAL_PROMPT=0 git ls-remote https://github.com/docker-library/hello-world.git HEAD >/dev/null
ssh -V
mkdir /tmp/runtime-fixture
cat > /tmp/runtime-fixture/action.ts <<'TS'
const message: string = 'runtime fixture';
await Deno.writeTextFile('/tmp/runtime-fixture/payload', message);
if (await Deno.readTextFile('/tmp/runtime-fixture/payload') !== message) throw new Error('File round trip failed');
try {
  await Deno.readTextFile('/etc/os-release');
  throw new Error('Read escaped the automation sandbox');
} catch (error) {
  if (!(error instanceof Deno.errors.NotCapable)) throw error;
}
TS
deno run --no-prompt --allow-read=/tmp/runtime-fixture --allow-write=/tmp/runtime-fixture \
  --allow-env=NO_COLOR,DENO_DIR /tmp/runtime-fixture/action.ts
export RESTIC_REPOSITORY=/tmp/restic-repository RESTIC_PASSWORD=runtime-test-only
restic init >/dev/null
restic backup --json /tmp/runtime-fixture/payload >/dev/null
restic restore latest --target /tmp/restored >/dev/null
cmp /tmp/runtime-fixture/payload /tmp/restored/tmp/runtime-fixture/payload
shoutrrr --version
echo 'Git HTTPS, SSH, Deno TypeScript sandbox and Restic backup/restore passed.'
SH

docker run --rm -i --user 65532:0 --group-add "$(stat -c %g "$socket")" \
  -v "$socket:/var/run/docker.sock" -e AUDIT_IMAGE="$build_image" \
  --entrypoint sh "$image" -s <<'SH'
set -eu
docker info --format '{{.ServerVersion}}' >/dev/null
printf 'services:\n  fixture:\n    image: scratch\n' | docker compose -p runtime-fixture -f - config --quiet
mkdir /tmp/build-fixture
printf 'runtime fixture\n' > /tmp/build-fixture/payload
printf 'FROM scratch\nCOPY payload /payload\n' > /tmp/build-fixture/Dockerfile
export DOCKER_BUILDKIT=1 AUDIT_SECRET=fixture-only
docker build --secret id=fixture,env=AUDIT_SECRET --tag "$AUDIT_IMAGE" /tmp/build-fixture
echo 'Non-root Docker access, Compose and BuildKit build passed.'
SH
