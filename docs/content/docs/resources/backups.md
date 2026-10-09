---
title: "Backups"
description: "Configure backup repositories, policies, runs, restores, and Swarm volume coverage."
---

Citadel provides backup and recovery for its control plane and Docker named
volumes, with snapshots stored in restic-compatible repositories.

## Make your first backup

A backup needs a **repository** (where it is stored) and a **policy** (what it saves).
This workflow backs up application data and restores a separate copy for testing.
For Citadel's database and security keys, use a separate **Citadel backup** policy
and follow [control-plane recovery](/docs/operations/control-plane-recovery).

You need an online Platform, a Docker named volume, and permission to manage
backup repositories and policies. Confirm the application's data actually lives
in that volume: bind mounts and container writable layers are not included.

### 1. Prepare the repository

1. Open **Backups** and create a backup repository.
2. Choose storage supported by the connection in the table below. For recovery
   from host failure, keep the repository away from that host.
3. Select a **Password Secret** for restic's repository encryption. For S3, also
   supply the endpoint, bucket, and credential secrets. See
   [repository settings](#s3-compatible-repository-settings).
4. Save, select **Initialize** for a new repository, then **Validate**. For an
   existing repository, use its original password and validate it.

| Backup execution | Supported repository |
| --- | --- |
| Citadel control plane or Local-connector workload | Core filesystem or S3-compatible |
| Local-connector workload with a repository path on that Docker host | Platform filesystem |
| Regular Agent, Edge Agent, or Swarm Node Agent | S3-compatible |

The S3 endpoint must be reachable from the target's backup helper, not just your
browser or Core. Keep the repository password separately accessible for recovery;
S3 credentials alone cannot decrypt the snapshots.

### 2. Create and run the policy

1. Create a policy and choose **Docker volume** under **Source → Type**.
2. Select the Platform and volume. For Swarm, select its Node and check
   [Swarm backup requirements](#docker-swarm) first.
3. On Standalone, keep **Consistency → Live**. Prepare an application-specific
   dump or maintenance window if the application needs coordinated writes;
   see [backup consistency](#backup-consistency) before copying live database files.
4. Select the repository under **Destination** and configure retention. Leave
   scheduled and webhook execution disabled for this first manual run.
5. Save, select **Run**, and wait for completion. Open **Runs** and inspect the
   snapshot status and logs, including warnings.

For a whole Stack or Deployment, select that source instead and review the
resolved named volumes before saving. Each volume has its own snapshot and
result; the run is not an atomic snapshot of the entire application.

Continue with a restore test below before relying on the policy. **Protected**
means a volume has backup coverage; it does not prove a backup succeeded.

### Backup consistency

The form offers **Stop attached containers**, but the current backup execution
path does not stop or restart workload containers. Use **Live** and arrange
application consistency separately. Do not rely on that setting to pause writes.

For Standalone workloads that require downtime, stop the relevant application
through its normal controls before starting the backup and restart it afterward.
For databases, follow the application's backup procedure and include its dump in
the protected volume. Swarm workload backups require running, stable Tasks and
support Live consistency only; see [Swarm requirements](#docker-swarm).

## Restores

Docker volume snapshots can be restored from a backup run into a Docker named volume.

### Restore a separate copy

1. Open the policy's **Runs** tab and select **Restore volume** on an available
   backup run. For a run containing several volumes, select the successful
   **Snapshot Volume** you want to recover.
2. Choose **Target Platform**. Check the repository rules below; on Swarm, also
   select **Target Node**.
3. Replace the pre-filled **Target Volume** with a new name, such as
   `application_data_recovered`. Leave **Overwrite existing target volume** disabled.
4. Select **Restore** and wait for completion. Check **Restore runs** and open
   the run's logs if it reports a failure or warning.
5. On the target Platform, open **Volumes** and use **Browse** to check the restored
   files. Then attach the restored volume to an isolated test instance of the
   application, using the expected image version and mount path. Check that it
   starts and can read representative data before declaring the backup usable.

Keep the test instance separate from production ports, networks, and external
integrations. File presence alone does not verify database or application
recovery. Record the snapshot tested, application version, and result.

[![Restore Volume dialog selecting an application data snapshot, the target Docker Platform, and a new recovery volume with overwrite disabled](/screenshots/backup-restore-selection.png)](/screenshots/backup-restore-selection.png)

This demo backup contains two volume snapshots. **Snapshot Volume** selects the
one to restore; **Target Platform** and **Target Volume** specify where it goes.
The example restores `application_data` as `application_data_recovered`. Review
all three before selecting **Restore**.

Overwrite restore is destructive. When **Overwrite existing target volume** is enabled, Citadel deletes and recreates the target volume before restoring the snapshot. The UI requires you to type the exact target volume name before the restore can start. Citadel rejects overwrite when the target volume is currently in use.

Restore history is shown in the same **Runs** tab under **Restore runs**. Use the log action on a restore run to inspect the restore output after the progress sheet is closed.

Repository and platform rules for restore:

- A Core filesystem repository can restore to the local platform only.
- A Platform filesystem repository restores on the same local platform that owns that repository path. Agent-based filesystem restore is not supported.
- An S3-compatible repository can restore to local, regular agent, or edge agent platforms because the restore runs from the target platform.
- A Swarm child snapshot can be restored only to a new named volume on an explicit current Node. Citadel does not overwrite a Swarm volume or automatically attach the restored volume to a Stack or Service.
- Citadel control-plane snapshots are restored offline while Core is stopped. They are not restored into a Docker volume through the web UI.

For the PostgreSQL dump, security assets, clean-environment restore sequence,
and recovery drill requirements, see
[Control-plane recovery](/docs/operations/control-plane-recovery).

## Backup Repositories

A backup repository is the destination where snapshots are stored.

**Filesystem** repositories store snapshots in a folder; **S3-compatible**
repositories use a bucket, such as MinIO or AWS S3. Choose a supported execution
location using the table in [Prepare the repository](#1-prepare-the-repository).

Remote regular Agent, Edge Agent, and Swarm Node Agent backup execution currently
requires **S3-compatible storage**. Filesystem repository fields may appear in
the UI, but the Agent executor rejects them. Use filesystem repositories only
for Citadel control-plane backups or workloads reached through the Local connector.

### S3-Compatible Repository Settings

S3-compatible repositories use restic over an S3-compatible API. The bucket stores restic's encrypted repository layout, not plain backup files.

When creating an S3-compatible repository, configure:

- Endpoint: the S3 API URL, such as `https://s3.example.com` or `http://192.168.1.50:9000`
- Bucket: the bucket name, such as `citadel-backups`
- Prefix: optional path inside the bucket for this repository
- Region: required by some providers; `us-east-1` is usually fine for local MinIO or RustFS-style stores
- Bucket Lookup: use `Path` for local S3-compatible stores unless DNS-style bucket hosts are configured
- Allow Insecure HTTP: enable only for local or private HTTP endpoints
- Password secret: the restic repository password used to encrypt and unlock backups
- Access Key Secret: the S3 access key
- Secret Key Secret: the S3 secret key
- Session Token Secret: optional, only for temporary S3 credentials

The restic password is separate from the S3 credentials. Losing the password secret means existing snapshots in that repository cannot be restored.

For Docker volume, stack, deployment, and Swarm Service backups, the S3 endpoint must be reachable from every helper container that executes the policy. For Citadel backups, the S3 endpoint must be reachable from Citadel Core.

Examples:

- If Citadel Core and the local Docker platform run on Docker Desktop, `http://host.docker.internal:9000` can be useful for a local RustFS or MinIO test service exposed on the host.
- If a regular agent or edge agent runs on another machine, do not use `host.docker.internal` unless that name is valid on that machine. Use a LAN-reachable IP address or DNS name, such as `http://192.168.1.50:9000` or `https://s3.internal.example.com`.

### Filesystem Repository Paths

Filesystem repository paths are resolved where the repository is executed:

- **Core filesystem** paths are on the Citadel Core host or inside the Core container volume.
- **Platform filesystem** paths are on the Docker host reached through the Local connector.

For example, a Platform filesystem repository path of `/srv/backup-01` on a local
platform stores the restic repository on that Docker host. The repository contains
folders such as `config`, `data`, `index`, `keys`, `locks`, and `snapshots`.

When Citadel runs against Docker Desktop on Windows, Linux-style paths such as `/srv/backup-01` are usually inside Docker Desktop's Linux VM filesystem. They will not appear as `C:\srv\backup-01` or `D:\srv\backup-01` in Windows Explorer unless that path is explicitly backed by a shared Windows bind mount.

After creating a repository:

1. Select **Initialize** if the repository is new.
2. Select **Validate** to confirm Citadel can read the repository.
3. Use **Check** for periodic repository health checks.
4. Use **Prune** when you want restic to remove unreferenced data after retention changes.

## Backup Policies

A backup policy defines what to back up, where to store it, and when it should run.

The policy's **Run As** identity supplies permissions for scheduled and
webhook-triggered runs. Choose an enabled User or, preferably for unattended
operation, a least-privileged Service Account. Manual runs use the
authenticated caller; runs started by an Automation Action use that Action
run's identity.

A Service Account run-as identity requires Team's **Custom access control**
capability. Scheduled and webhook-triggered policies additionally require
**Automated Operations**. If the Custom access control capability becomes
unavailable, Citadel preserves the binding as `Paused by license` and does not
start new runs under it.

See [Service Accounts](/docs/guides/service-accounts) for creating an account, assigning
access, and understanding **Use** permission. Backup Policies never store the
Service Account's persistent API token.

Supported sources:

- **Citadel backup**: creates a PostgreSQL logical dump and packages the file-backed keys required to recover the Citadel control plane.
- **Docker Volume**: backs up one selected Docker named volume.
- **Stack**: backs up all resolved Docker named volumes used by a stack.
- **Deployment**: backs up all resolved Docker named volumes used by a deployment.
- **Swarm Service**: backs up the supported node-local named volumes mounted by the current Tasks of a managed Swarm Service.

[![Backup policy selecting a Stack, previewing its named volume, and choosing a repository with retention of fourteen successful snapshots](/screenshots/backup-policy-source-destination.png)](/screenshots/backup-policy-source-destination.png)

Example policy using demo resources. Review the resolved volume and Node under
**Source**, then choose the repository and retention under **Destination**.
The example keeps 14 successful snapshots. **Protected** indicates backup
coverage for that volume; inspect completed runs to confirm successful backups.

### Backup Counts

The main **Backups** page lists every backup policy in the Citadel instance.
An individual platform's **Backups** summary only counts policies whose source
belongs to that platform:

- Docker volume policies created for the platform
- Stack policies whose stack runs on the platform
- Deployment policies whose deployment runs on the platform
- Swarm Service policies whose managed Service runs on the platform

Citadel control-plane backups are instance-wide and do not belong to a Docker
platform. They appear on the main **Backups** page but are not included in any
platform's backup count. For example, one Citadel backup and one volume backup
produce a total of two policies on the main page and one policy on the volume's
platform.

Only Docker named volumes are backed up. Bind mounts such as `./data:/app/data` or `/host/path:/data` are host paths, not Docker volumes, and are not included in Docker volume, stack, or deployment backups.

A Citadel backup does not archive all of `/app/data`. Git caches, stack
workspaces, automation working directories, logs, and backup staging files are
recreated from database state or external systems and are deliberately
excluded.

If the JWT or local secret-encryption key is supplied through external
configuration, its plaintext value is not copied into the snapshot. The
manifest records that dependency. Preserve the external value separately or
the recovered installation will not be able to authenticate existing sessions
or decrypt stored credentials.

The official Citadel Core image includes `pg_dump`. Native installations and
custom images must provide a PostgreSQL client that supports the configured
server version. Set `Backups__PostgresDumpPath` when `pg_dump` is not available
on `PATH`.

## Docker Platform Backups

Citadel reads named volumes through the target's Docker daemon using a backup
helper. S3 backups upload from that target directly to the bucket, so the S3
address and credentials must work from the helper's network. Remote volume
contents do not pass through Core. For supported repository locations, see
[Prepare the repository](#1-prepare-the-repository).

### Docker Swarm

Citadel supports node-scoped backup of Docker Swarm local named volumes. A
Swarm volume is identified by its Platform, owning Node, and volume name. Equal
volume names on two Nodes are treated as different backup sources.

Supported Swarm sources are:

- one named volume selected on an explicit Node
- a managed Swarm Stack, resolved from its current Services and Tasks
- a managed Swarm Service, resolved from its current Tasks

Swarm backup requires:

- a usable connection to every required Node: the existing manager connection
  covers that manager, and node agents cover other Nodes
- current, stable Task placement with all desired Tasks running
- Docker-managed named volumes using the `local` driver without driver options
- an S3-compatible backup repository reachable from every required Node
- **Live** consistency

Citadel rejects stale or incomplete placement, missing Node coverage, scaled-to-zero workloads, cluster volumes, volume-plugin storage, and `local` volumes configured with driver options. Bind mounts, `tmpfs`, container writable layers, Docker Secrets, and Docker Configs are not included.

For a Stack or Service, snapshots are created one volume at a time. They are
crash-consistent at best and do not represent one atomic point in time across
all Nodes. A failed child marks the workload run as failed, while snapshots
already completed remain available for individual restore.

Keep a Citadel control-plane backup alongside workload-volume policies. The
control-plane backup protects Citadel configuration, bindings, encrypted
secrets, and recovery assets; it does not replace an application-data backup.

## Running A Backup

Open **Backups**, select a policy, and use **Run**. Citadel shows a progress sheet with the run status and restic summary.

Successful runs show:

- Snapshot count or snapshot ID
- Files processed
- Bytes scanned and added
- Any warnings

If a policy contains multiple volumes, Citadel creates one backup item per volume so each volume has its own status and snapshot metadata.

The policy **Runs** tab contains backup and restore execution history and logs.
The **Activities** tab records policy creation, configuration changes, renames,
and archival actions.

## Scheduled Backups

Scheduled backups require Team's `Automated Operations` capability.

Configure the policy schedule and timezone when Citadel should queue backups
without an operator. If the capability becomes unavailable, Citadel preserves
the schedule but does not queue new scheduled runs. Manual backup and restore
remain available.

## Webhook Triggers

Webhook-triggered backups require Team's `Automated Operations` capability.

Enable **Webhook** on a backup policy when an external system should queue the backup.

For the shared listener model, authentication options, URL shape, and troubleshooting, see [Webhooks](/docs/guides/webhooks).

Common uses:

- run a backup before a deployment pipeline updates a stack
- run a backup before scheduled maintenance
- let an external scheduler trigger a policy without using Citadel's cron schedule

After the policy is saved, copy the listener URL from the Webhook section into the external provider. Citadel validates the configured provider signature or token when a secret is set, then queues a normal backup run.

Webhook runs use the policy's **Run As** User or Service Account and appear in
the same run history as manual and scheduled runs. The webhook secret
authenticates the delivery; it is not the execution identity. Disabled policies
do not run from webhooks.

## Retention

The **Keep last successful** setting controls how many successful snapshots Citadel keeps for a policy.

When retention is enabled, Citadel runs restic retention after the backup completes. A retention failure is reported as a warning so the backup snapshot itself is still recorded.

## Alerts

The policy form exposes **Alert on failure**, but the current backup execution
path does not emit failure alerts from that setting. Review **Runs**, its logs,
and **Activities** for failed or interrupted backups. Do not rely on the absence
of a notification as evidence that a backup succeeded.

## Practical Recommendations

Keep repository credentials and external encryption keys accessible outside the
installation they protect. Repeat a restore test after material storage or
application changes. On Swarm, record the source Node as well as the volume
name: identical names on different Nodes can contain different data.

## Troubleshooting

| Symptom | Next step |
| --- | --- |
| Repository test fails | Verify the destination, restic password, storage credentials, and connectivity from the configured execution location. |
| Policy has no eligible data | Preview coverage and confirm it contains the intended named volumes. Bind mounts are not included in workload-volume backups. |
| Remote filesystem repository is rejected | Use S3-compatible storage for Agent, Edge Agent, or Swarm Node execution. |
| A run failed or its progress connection closed | Inspect its entry in Runs and the logs before retrying; check whether it already completed. |
| Restore target is rejected | Check target Platform/Node access and existing-volume restrictions under [Restores](#restores). |
| Restored files exist but the application fails | Check application consistency, ownership, image/configuration compatibility, and the application's recovery steps. A file restore alone does not verify application recovery. |

## License Availability

Community can:

- create and manage backup repositories
- create backup policy definitions
- start backups manually
- restore Docker volume backups
- view backup and restore history and logs

Backup schedules and webhook-triggered backup execution require Team's
`Automated Operations` capability. If the capability becomes unavailable,
configured triggers remain stored but pause. Manual backup and restore remain
available.
