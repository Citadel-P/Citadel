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

## Choose a time range

The shared History dropdown keeps all charts on the same timeline. **1 hour** is
selected initially. Quick ranges include **15 minutes**, **6 hours**,
**24 hours**, **48 hours**, **7 days**, **Today**, and **Yesterday**.

Chart times use your browser's timezone. Relative ranges and Today follow
incoming live samples; Yesterday stays fixed. Historical summaries use the last
sample in the selected range.
Charts use straight segments and leave gaps where readings are missing. The Y
axis shows percentages or byte units. Metrics are retained for seven days; a new
or disconnected resource may have less history.

History uses averages: 10-second buckets up to one hour, 30-second buckets up
to six hours, one-minute buckets up to one day, five-minute buckets up to three
days, and 15-minute buckets for longer ranges. These averages can smooth brief
spikes; narrow the range for finer detail.

Statistics API endpoints accept `minutes=15|60|360|1440|10080`, or inclusive Unix
second bounds `from` and `to`. Omitting `to` uses the current time. Existing
`hours=24|48|72` requests remain supported, and omitting all range parameters
keeps the API's 24-hour default. Do not combine range formats.

## History across deployments

Deployment statistics follow the Deployment ID across container replacements.
Stack charts group history by Compose service within the Stack, and Swarm
service history survives task replacement and scaling. Replacements in the same
known replica slot share a time bucket; separate replicas contribute to the
service total. Unknown replica identities stay separate.

Workload samples are retained for seven days independently of current container
inventory. History remains readable during a replacement when no container is
present. A newly created workload starts its own history even if it reuses a name.
Individual container charts still describe only that container instance.
Missing observations remain gaps.

Compose replica continuity uses the replica number reported by Docker. Samples
without that identity keep their original instance identity. Live Stack totals
wait for readings from every running replica in the time bucket; partial live
readings do not replace a complete saved total.

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
Docker volume, bind mount, stack, deployment, or managed Swarm Service. Citadel control-plane backups are
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
operations continue. Include the mount when configuring Core and Platform
Agent containers. Build-pool-only Edge Agents do not need it.

Custom packaging can change the in-container mount location with:

```text
CITADEL_HOST_ROOT=/host
```

The environment value and actual bind target must match.

For ZFS, Citadel reports the mounted dataset or filesystem containing Docker's
data root, not whole-pool allocation.
