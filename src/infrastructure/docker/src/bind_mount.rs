//! Exact helper commands shared with the restricted Swarm Node Agent allowlist.

pub use citadel_backups::spec::validate_bind_path;

pub const BIND_BACKUP_SCRIPT: &str = r#"
set -eu
if ! test -d /data && ! test -f /data; then
  echo 'Bind backup source must be a regular file or directory.' >&2; exit 1
fi
exec restic "$@"
"#;

pub const BIND_RESTORE_SCRIPT: &str = r#"
set -eu
if ! test -d /target && ! test -f /target; then
  echo 'Restore target must be an existing regular file or directory.' >&2; exit 1
fi
if test "$2" != true; then
  if test -f /target || test -n "$(find /target -mindepth 1 -maxdepth 1 -print -quit)"; then
    echo 'Target is not empty. Explicit overwrite permission is required.' >&2; exit 1
  fi
fi
mkdir -p /tmp/citadel-restore
restic restore "$1" --target /tmp/citadel-restore
source=/tmp/citadel-restore/source
if ! test -e "$source"; then source=/tmp/citadel-restore/data; fi
if test -L "$source"; then echo 'Snapshot root must not be a symbolic link.' >&2; exit 1; fi
if test -d "$source" && test -d /target; then
  cp -a "$source/." /target/
elif test -f "$source" && test -f /target; then
  # A mounted file cannot be unlinked/replaced by cp. Write through its inode.
  cat "$source" > /target
  chown "$(stat -c '%u:%g' "$source")" /target
  chmod "$(stat -c '%a' "$source")" /target
  touch -r "$source" /target
else
  echo 'Snapshot and target must both be directories or both be regular files.' >&2; exit 1
fi
"#;

pub const BIND_PATH_PROBE: &str = r#"
set -eu
path=$1
while test -n "$path" && test "$path" != /; do
  if test -L "/source$path"; then echo 'Host path contains a symlink; select its resolved path instead.' >&2; exit 1; fi
  path=${path%/*}
done
if ! test -d "/source$1" && ! test -f "/source$1"; then
  echo 'Host path must already exist as a regular file or directory on the selected Docker host.' >&2; exit 1
fi
"#;
