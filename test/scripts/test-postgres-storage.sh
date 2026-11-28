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
inspect=(docker run --rm --entrypoint sh -v "$volume:/var/lib/postgresql" "$image" -ec)
reset_volume() {
  "${compose[@]}" rm --stop --force pg_db >/dev/null
  docker volume rm "$volume" >/dev/null
  docker volume create --label "com.docker.compose.project=$project" \
    --label com.docker.compose.volume=postgres_data "$volume" >/dev/null
}
sql() { "${compose[@]}" exec -T pg_db psql -XAt -U citadel -v ON_ERROR_STOP=1 -c "$1"; }
for layout in '' 19 19/docker; do
  if [[ -n "$layout" ]]; then
    reset_volume
    "${inspect[@]}" 'mkdir -p "/var/lib/postgresql/$1"' sh "$layout"
    "${compose[@]}" run --rm --no-deps pg_db postgres --version
  fi
  "${compose[@]}" up -d --wait --wait-timeout 90 pg_db
  [[ $(sql "SHOW server_version_num") == 190000 ]]
done
sql "CREATE TABLE retained(marker int); INSERT INTO retained VALUES (37)"
database=$("${compose[@]}" ps --quiet pg_db)
"${compose[@]}" stop pg_db
if PG_DATA=/var/lib/postgresql/18/docker "${compose[@]}" run --rm --no-deps pg_db postgres --version; then exit 1; fi
"${compose[@]}" up -d --no-recreate --wait --wait-timeout 90 pg_db
[[ $("${compose[@]}" ps --quiet pg_db) == "$database" ]]
[[ $(sql "SELECT marker FROM retained") == 37 ]]
# Replace only this disposable test volume with a retained older cluster marker.
reset_volume
"${inspect[@]}" 'cd /var/lib/postgresql; mkdir -p 18/docker; echo 18 >18/docker/PG_VERSION'
if "${compose[@]}" run --rm --no-deps pg_db postgres --version; then exit 1; fi
"${inspect[@]}" 'cd /var/lib/postgresql; test "$(cat 18/docker/PG_VERSION)" = 18; test ! -e 19; mv 18 unknown'
if "${compose[@]}" run --rm --no-deps pg_db postgres --version; then exit 1; fi
"${inspect[@]}" 'cd /var/lib/postgresql; test "$(cat unknown/docker/PG_VERSION)" = 18; test ! -e 19'
reset_volume
"${inspect[@]}" 'cd /var/lib/postgresql; mkdir -p 19/docker; touch 19/docker/.partial'
if "${compose[@]}" run --rm --no-deps pg_db postgres --version; then exit 1; fi
"${inspect[@]}" 'test -f /var/lib/postgresql/19/docker/.partial'
"${inspect[@]}" 'printf "19\nunexpected\n" > /var/lib/postgresql/19/docker/PG_VERSION'
if "${compose[@]}" run --rm --no-deps pg_db postgres --version; then exit 1; fi
"${inspect[@]}" 'cd /var/lib/postgresql/19/docker; mv PG_VERSION malformed; printf "19\n" > actual; ln -s actual PG_VERSION'
if "${compose[@]}" run --rm --no-deps pg_db postgres --version; then exit 1; fi
reset_volume
"${inspect[@]}" 'cd /var/lib/postgresql; mkdir 19; ln -s /tmp 19/docker'
if "${compose[@]}" run --rm --no-deps pg_db postgres --version; then exit 1; fi
echo 'PG19 empty-volume and empty-directory initialization, retained data and incompatible-storage rejection passed.'
