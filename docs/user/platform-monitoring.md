# Platform Monitoring

Citadel reports CPU, memory, network, and Docker storage filesystem usage for
Local, regular Agent, and Platform Edge Agent connections.

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
