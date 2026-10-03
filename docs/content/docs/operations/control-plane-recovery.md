---
title: "Control-plane recovery"
description: "Back up and recover the Citadel database, keys, and control-plane state."
---

Citadel workload backups and recovery of Citadel itself are separate
operations. A complete Citadel control-plane backup contains:

- a PostgreSQL logical dump created with `pg_dump --format=custom`;
- the JWT signing key;
- the local secret-encryption key;
- the Core-to-Agent Ed25519 key pair;
- a manifest entry for each equivalent key supplied through external
  configuration.

The database alone is not a usable control-plane backup. Losing the
secret-encryption key makes stored local secrets and provider credentials
undecryptable. Losing the Core Ed25519 private key prevents existing Agents
from trusting Core requests.

## System Containers

On the local Docker Platform, Citadel marks its Core and PostgreSQL containers
as **System**. On remote Platforms, generated regular Agent and Platform Edge
Agent installation commands apply the same classification to the container
that maintains the Platform connection. System containers remain visible so
operators can inspect their state, resource usage, logs, and configuration
under the normal Platform permissions. They are also included in Platform
container totals and usage statistics.

Citadel does not expose start, stop, pause, resume, restart, or delete controls
for System containers. This prevents a bulk action or direct API request from
disabling the control plane or remote connection that must report and recover
the operation. Manage local Core and PostgreSQL containers from the Docker host
with the Compose file used to install Citadel:

```powershell
docker compose ps
docker compose restart
docker compose up -d
docker compose down
```

Manage regular and Edge Agent containers directly from their remote Docker
host, using the installation method shown by Citadel. For example:

```powershell
docker restart citadel-agent
docker restart edge-agent
```

Existing Agent containers must be recreated once with a current generated
installation command before Citadel can classify and protect them.

Use `docker compose down` only when intentionally stopping the complete
installation. The System label is a safety boundary in Citadel, not a
replacement for restricting access to the Docker host.

## Current Availability

Citadel creates complete control-plane backup bundles from the **Citadel
backup** source in the backup-policy UI. Each restic snapshot contains a
PostgreSQL custom-format dump, a recovery manifest, checksums, and every
required file-backed recovery key.

The offline `citadel-recovery` command is not yet shipped. Restore the bundle
manually into a clean environment using the procedure below. Test this
procedure before relying on it in production.

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

1. Create or select a Core filesystem or S3-compatible backup repository.
2. Store its restic password outside Citadel as part of the recovery plan.
3. Create a backup policy with **Citadel backup** as its source.
4. Select **Run** and wait for the run to succeed.
5. Record the backup run ID and snapshot ID shown in the run details.

Citadel runs `pg_dump --format=custom --no-owner --no-privileges` without
putting the database password in process arguments. It stages the dump and
recovery assets in a private directory, sends that directory to restic, and
deletes the plaintext staging directory before completing the run.

The official Citadel Core image includes the PostgreSQL client. Native
installations and custom images must install a `pg_dump` version that supports
the PostgreSQL server and can set `Backups__PostgresDumpPath` to its executable.

The snapshot root contains:

```text
manifest.json
checksums.json
database/citadel.dump
recovery/jwtsecret
recovery/secret-encryption-key
recovery/keys/id_ed25519
recovery/keys/id_ed25519.pub
recovery/keys/dataprotection/
```

Assets supplied through external configuration are listed in `manifest.json`
with origin `ExternalConfiguration` and are not copied into the snapshot.

## Export A Recovery Bundle

Use restic with the same repository configuration and password used by
Citadel. Locate the snapshot by its backup-run tag, then restore it into a
private local directory:

```powershell
$runId = "<backup-run-id>"
$backup = "citadel-recovery-$runId"
restic -r "<repository>" snapshots --tag "backup-run:$runId"
restic -r "<repository>" restore "<snapshot-id>" --target "$backup"
```

For S3-compatible repositories, configure `AWS_ACCESS_KEY_ID`,
`AWS_SECRET_ACCESS_KEY`, the restic repository URL, and `RESTIC_PASSWORD`
before running these commands. Do not store these credentials in shell
history.

Verify `checksums.json` against every listed file before proceeding. Restrict
access to the exported directory because it can contain keys that decrypt
stored secrets and authenticate Core to Agents.

## Restore Into A Clean Environment

Restore into an empty PostgreSQL database and an empty Citadel data volume.
Do not overwrite a running installation.

Use a clean host or a new Compose project so Docker creates new volumes. The
example below uses `citadel-recovery`; choose a name that does not already
exist. Set `$backup` to the verified backup directory.

1. Stop the original Core, select the clean Compose project, and start only
PostgreSQL:

```powershell
$backup = "path\to\citadel-recovery-<backup-run-id>"
docker compose stop server
$env:COMPOSE_PROJECT_NAME = "citadel-recovery"
docker compose up -d pg_db
```

2. Verify `manifest.json`, confirm its Citadel version is compatible with the
   target image, and verify every entry in `checksums.json`.

3. Recreate the target database. Replace the database name when your Compose
configuration does not use `POSTGRES_DB`:

```powershell
docker compose exec -T pg_db sh -c 'PGPASSWORD="$POSTGRES_PASSWORD" dropdb --host=127.0.0.1 --username="$POSTGRES_USER" --if-exists "$POSTGRES_DB"'
docker compose exec -T pg_db sh -c 'PGPASSWORD="$POSTGRES_PASSWORD" createdb --host=127.0.0.1 --username="$POSTGRES_USER" "$POSTGRES_DB"'
```

4. Copy and restore the logical dump:

```powershell
docker compose cp "$backup\database\citadel.dump" pg_db:/tmp/citadel.dump
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


