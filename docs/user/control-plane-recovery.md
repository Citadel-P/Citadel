# Citadel Control-Plane Recovery

Citadel workload backups and recovery of Citadel itself are separate
operations. A complete Citadel control-plane backup contains:

- a PostgreSQL logical dump created with `pg_dump --format=custom`;
- the JWT signing key;
- the local secret-encryption key;
- the Core-to-Agent Ed25519 key pair;
- ASP.NET Core data-protection keys;
- any equivalent keys supplied through external configuration.

The database alone is not a usable control-plane backup. Losing the
secret-encryption key makes stored local secrets and provider credentials
undecryptable. Losing the Core Ed25519 private key prevents existing Agents
from trusting Core requests.

## Current Availability

The offline `citadel-recovery` command and the final packaged Citadel System
backup flow are not yet shipped. Until they are available, use the manual
Compose procedure below and test it before relying on it.

Do not treat the current Citadel System restic snapshot as a complete
control-plane backup unless it contains a PostgreSQL dump and the recovery
assets listed in this document.

## Required Inputs

For the default file-backed configuration, preserve these paths from the
`/app/data` volume:

```text
jwtsecret
secret-encryption-key
keys/id_ed25519
keys/id_ed25519.pub
keys/dataprotection/
```

If `Jwt__Key` or `Secrets__EncryptionKey` is supplied by a secret manager or
environment configuration, preserve that external value through the same
secure system. It will not exist as a file in `/app/data`.

Also retain:

- the exact Citadel image tag or digest;
- the Compose file and non-secret configuration;
- PostgreSQL connection settings;
- the password for every restic repository, stored outside Citadel;
- externally managed OIDC, Vault, S3, registry, and Git credentials.

Do not include Git caches, stack workspaces, automation working directories,
logs, or backup staging folders. Citadel rebuilds those from database state and
the original external systems.

## Create A Backup

The commands below use the repository's default Compose service names:
`server` and `pg_db`.

1. Create a private backup directory and stop Core:

```powershell
$backup = "citadel-recovery-$(Get-Date -Format yyyyMMdd-HHmmss)"
New-Item -ItemType Directory -Path "$backup\recovery\keys" -Force
docker compose stop server
```

2. Create the logical database dump inside PostgreSQL, then copy it out:

```powershell
docker compose exec -T pg_db sh -c 'PGPASSWORD="$POSTGRES_PASSWORD" pg_dump --host=127.0.0.1 --username="$POSTGRES_USER" --format=custom --no-owner --no-privileges --file=/tmp/citadel.dump "$POSTGRES_DB"'
docker compose cp pg_db:/tmp/citadel.dump "$backup\citadel.dump"
docker compose exec -T pg_db rm -f /tmp/citadel.dump
```

3. Copy the file-backed recovery assets from the stopped Core container:

```powershell
docker compose cp server:/app/data/jwtsecret "$backup\recovery\jwtsecret"
docker compose cp server:/app/data/secret-encryption-key "$backup\recovery\secret-encryption-key"
docker compose cp server:/app/data/keys/id_ed25519 "$backup\recovery\keys\id_ed25519"
docker compose cp server:/app/data/keys/id_ed25519.pub "$backup\recovery\keys\id_ed25519.pub"
docker compose cp server:/app/data/keys/dataprotection "$backup\recovery\keys\dataprotection"
```

Skip a file-backed key only when the equivalent value is explicitly supplied
and backed up through external configuration.

4. Record checksums and restart Core:

```powershell
Get-ChildItem $backup -Recurse -File |
  Get-FileHash -Algorithm SHA256 |
  Format-Table Hash, Path |
  Out-File "$backup\SHA256SUMS.txt"
docker compose start server
```

Store the resulting directory in encrypted storage separate from the Citadel
host. Restrict access because the recovery assets can decrypt stored secrets
and authenticate Core to Agents.

## Restore Into A Clean Environment

Restore into an empty PostgreSQL database and an empty Citadel data volume.
Do not overwrite a running installation.

Use a clean host or a new Compose project so Docker creates new volumes. The
example below uses `citadel-recovery`; choose a name that does not already
exist. Set `$backup` to the verified backup directory.

1. Stop the original Core, select the clean Compose project, and start only
PostgreSQL:

```powershell
$backup = "path\to\citadel-recovery-yyyyMMdd-HHmmss"
docker compose stop server
$env:COMPOSE_PROJECT_NAME = "citadel-recovery"
docker compose up -d pg_db
```

2. Verify the recorded checksums before making changes.

3. Recreate the target database. Replace the database name when your Compose
configuration does not use `POSTGRES_DB`:

```powershell
docker compose exec -T pg_db sh -c 'PGPASSWORD="$POSTGRES_PASSWORD" dropdb --host=127.0.0.1 --username="$POSTGRES_USER" --if-exists "$POSTGRES_DB"'
docker compose exec -T pg_db sh -c 'PGPASSWORD="$POSTGRES_PASSWORD" createdb --host=127.0.0.1 --username="$POSTGRES_USER" "$POSTGRES_DB"'
```

4. Copy and restore the logical dump:

```powershell
docker compose cp "$backup\citadel.dump" pg_db:/tmp/citadel.dump
docker compose exec -T pg_db sh -c 'PGPASSWORD="$POSTGRES_PASSWORD" pg_restore --host=127.0.0.1 --username="$POSTGRES_USER" --dbname="$POSTGRES_DB" --no-owner --no-privileges --exit-on-error --single-transaction /tmp/citadel.dump'
docker compose exec -T pg_db rm -f /tmp/citadel.dump
```

5. Create the stopped Core container with a new empty data volume, then copy
the recovery assets:

```powershell
docker compose create server
docker compose cp "$backup\recovery\." server:/app/data/
```

Supply any externally configured keys before starting Core.

6. Start Core and verify:

```powershell
docker compose start server
docker compose ps
```

Confirm that:

- an administrator can sign in;
- the instance and license state are unchanged;
- users, teams, roles, and resource access remain correct;
- stored secrets can be used;
- platforms, stacks, releases, Git repositories, backup history, activity, and
  schedules are present;
- Agents reconnect without re-enrollment;
- a new harmless operation, such as creating and deleting a tag, succeeds.

## Failure Rules

Do not start Core when:

- the database dump checksum fails;
- the target database was not empty before restore;
- `secret-encryption-key` is missing and no external
  `Secrets__EncryptionKey` is available;
- `jwtsecret` is missing and no external `Jwt__Key` is available;
- either Core Ed25519 key is missing;
- data-protection keys are missing from an installation that used them.

Keep the failed target isolated, correct the recovery inputs, recreate the
empty target, and repeat the restore.
