#!/usr/bin/env bash
# Exercise the distributed installer against uniquely owned Docker volumes.
set -euo pipefail
unset PG_IMAGE PG_DATA PG_USER PG_DATABASE COMPOSE_ENV_FILES
root=$(mktemp -d)
project="citadel-pg-storage-${BASHPID}-$RANDOM"
compose=(docker compose -p "$project" -f "$root/compose.yml")
cp "$(dirname "$0")/../../deploy/install/docker-compose.yml" "$root/compose.yml"
echo PG_PASSWORD=storage-fixture > "$root/.env"
volume="${project}_postgres_data"
# Refuse to acquire or remove any pre-existing fixture.
if docker volume inspect "$volume" >/dev/null 2>&1; then exit 1; fi
cleanup() {
  "${compose[@]}" rm --stop --force pg_db >/dev/null
  docker network rm "${project}_default" >/dev/null 2>&1 || true
  docker volume rm "$volume" >/dev/null
  rm -rf "$root"
}
trap cleanup EXIT
image=$("${compose[@]}" config --images pg_db)
"${compose[@]}" up -d --wait --wait-timeout 90 pg_db
sql() { "${compose[@]}" exec -T pg_db psql -XAt -U citadel -v ON_ERROR_STOP=1 -c "$1"; }
[[ $(sql "SHOW server_version_num") == 190000 ]]
sql "CREATE TABLE retained(marker int); INSERT INTO retained VALUES (37)"
"${compose[@]}" stop pg_db
if PG_DATA=/var/lib/postgresql/18/docker "${compose[@]}" run --rm --no-deps pg_db postgres --version; then exit 1; fi
"${compose[@]}" start --wait pg_db
[[ $(sql "SELECT marker FROM retained") == 37 ]]
"${compose[@]}" rm -sf pg_db
# Replace only this disposable test volume with a retained older cluster marker.
docker volume rm "$volume" >/dev/null
docker volume create "$volume" >/dev/null
inspect=(docker run --rm --entrypoint sh -v "$volume:/var/lib/postgresql" "$image" -ec)
"${inspect[@]}" 'cd /var/lib/postgresql; mkdir -p 18/docker; echo 18 >18/docker/PG_VERSION'
if "${compose[@]}" run --rm --no-deps pg_db postgres --version; then exit 1; fi
"${inspect[@]}" 'cd /var/lib/postgresql; test "$(cat 18/docker/PG_VERSION)" = 18; test ! -e 19; mv 18 unknown'
if "${compose[@]}" run --rm --no-deps pg_db postgres --version; then exit 1; fi
"${inspect[@]}" 'cd /var/lib/postgresql; test "$(cat unknown/docker/PG_VERSION)" = 18; test ! -e 19'
echo 'PG19 initialization, retained data and incompatible-storage rejection passed.'
