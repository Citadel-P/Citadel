refuse() { echo "Citadel requires a dedicated PostgreSQL 19 volume at /var/lib/postgresql/19/docker. Inspect and preserve existing storage." >&2; exit 1; }
[[ $PG_MAJOR == 19 && $PGDATA == /var/lib/postgresql/19/docker ]] || refuse
cd /var/lib/postgresql
shopt -s nullglob dotglob
for path in * 19/*; do
  [[ ( $path == 19 || $path == 19/docker ) && -d $path && ! -L $path && -r $path && -x $path ]] || refuse
done
contents=(19/docker/*)
if ((${#contents[@]})); then
  [[ -f 19/docker/PG_VERSION && ! -L 19/docker/PG_VERSION ]] && cmp -s 19/docker/PG_VERSION <(printf '19\n') || refuse
fi
exec docker-entrypoint.sh "$@"
