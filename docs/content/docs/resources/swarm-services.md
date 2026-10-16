---
title: "Swarm Services"
description: "Configure, deploy, scale, and monitor independently managed Docker Swarm Services."
---

A Swarm Service stores the desired configuration of one application managed by
Docker Swarm. Use **Swarm Services** for independently managed Services; use a
[Stack](/docs/resources/stacks/swarm) when several Services belong to one Compose
application. Connect a [Swarm Platform](/docs/resources/platforms/docker-swarm)
before creating a Service.

## Manage A Swarm Service

Open Citadel's **Swarm Services** page, create a Service, and choose a Docker
Swarm Platform. Configure the image, task command and environment, replicated
or global mode, networks,
published ports, storage, Swarm Secrets and Configs, placement, resources,
health, restart, rolling-update policy, and Docker Service labels. Enter labels
as `KEY=value`, one per line. The `com.citadel.*` namespace is reserved for
Citadel ownership and operation metadata. Saving creates Citadel desired
state; it does not contact Docker until you select **Apply**.

The Environment editor supports Citadel variables and environment-delivered
secrets. Use `KEY` to inject a binding with the same name, or
`KEY=${OTHER_KEY}` to map another binding. Unknown references are marked in the
editor and Apply rejects them before contacting Docker. Only referenced
bindings are injected into the Service tasks. The Apply progress sheet reports
when binding resolution starts and summarizes the variables and secrets used;
secret values are never displayed.

Do not add Docker's reserved `ingress` Network to a Service. Swarm manages it
automatically for ports published in Ingress mode. Citadel excludes it from
Network selections. If a saved Service configuration contains it, remove it
before applying.

After the first Apply, use:

- **Duplicate** to choose **Configuration only** or **Configuration + data**,
  then review and create a separate, undeployed Service;
- **Apply** to send the complete saved configuration to Docker;
- **Scale** to change the replica count of a replicated Service;
- **Restart Tasks** to recreate the Service's Tasks without changing its image
  or configuration;
- **Check for updates** to compare an applied external tagged image with its
  Registry digest;
- **Delete** to remove the Docker Service and then its Citadel record.

Global Services cannot be scaled by replica count. Scheduling mode cannot be
changed after the first successful Apply; create another Service when you
need to change between replicated and global mode.

### Duplicate

**Duplicate → Configuration only** opens a prefilled form and **Review copy** reviews the
edited values before creating an undeployed Service. Automatic updates and
webhooks are disabled. The source Service is unchanged.

Configuration only does not copy volume contents. Mounts, networks, secrets, configs and placement
constraints remain references that need review before Apply. A local volume
name identifies different storage on different Swarm nodes; duplication does
not move that data or change placement to follow it. No tasks, history or access
grants are copied.

**Duplicate → Configuration + data** guides you through naming the copy,
choosing current data or existing backups, and reviewing the new volumes. Current
data needs no backup policy; confirm source consistency first. This requires one replica
pinned to one exact `node.id`. Current-data copies stay on that node and Platform.
Existing S3-compatible backups can restore to another Swarm Platform and a selected
Ready, Active node. The copied Service is pinned to the destination node; networks,
secrets and configs are matched by name and must already exist there. Global,
multi-replica and unpinned Services are blocked because a local volume name does
not identify one storage location across nodes. See
[Copy a workload with data](/docs/resources/backups#copy-a-workload-with-data).

### Backup storage

Swarm service storage policies include supported local Docker volumes and bind
mounts discovered from current Tasks on their exact owning Nodes. Host paths
require Platform write access for both configuration and execution. Review the
source preview and discovery warnings. Tmpfs, nested mounts, system/runtime paths,
and writable layers remain excluded. See [Backups](/docs/resources/backups).

Use **Backup Filter** at the right of the resource header to filter by backup status.
The resource table displays a **Backup** column. Hover over its shield to see
the same coverage card as on the resource page. Manual-only or disabled policies
show amber, even after a successful manual run. Green requires an active schedule,
a ready repository, and a successful available backup for all eligible storage.
See [Backup coverage](/docs/resources/backups#coverage-on-resource-pages) for all
status meanings and scheduling requirements.

## Scale A Replicated Service To Zero

Docker Swarm Services do not have a container-style **Stop** operation. Swarm
continuously tries to maintain each Service's desired state. To intentionally
run no Tasks while keeping a replicated Service available for later use, scale
it to zero replicas.

Scaling to zero stops all active Tasks but keeps the Service definition,
configuration, ownership, Networks, Secrets, and Config references in Swarm.
Scale it above zero to let Swarm create Tasks again. Treat this intentional
state as **Scaled to zero**: it is not a failed Service and it is not the same
as deleting the Service.

Only replicated Services support this operation. A global Service is scheduled
once on each eligible active Node and cannot be scaled by replica count. If a
global Service has no running Tasks, inspect its Nodes, placement constraints,
and Task errors instead of treating it as intentionally stopped.

Do not confuse a Service scaled to zero with an unhealthy Service whose desired
replica count is greater than zero. The latter still asks Swarm to run Tasks and
is reported as progressing, degraded, or failed according to its current Task
and rollout state.

The Config tab stores desired state. Runtime shows the current Service summary,
bounded Tasks, logs, Task terminal access, inspect data, and aggregate current
Task statistics. Bindings and Activities use the same Citadel controls as
other managed resources. Runtime identifies partial statistics when one or
more owning Nodes or current Task samples are unavailable. That warning
describes current coverage; it does not remove previously collected chart
history. Statistics are retained for seven days. Open an individual Task to
inspect that Task's current runtime data; the Service chart is the aggregate
across its logical replicas.

External tagged images support **Disabled**, **Notify only**, and **Auto
deploy** update behavior. Disabled Services are not scanned in the background,
but an authorized manual check remains available after the first Apply.
Digest-pinned and build-produced images do not support Registry update checks.
Auto deploy requires the Operational Guardrails license capability.

An external tagged Service can also expose a resource-owned webhook. GitHub,
GitLab, and Generic / CI callers use the same listener infrastructure as other
Citadel resources. A Service webhook runs the normal image check: **Notify
only** reports changed content for the Service’s task platforms, while **Auto deploy** starts the durable Apply
path only when an update exists. Generic / CI callers must send the configured
shared secret as an `Authorization: Bearer` header. Build-backed Services use
their Build Project webhook instead. This resource-owned shared secret is not a
Citadel user access token.

Scale and Restart Tasks keep the currently applied image even if its source tag
has moved. Apply resolves the current tag digest and performs the image update.

## Understand Service Ownership

The Services page distinguishes:

- **Unmanaged** Services;
- Services belonging to an **External Docker Stack**;
- **Citadel Service** observations linked to a managed Service;
- Services carrying invalid or conflicting Citadel ownership labels.

This Platform page is observed inventory, so it does not show an **Add Service**
button. Create a new managed Service from Citadel's **Swarm Services** page.

An unmanaged Service displays an unlink icon and offers **Adopt Service** from
its row, detail page, and the bottom action bar. Adoption opens a reviewed
managed-Service draft and links the saved Citadel Service to the existing
Docker Service without changing Docker. The first later Apply updates the same
Docker Service and establishes Citadel ownership labels. Services belonging to
an external Docker Stack cannot be adopted individually.

The Containers page groups manager-local Docker Stack Task containers under
their Stack namespace. The namespace row offers **Import Stack** when the
Stack is external or its previous Citadel owner no longer exists. Individual
Task containers remain read-only because Docker Swarm owns their lifecycle.

Citadel shows only current Swarm runtime containers in this view. Docker keeps
exited Task containers for diagnostic history, so Docker Desktop may show more
containers than Citadel when automatic pruning is disabled or when the
containers are local to a worker Node.
Citadel excludes exited, dead, and removing Swarm Task containers from Stack
membership and health calculations. The default Platform setting also deletes
bounded batches of that history from the connected manager and covered workers
during container synchronization.
Stopped Docker Standalone and Compose containers remain visible because they
are current workloads rather than immutable Swarm Task history.

Observed Docker labels never grant Citadel write ownership by themselves. When
valid Citadel Service or Stack labels reference an owner that no longer exists
in the current database, Citadel reports the workload as recoverable and offers
the normal reviewed **Adopt Service** or **Import Stack** flow. The action checks
the referenced owner again before saving. Invalid, conflicting, mixed, or
still-owned metadata remains read-only. The first later Apply replaces the old
ownership labels with the new Citadel owner. Stack-owned Services remain
read-only from Platform inventory; change their desired state through the
owning Stack's **Config** tab and **Deploy** or **Redeploy** action.

## View Service And Task Logs

Open a Service or Task detail page to view its latest Docker log tail. Log
access for inventory resources requires Platform Read plus Logs. A managed
Service requires Service Read plus Service Logs and visibility of its Platform.
The Platform must be online.

The current viewer is a snapshot, not a follow stream. Citadel requests at most
200 lines and materializes at most 1 MiB for one response. It closes the Docker
response stream on completion, cancellation, or failure. A truncation notice
appears when the byte limit is reached.

Service logs use the connected manager's Docker Service logs endpoint. Task
logs use the running Task's container on its owning Node and require current
node coverage. The Docker logging driver must support the corresponding log
read operation. If retrieval fails, use the logging driver's external destination.

## Open A Task Terminal

Open a running Task and select **Terminal**. Terminal access requires Platform
Read plus the Terminal permission. Docker exec is node-local, so Citadel opens
the terminal through the exact owning Node data source. A Task on an uncovered,
offline, stale, or unsupported Node remains visible but cannot open a terminal.

The same Task selector is available in a managed Service's **Runtime** tab.
Running Tasks on covered Nodes can be selected; Tasks without a usable owning
Node data source are shown but disabled.

## Permissions

Managed Services require visibility of their Platform, plus:

- Swarm Service Write to create or edit desired state.
- Swarm Service Read plus Apply to create, update, or restart Tasks.
- Swarm Service Write plus Apply to scale a replicated Service.
- Swarm Service Execute to delete a managed Service.
- Swarm Service Read plus Logs to read managed Service logs.

Task terminals require Platform Read plus Terminal and current coverage of the
owning Node. The API enforces permissions for each operation.

## Troubleshooting

For connection and inventory problems, see [Swarm Platform troubleshooting](/docs/resources/platforms/docker-swarm#troubleshooting).
For unavailable Task logs, statistics, or terminals, check
[Cluster node coverage](/docs/resources/platforms/docker-swarm#understand-cluster-node-coverage).

Manage Task lifecycle through its Service or Stack. Citadel does not offer
direct start, stop, or restart actions on individual Swarm Tasks.
