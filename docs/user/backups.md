# Backups

Citadel backups let you save Citadel system data and Docker named volumes to a restic-compatible repository.

## Backup Repositories

A backup repository is the destination where snapshots are stored.

Supported repository types:

- **Filesystem**: stores snapshots in a folder.
- **S3 compatible**: stores snapshots in an S3-compatible bucket such as MinIO, AWS S3, or another compatible service.

For filesystem repositories:

- **Core** location stores backups on the Citadel Core host or container volume.
- **Platform** location stores backups on the selected Docker platform.

Choose the repository location based on where the backup runs:

- Use **Core filesystem** for Citadel system backups and Docker volume backups from a local platform.
- Use **Platform filesystem** for Docker volume, stack, or deployment backups from a regular agent or edge agent when snapshots should stay on that platform host.
- Use **S3 compatible** when backups should be independent of the Core host and platform host filesystem.

Citadel prevents policies that combine a remote regular agent or edge agent source with a Core filesystem repository. Core cannot directly write a filesystem snapshot for Docker volumes that live behind an agent.

For remote regular agent and edge agent platforms, prefer an S3-compatible repository unless you explicitly want snapshots stored on that platform's host filesystem.

### Filesystem Repository Paths

Filesystem repository paths are resolved where the repository is executed:

- **Core filesystem** paths are on the Citadel Core host or inside the Core container volume.
- **Platform filesystem** paths are on the selected platform's Docker host.
- **Regular agent** and **edge agent** platform paths are still Docker host paths, not paths inside the agent container.

For example, a Platform filesystem repository path of `/srv/backup-01` on an agent platform creates the restic repository on the Docker host managed by that agent. The repository contains folders such as `config`, `data`, `index`, `keys`, `locks`, and `snapshots`.

When Citadel runs against Docker Desktop on Windows, Linux-style paths such as `/srv/backup-01` are usually inside Docker Desktop's Linux VM filesystem. They will not appear as `C:\srv\backup-01` or `D:\srv\backup-01` in Windows Explorer unless that path is explicitly backed by a shared Windows bind mount.

To inspect a Platform filesystem repository on the same Docker daemon, run a temporary container with the image used by that platform type.

For a local platform:

```powershell
docker run --rm -it --entrypoint sh --mount type=bind,source=/srv/backup-01,target=/backup ghcr.io/citadel-p/citadel:1.0
```

For a regular agent or edge agent platform:

```powershell
docker run --rm -it --entrypoint sh --mount type=bind,source=/srv/backup-01,target=/backup ghcr.io/citadel-p/citadel.agent:1.0
```

Then inside the container:

```sh
ls -la /backup
```

Check the path carefully. `/srv/backup-01` and `/serv/backup-01` are different paths.

After creating a repository:

1. Select **Initialize** if the repository is new.
2. Select **Validate** to confirm Citadel can read the repository.
3. Use **Check** for periodic repository health checks.
4. Use **Prune** when you want restic to remove unreferenced data after retention changes.

## Backup Policies

A backup policy defines what to back up, where to store it, and when it should run.

Supported sources:

- **Citadel System**: backs up Citadel's own data folder.
- **Docker Volume**: backs up one selected Docker named volume.
- **Stack**: backs up all resolved Docker named volumes used by a stack.
- **Deployment**: backs up all resolved Docker named volumes used by a deployment.

Only Docker named volumes are backed up. Bind mounts such as `./data:/app/data` or `/host/path:/data` are host paths, not Docker volumes, and are not included in Docker volume, stack, or deployment backups.

## Remote Platforms

Backups can run against:

- Local platforms
- Regular agent platforms
- Edge agent platforms

For remote Docker volume, stack, and deployment backups:

- S3-compatible repositories run from the target platform and upload directly to the bucket.
- Platform filesystem repositories run on the selected platform and write to the configured host path.
- Core filesystem repositories are only valid for Citadel system backups and local-platform backups.

This avoids routing remote volume contents through Citadel Core.

## Running A Backup

Open **Backups**, select a policy, and use **Run**. Citadel shows a progress sheet with the run status and restic summary.

Successful runs show:

- Snapshot count or snapshot ID
- Files processed
- Bytes scanned and added
- Any warnings

If a policy contains multiple volumes, Citadel creates one backup item per volume so each volume has its own status and snapshot metadata.

## Webhook Triggers

Enable **Webhook** on a backup policy when an external system should queue the backup.

For the shared listener model, authentication options, URL shape, and troubleshooting, see `docs/user/webhooks.md`.

Common uses:

- run a backup before a deployment pipeline updates a stack
- run a backup before scheduled maintenance
- let an external scheduler trigger a policy without using Citadel's cron schedule

After the policy is saved, copy the listener URL from the Webhook section into the external provider. Citadel validates the configured provider signature or token when a secret is set, then queues a normal backup run.

Webhook runs use the policy's **Run As User** setting and appear in the same run history as manual and scheduled runs. Disabled policies do not run from webhooks.

## Retention

The **Keep last successful** setting controls how many successful snapshots Citadel keeps for a policy.

When retention is enabled, Citadel runs restic retention after the backup completes. A retention failure is reported as a warning so the backup snapshot itself is still recorded.

## Restores

Docker volume snapshots can be restored from a backup run into a Docker named volume.

Open a backup policy, go to **Runs**, and select the restore action on an available Docker volume backup run. Citadel opens a restore dialog and then streams progress in a sheet.

By default, restore to a new volume name. This is the safest option because the original volume is left untouched.

Overwrite restore is destructive. When **Overwrite existing target volume** is enabled, Citadel deletes and recreates the target volume before restoring the snapshot. The UI requires you to type the exact target volume name before the restore can start. Citadel rejects overwrite when the target volume is currently in use.

Restore history is shown in the same **Runs** tab under **Restore runs**. Use the log action on a restore run to inspect the restore output after the progress sheet is closed.

Repository and platform rules for restore:

- A Core filesystem repository can restore to the local platform only.
- A Platform filesystem repository restores on the same platform that owns that repository path.
- An S3-compatible repository can restore to local, regular agent, or edge agent platforms because the restore runs from the target platform.
- Citadel system backups are restored offline, not through the web UI.

## Alerts

Backup policies can alert on failure when **Alert on failure** is enabled. Delivery is configured through Alert Rules and Alert Channels.

## Practical Recommendations

Use S3-compatible storage for remote platforms and edge agents. It keeps the backup path independent from where Citadel Core is running and avoids requiring shared host folders.

Use filesystem repositories for simple local setups or when the backup storage is mounted directly on the platform that runs the backup.
