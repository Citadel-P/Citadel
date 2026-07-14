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

For remote regular agent and edge agent platforms, prefer an S3-compatible repository unless you explicitly want snapshots stored on that platform's host filesystem.

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

Use a new target volume name unless you intentionally want to overwrite an existing volume. Citadel rejects overwrite when the target volume is currently in use.

## Alerts

Backup policies can alert on failure when **Alert on failure** is enabled. Delivery is configured through Alert Rules and Alert Channels.

## Practical Recommendations

Use S3-compatible storage for remote platforms and edge agents. It keeps the backup path independent from where Citadel Core is running and avoids requiring shared host folders.

Use filesystem repositories for simple local setups or when the backup storage is mounted directly on the platform that runs the backup.
