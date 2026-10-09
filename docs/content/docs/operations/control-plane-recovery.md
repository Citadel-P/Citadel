---
title: "Control-plane recovery"
description: "Back up and recover the Citadel database, keys, and control-plane state."
---

Citadel workload backups and recovery of Citadel itself are separate
operations. A complete Citadel control-plane backup contains:

- a PostgreSQL logical dump created with `pg_dump --format=custom`;
- the JWT signing key;
- the local secret-encryption key;
- the Core-to-Agent Ed25519 private signing key (the public key is derived from it);
- a manifest listing any keys that must instead be recovered from external
  configuration.

The database alone is not a usable control-plane backup. Losing the
secret-encryption key makes stored local secrets and provider credentials
undecryptable. Losing the Core Ed25519 private key prevents existing regular Agents
from trusting Core requests.

For an upgrade, create and export a bundle before replacing Core. For a lost or
unusable installation, start with [Required Inputs](#required-inputs), then
[export the bundle](#export-a-recovery-bundle) and restore it offline.

This procedure restores Citadel's management state. It does not restore Docker
workload volumes, application databases, or the hosts themselves. Protect those
separately with [workload backups](/docs/resources/backups).

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

```bash
docker compose ps
docker compose restart
docker compose up -d
docker compose down
```

Manage regular and Edge Agent containers directly from their remote Docker
host, using the installation method shown by Citadel. For example:

```bash
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

The Core executable includes an offline `restore-system` command. It validates
the bundle inventory and checksums before restoring PostgreSQL in one transaction.
It requires the original secret-encryption key and explicit confirmation. It
does not copy recovery keys into the data volume; the procedure below does that
first. Test recovery before relying on it in production.

## Required Inputs

For the default file-backed configuration, preserve these paths from the
`/app/data` volume:

```text
jwtsecret
secret-encryption-key
agent/signing-key
```

If `Jwt__Key` or `Secrets__EncryptionKey` is supplied by a secret manager or
environment configuration, preserve that external value through the same
secure system. It will not exist as a file in `/app/data`.

Also retain:

- the exact Citadel image tag or digest;
- the PostgreSQL image reference used by the installation;
- the Compose file and non-secret configuration;
- PostgreSQL connection settings;
- the password for every restic repository, stored outside Citadel;
- externally managed OIDC, Vault, S3, registry, and Git credentials.

Do not include Git caches, stack workspaces, automation working directories,
logs, or backup staging folders. Citadel rebuilds those from database state and
the original external systems.

## Create A Backup

1. Create or select a Core filesystem or S3-compatible backup repository.
2. Store its restic password, repository location, and any storage credentials
   outside Citadel as part of the recovery plan. A repository on the same host
   needs an independently recoverable copy if that host is lost.
3. Create a backup policy with **Citadel backup** as its source.
4. Select **Run** and wait for the run to succeed.
5. Record the backup run ID, snapshot ID, and Core image reference. Export the
   bundle below to verify that you can retrieve it without signing into Citadel.

Citadel runs `pg_dump --format=custom --no-owner --no-privileges` without
putting the database password in process arguments. It stages the dump and
recovery assets in a private directory, sends that directory to restic, and
deletes the plaintext staging directory before completing the run.

The official Citadel Core image includes the PostgreSQL client. Native
installations and custom images must install a `pg_dump` version that supports
the PostgreSQL server and can set `Backups__PostgresDumpPath` to its executable.

The recovery bundle directory contains:

```text
manifest.json
checksums.json
database/citadel.dump
recovery/jwtsecret
recovery/secret-encryption-key
recovery/agent/signing-key
```

Keys supplied through `Jwt__Key` or `Secrets__EncryptionKey` are listed in
`manifest.json` under `requiredExternalConfiguration` and are not copied into
the bundle. Recover their original values from your secret manager.

## Export A Recovery Bundle

Install restic on the recovery host. Configure `RESTIC_REPOSITORY` and its
password through `RESTIC_PASSWORD` or `RESTIC_PASSWORD_FILE`, using the same
repository settings as Citadel. For S3, also supply the original AWS credentials
and any required session token. Load credentials securely; do not put their
literal values in shell history.

The commands below use Bash on a Linux Docker host. Find the snapshot by its
backup-run tag, then list its contents:

```bash
run_id="<backup-run-id>"
backup="$PWD/citadel-recovery-$run_id"
umask 077
restic snapshots --tag "backup-run:$run_id"

snapshot_id="<snapshot-id-from-the-list>"
restic ls "$snapshot_id"
```

Find the directory containing `manifest.json`, `checksums.json`, `database/`,
and `recovery/`. Set `bundle_path` to that directory's full path **inside the
snapshot**, then restore just that directory:

```bash
bundle_path="/<bundle-directory-from-restic-ls>"
restic restore "${snapshot_id}:${bundle_path}" --target "$backup"
ls -la "$backup"
```

Citadel backs up an absolute staging path. Restoring the whole snapshot keeps
those parent directories, so the manifest would not be directly under
`$backup`. Restic's [subfolder restore syntax](https://restic.readthedocs.io/en/stable/050_restore.html)
places the bundle's contents directly in the target directory.

Keep the export private: it can contain keys that decrypt stored secrets and
authenticate Core to Agents. The offline command below verifies every checksum
before modifying PostgreSQL.

## Restore Into A Clean Environment

Use an empty PostgreSQL database and an empty Citadel data volume. The restore
command can replace existing database objects; the confirmation flag is not a
check that the target is empty.

These steps assume the supplied published-image Compose file, with PostgreSQL
at `PG_HOST=pg_db` and project-scoped volumes. A different project name does
not isolate an external database or explicitly shared volumes. If you changed
those settings, configure separate recovery storage before continuing.

Check for `DATABASE_URL` and `ConnectionStrings__Postgres` overrides in the
copied environment: they take precedence over `PG_HOST` and the other `PG_*`
settings. Remove them to use the example's new `pg_db`, or point them explicitly
at an empty recovery database. Likewise, the commands below assume the default
data path `/app/data`; adapt them if you use `CITADEL_DATA_ROOT`.

1. Prepare a separate recovery installation directory with copies of your
   Compose file and private `.env`. Pin `CITADEL_IMAGE` to the exact image
   recorded when the backup was created; compare it with `coreVersion` in
   `manifest.json`. Do not use a moving `latest` tag for recovery. Preserve
   the original external keys listed in `requiredExternalConfiguration`,
   and copy your TLS overlay and certificates if needed.

2. From the **original installation directory**, stop Core. Stop every other
   Core instance that uses that database too:

```bash
docker compose stop server
```

3. From the **recovery installation directory**, set the absolute export path
   and choose a project name that has never been used. Check for existing volumes:

```bash
backup="/absolute/path/to/citadel-recovery-<backup-run-id>"
recovery_project="citadel-recovery"
docker volume ls --filter "label=com.docker.compose.project=$recovery_project"
```

If the list contains any volumes, choose another project name. Keep the original
volumes intact. Start only the new PostgreSQL instance and wait for it to be healthy:

```bash
docker compose -p "$recovery_project" up -d --wait pg_db
```

4. Copy the recovery keys into the new data volume, give them to the Core
   runtime user, and run the offline restore. This one-off container runs the
   restore command, not the Core server:

```bash
docker compose -p "$recovery_project" run --rm --no-deps \
  --user 0 --entrypoint sh \
  --volume "$backup:/bundle:ro" server -ec '
    umask 077
    cp -R /bundle/recovery/. /app/data/
    chown -R 65532:0 /app/data
    chmod -R go-rwx /app/data
    exec /app/citadel-server restore-system \
      --bundle /bundle --confirm-instance-replacement
  '
```

The command reads the original file-backed encryption key from `/app/data`,
or the original `Secrets__EncryptionKey` from the container environment.
It never generates a replacement key. A new key cannot decrypt the restored
secrets. Wait for `Citadel system recovery bundle restored successfully.`
before continuing.

5. Start the recovered Core and check its status:

```bash
docker compose -p "$recovery_project" up -d --wait server
docker compose -p "$recovery_project" ps
docker compose -p "$recovery_project" logs --tail=100 server
```

Include your TLS overlay in all Compose commands if configured. Keep using the
recovery project name for later commands. Leave the original Core stopped while
checking the recovered instance. Starting Core also starts its background
workers and schedules; review the restored automation settings as part of recovery.

Confirm that:

- an administrator can sign in;
- the instance and license state are unchanged;
- users, teams, roles, and resource access remain correct;
- stored secrets can be used;
- platforms, stacks, releases, Git repositories, backup history, activity, and
  schedules are present;
- Agents reconnect without re-enrollment;
- a new harmless operation, such as creating and deleting a tag, succeeds.

Compare current workloads with the restored configuration before running a
deployment or automatic reconciliation. Changes made after the snapshot are
absent from Citadel's recovered history, even if they are still running on Docker.

## Rehearse recovery

Use a separate test host with its own Docker engine, database, data volume, and
private access URL. Before starting the recovered Core, isolate it from
production Agents, external service APIs, and notification destinations. The
restored database contains real credentials, schedules, and resource addresses;
changing the Compose project name alone does not isolate those connections.

Use the export and offline restore steps above against that test installation.
Keep production Core running only when the test environment is isolated in this
way. Verify sign-in, resource records, and a harmless write. Expect production
Agents to remain disconnected in the drill; test those connections only during
a controlled recovery or against separate test Agents.

Record the snapshot, image versions, required external keys, and time taken to
recover. An offline restore success validates the bundle and database import;
sign-in and functional checks establish whether the recovered instance is usable.

## Failure Rules

Do not start Core when:

- bundle validation or the restore command fails;
- the target database or data volume was not empty before this procedure;
- `secret-encryption-key` is missing and no external
  `Secrets__EncryptionKey` is available;
- `jwtsecret` is missing and no external `Jwt__Key` is available;
- `agent/signing-key` is missing.

Keep the failed target isolated, correct the recovery inputs, and repeat the
procedure with a new empty target. Preserve the original installation and backup.
