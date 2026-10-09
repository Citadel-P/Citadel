---
title: "Platform monitoring"
description: "Monitor platform health, capacity, statistics, and stale data."
---

Citadel reports CPU, memory, network, and Docker storage filesystem usage for
Local, regular Agent, and Platform Edge Agent connections.

Open the Platform's **Stats** tab for host history, or a
[container's **Stats** tab](/docs/resources/docker/containers#inspect-a-container)
for an individual workload. If the header reports interrupted live updates,
displayed information may be outdated; follow
[connection troubleshooting](/docs/operations/troubleshooting#live-updates-stop).

## Investigate resource pressure

1. Open the affected Platform and check that it is connected and receiving
   current readings. An old chart cannot confirm recovery.
2. Use **Stats** to see whether the increase is sustained and when it started.
   Compare that time with recent deployments, builds, and other operations.
3. For CPU or memory pressure, inspect the busy containers' **Stats** and logs.
   For disk pressure, check the filesystem described below before removing data.
4. After addressing the cause, confirm fresh readings and application health,
   then review the alert's [resolution state](/docs/resources/alert-rules#respond-to-an-alert).

Assign notification channels to the built-in capacity rules under
**Settings → Alert Rules** to receive future alerts. Custom thresholds and
required-match counts require **Advanced Alerting**.

## Container memory

The Containers table's **Memory** column shows usage excluding reported file
cache, followed by the memory limit. Container and Stack Stats show that same
**Usage** value and **Cache** separately:

```text
Usage = raw container memory − reported file cache
Cache = reported file cache
```

Usage may be lower than Linux `docker stats`, which excludes only inactive file
cache. If Docker does not report cache, no cache is subtracted.

## Disk Usage

Disk Usage is the capacity of the host filesystem containing Docker's
configured data root. It is not the sum of image, container, volume, and build
cache sizes, and it does not represent reclaimable Docker space.

Citadel shows current used and total capacity in the Platform summary and
historical percentage usage in the Platform Stats tab. Missing readings appear
as unavailable rather than zero.

Citadel includes these system rules:

| Rule | Severity | Required matches | Cooldown |
| --- | --- | ---: | ---: |
| Disk above 70% | Warning | 3 | 300 seconds |
| Disk above 90% | Critical | 3 | 300 seconds |

When both thresholds match, the Critical rule takes precedence. Configure
channels on both rules if you need notifications at both severities; cooldown
does not send periodic reminders for an open incident.

Inspect unused [images](/docs/resources/docker/images) and
[volumes](/docs/resources/docker/volumes) before deleting anything. Other files
on the same filesystem can also consume space, so Docker cleanup may not remove
the cause. Confirm that a volume's data is no longer needed before deleting it.

If disk usage is unavailable, verify the host mount below and the Agent version.
A missing reading does not mean the filesystem is empty or that a disk alert
has recovered.

## Backup Summary

The platform summary counts backup policies attached to that platform through a
Docker volume, stack, deployment, or managed Swarm Service. Citadel control-plane backups are
instance-wide, so they are shown on the main **Backups** page and are not
included in a platform's total. See
[Backups](/docs/resources/backups#backup-counts) for details.

## Container Mount

Linux Core and Platform Agent containers need this bind mount:

```text
/:/host:ro
```

The mount lets Citadel read filesystem capacity for Docker's configured data
root, including a custom data root on another mounted filesystem. It is
read-only, but it exposes host paths to the trusted Citadel process. Citadel
does not browse or transfer host files through this feature.

Removing the mount disables only disk metrics. CPU, memory, network, and Docker
operations continue. Older Agent containers must be upgraded and recreated
with the mount. Build-pool-only Edge Agents do not need it.

Custom packaging can change the in-container mount location with:

```text
CITADEL_HOST_ROOT=/host
```

The environment value and actual bind target must match.

For ZFS, Citadel reports the mounted dataset or filesystem containing Docker's
data root, not whole-pool allocation.
