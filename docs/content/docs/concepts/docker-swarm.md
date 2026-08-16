---
title: "Docker Swarm"
description: "Understand Citadel Swarm platforms, node coverage, Services, Stacks, and cluster behavior."
---

Docker Swarm support is included with Citadel and does not require a feature
flag. Citadel can register an existing manager, show persisted cluster
inventory, inspect resources, retrieve bounded Service and Task logs, manage
standalone Swarm Secrets and Configs, and create first-class managed Swarm
Services. Operators can also change Node availability and labels, and create or
safely delete overlay Networks. Citadel can also save, validate, and apply
Docker Swarm Stacks from the Web Editor or a Git source.

Task mutations and destructive Node administration remain unavailable. Citadel
Deployments stay Docker Standalone workloads. Docker Swarm Stacks support
reviewed import, Apply, safe rollback, and ownership-checked deletion.

Citadel reaches the connected manager's node-local Docker resources by
default. The optional node data plane extends current Container, Image,
Volume, and local Network inventory to covered workers and routes Task runtime
plus Volume browsing and bounded Volume backup/restore to the owning Node.
General-purpose destructive Image, Volume, and local Network actions remain
unavailable. See [[Backups](/docs/guides/backups)](/docs/guides/backups#docker-swarm) for the supported
backup sources and safety limits.

## Before You Begin

Citadel connects to an existing Swarm. It does not initialize, join, leave, or
repair a cluster.

You need:

- an active Docker Swarm;
- a reachable manager node;
- Docker Engine API v1.41 or newer;
- Platform Write permission to register the Platform;
- a Local, regular Agent, or Edge Agent connection to the manager.

Do not connect a Swarm Platform to a worker. Workers run Tasks but cannot serve
the manager API used for cluster inventory.

## Add A Swarm Platform

Open **Platforms**, select **Add**, and choose **Docker Swarm** as the Platform
type.

For a Local or regular Agent connection:

1. Enter the connection details.
2. Select **Test connection**.
3. Review the detected Cluster ID, manager status, and Docker version.
4. Save the Platform.

For an Edge Agent connection:

1. Save the Platform placeholder.
2. Enroll the Edge Agent on an active Swarm manager.
3. Wait for Citadel to validate the daemon and mark the Platform online.

Citadel rejects a Standalone daemon, a Swarm worker, an unsupported API
version, or a Cluster ID already registered by another Platform. The Platform
type cannot be changed after creation. A replacement manager endpoint must
report the same Cluster ID.

## Understand Cluster Node Coverage

Swarm cluster resources and node-local Docker resources come from different
Docker APIs:

- Nodes, Services, Tasks, Secrets, Configs, and overlay Networks are available
  through an active manager;
- Containers, live container statistics, exec sessions, local Images, local
  Volumes, and local Networks belong to the Node running or storing them.

Without node agents, the **Containers**, **Images**, **Volumes**, and local
**Networks** views describe the connected manager rather than every Node. A
Task scheduled on a worker remains visible, but its node-local runtime
operations are unavailable.

Use **Cluster node coverage** on the Platform page to install the node data
plane. **Install node agents** creates a Citadel System global Docker Service
on eligible Linux Nodes not already covered by the manager connection. Each
satellite runs the existing Agent in a restricted outbound `swarm-node`
profile and opens no inbound management port. The Platform remains one Swarm
Platform; its manager connection can be Local, regular Agent, or Edge Agent.

With coverage installed, Citadel aggregates current Containers, Images,
Volumes, and local Networks from covered Nodes. Each local resource keeps its
Node identity: same-named Volumes on different Nodes remain separate. Image,
Volume, and local Network inspect plus Volume browse/download route to the
selected owning Node. Task inspect, logs, statistics, lifecycle operations,
and Terminal also route to the exact owning Node. Service statistics sum the
available current Task samples and explicitly report partial coverage when a
Node or Task sample is missing. Service chart history is retained across
routine Task replacement, rollout, restart, and cleanup of stopped Task
containers. An offline satellite keeps its last-known node-local projections
marked stale; it does not make the manager or whole Platform offline.

An active supported worker remains part of expected coverage while it is down,
so coverage becomes **Partial** instead of shrinking the total. A deliberately
paused or drained Node is shown as unschedulable and is not counted as a missing
Agent target until it becomes active again.

The System Service is infrastructure owned by Citadel, not a user Stack or
managed Service. It mounts each Node's Docker socket read-write, which gives it
daemon-level access. Installation, repair, upgrade, and removal are therefore
explicit privileged actions and never happen when you only Test or Save a
Platform. A single-Node Swarm is already covered by its manager and does not
need a satellite Service.

Do not reuse the regular Platform Agent or Edge Agent enrollment token on every
Node. Citadel creates a separate, expiring, cluster-scoped bootstrap credential
for satellite enrollment. Use **Repair coverage** when a new eligible Node
joins after that enrollment window expires. Upgrade and removal are also
explicit Platform actions. Requests and final outcomes appear in the Platform's
activity history; bootstrap credentials are never included there.

## Clean Up Historical Task Containers

The Swarm Platform **Config** tab includes **Prune historical task containers**.
It is enabled by default. During container synchronization, Citadel deletes a
bounded batch of exited or dead Swarm Task containers on the connected manager
and every covered worker. Running Tasks and stopped Docker Standalone or
Compose containers are never removed by this setting.

Disable the setting if you need Docker Desktop or the Docker CLI to retain the
manager's stopped Task containers for manual inspection. Citadel still hides
that terminal history from current Stack membership, counts, and health.
Worker history is pruned only while that worker has a usable node data source.

## Use The Swarm Pages

Selecting a Swarm Platform opens its Swarm navigation:

- **Overview** shows connection/freshness health, manager quorum, and Node,
  manager, Service, and running/desired Task counts. The Platforms list also
  shows quorum separately from the manager connection indicator.
- **Nodes** shows manager/worker role, readiness, availability, reachability,
  Engine details, labels, and Task counts.
- **Services** shows mode, image, replica counts, update state, ports, labels,
  and ownership classification. Eligible unmanaged Services can be adopted;
  create new managed Services from Citadel's main **Services** page.
- **Tasks** shows current and recent scheduler attempts, including Service,
  Node, desired/current state, image, and Docker error details. A running Task
  can use inspect, logs, statistics, lifecycle actions, and Terminal when its
  owning Node is covered.
- **Networks** shows Swarm-scoped Network metadata and attached Services.
- **Secrets** uses one Inspect tab for details, referencing Services, and
  labels. Docker never returns stored Secret values, so the Edit dialog exposes
  labels only. Citadel can create Secrets, edit their labels, and delete unused
  Secrets.
- **Configs** uses one Inspect tab for details, referencing Services, and
  labels. The Edit dialog loads current content directly from Docker into a
  read-only editor and keeps labels editable. Citadel can create Configs, edit
  their labels, and delete unused Configs.

Images, Volumes, and local Networks are Node-local rather than cluster
inventory. Their tables show the owning Node and can contain multiple entries
with the same local name. Citadel does not infer that an Image present on one
Node exists on another. Browsing a Volume targets its exact Node; destructive
Image, Volume, and local Network actions remain disabled. Container adoption
remains available only on Docker Standalone Platforms. On a Swarm manager,
Citadel can import an existing Docker Stack namespace or a regular Docker
Compose project. A Docker Stack is imported as one complete namespace from
**Services** or its parent row
in **Containers**. A regular Compose project is imported from its parent row in
**Containers** and converted on its first explicit Apply. Swarm Task containers
never expose individual adoption actions.

Importing a regular Compose project does not mutate Docker. Citadel validates
the authoritative Web Editor or Git source against both the running project and
Swarm compatibility rules. On the first Apply, Citadel stops the Compose
project without deleting named volumes, then deploys the reviewed source as a
native Docker Stack under the same project name. This transition has downtime.
If Compose shutdown is incomplete or Docker rejects the `docker stack deploy`
command, Citadel restores the Compose project from the reviewed source and
reports both outcomes in the progress sheet. If Docker accepts the command but
the later rollout fails, the workload remains a Swarm Stack so its failure can
be inspected and corrected.

## Create And Apply A Swarm Stack

Open **Stacks**, select **Add**, and choose a Docker Swarm Platform. The Stack
form removes Docker Standalone-only controls such as container drift repair,
pre-deploy and post-deploy commands, and destroy-before-deploy.

The Web Editor switches validation with the selected Platform. Docker
Standalone Stacks use the current Compose Specification; Swarm Stacks use the
Docker CLI Compose v3.13 syntax plus Citadel compatibility diagnostics. Editor
squiggles provide early feedback, but saving still runs the authoritative
server-side compatibility check.

Saving runs a compatibility check first. Errors identify Compose fields that
Swarm Stacks cannot support, such as `container_name`, standalone restart
settings, or Citadel ownership labels. Portability warnings call out settings
that can work but depend on every eligible Node, such as bind mounts, local
Volumes, and fixed Host-mode ports.

The selected orchestration type is locked after creation. A never-applied draft
can move to another Platform of the same type. Moving between Docker Standalone
and Docker Swarm requires duplicating the Stack and reviewing the new draft.

Select **Apply** after reviewing a valid draft. The progress sheet reports
preflight, Docker CLI output, and rollout observation. Citadel supports this
flow through Local, regular Agent, and Edge Agent connections.

Swarm owns the lifecycle of a Stack's Tasks, so Swarm Stacks do not show the
container-oriented **Start**, **Stop**, **Pause**, **Resume**, or **Reconcile
drift** actions used by Docker Standalone Stacks. Manually changing an
individual Task container is temporary because Swarm recreates it to restore
the Service's desired state. Change the Compose configuration, including
`deploy.replicas`, then use **Apply** or **Redeploy** instead.

Docker accepting the Stack definition does not by itself mark the release
healthy. Citadel observes the Stack Services and their Tasks and reports a
failed Task error when the rollout cannot converge. A deployment process is
limited to five minutes and the initial rollout observation is limited to two
minutes. If the connection or progress stream is interrupted after dispatch,
the release remains **Unknown** or **Timed out** and background Swarm
reconciliation can recover it after Docker's state becomes observable.

Stack-owned Services remain visible from the Swarm Platform's **Services** and
**Tasks** pages. After the first Apply, the Stack's **Services** tab also shows
only the Services owned by that Stack, with their current Tasks as expandable
rows. The table follows the live Swarm inventory stream. Select a Service to
use the bounded log viewer, inspect its Docker definition, or open a terminal
for a running Task on a covered Node. Citadel does not show aggregate Stack
statistics.

Rollback reapplies a selected healthy release through the same preflight and
convergence checks. Each successful release records the immutable Docker Secret
and Config IDs it used. By default, the newest 10 healthy releases retain those
resources for rollback; administrators can configure a bounded value from 1 to
50 with `SwarmStacks:RetainedRollbackReleases`. Citadel verifies every retained ID before changing Docker and
reuses mounted Secrets without resolving a secret provider's current value. If
a required resource has been removed or is outside the retention window,
rollback stops before mutation with an explicit error.

Deleting a Swarm Stack first verifies namespace ownership, removes its owned
Services, waits for their absence, and then removes only unreferenced owned
Networks, Secrets, and Configs. Named and local Volumes are retained. The Stack
record is removed only after runtime cleanup is confirmed, so a retry resumes
from Docker's observed state.

Mounted-file Citadel bindings must target one filename under `/run/secrets/`.
Citadel delivers them as Docker Swarm Secrets and removes the temporary
plaintext and generated Compose override after every Apply outcome. After a
healthy Apply, Citadel removes only older, unreferenced Secret and Config
versions outside the configured rollback window. Failed, timed-out, unknown, or
partially observed operations never trigger this cleanup.

For Agent and Edge Agent connections, a Git source snapshot is rejected before
dispatch when it exceeds 512 files or 12 MiB. Reduce the Stack deployment
source or move large application data out of the repository before retrying.

## Understand Health And Freshness

Citadel stores the last successful Swarm observation so inventory remains
available when the manager disconnects.

Connection and quorum are different signals. Connection reports whether
Citadel can reach the configured manager. Quorum reports whether the Swarm
control plane has an elected leader and enough reachable managers for a Raft
majority. Workers do not count toward quorum.

- **Quorum healthy** means a leader exists and all known managers are
  reachable.
- **Quorum degraded** means a majority is still reachable, but at least one
  manager is unavailable.
- **Quorum lost** means no leader exists or fewer than
  `floor(manager count / 2) + 1` managers are reachable.
- **Quorum unknown** means Citadel cannot safely evaluate current manager
  membership because the Platform is offline or manager inventory is stale.

A healthy single-manager Swarm has quorum but no manager failure tolerance.
Citadel reports that limitation in the quorum tooltip.

- **Healthy** means the manager is online, exposes Swarm control, and the
  persisted inventory and quorum are healthy.
- **Stale** means the most recent refresh failed. The displayed inventory is
  last-known state and may no longer match Docker.
- **Degraded** means the endpoint is online but does not currently expose
  usable manager control, or Docker reported a sanitized manager error.
- **Offline** means Citadel cannot currently reach the Platform.

Treat stale data as diagnostic only; do not assume it is safe to act on
outside Citadel.

Citadel uses Docker daemon events for prompt refreshes after Service, Node,
Network, Secret, or Config changes. A bounded reconciliation also runs every
30 minutes to recover from missed events, reconnects, and partial event data.
Multiple events for one Platform are coalesced so they do not create an
unbounded queue of refresh jobs.

## Manage Nodes

Platform Write permission allows you to edit a Node's availability and labels.
Open a Node and use the grouped scheduling actions:

- **Active** to allow the scheduler to assign new Tasks;
- **Pause** to prevent new assignments without moving existing Tasks;
- **Drain** to move eligible Service Tasks away from the Node and prevent new
  assignments.

Use the separate **Edit** action to change Node labels. The current labels are
shown at the bottom of the Node page.

You can also select one or more Nodes in the Nodes table and set their
availability from the bottom action bar. Draining requires confirmation. Node
indicators remain green while ready and active, turn orange while scheduling is
paused, and turn gray when drained; a down or disconnected Node remains red.

Citadel preserves the Node's name and manager/worker role. It rejects an edit
if the Node observation is stale or Docker's Node version changed after the
form was opened. Citadel does not promote, demote, join, remove, or leave Nodes.

## Manage Overlay Networks

Use the Swarm Platform's **Networks** page to create an overlay Network. The
form supports attachable and internal modes, IPAM settings, IPv6 where Docker
supports it, driver options, and labels. Swarm scope requires the overlay
driver and a Docker Swarm Platform.

Deleting a Network requires Platform Execute permission. Citadel refuses to
delete Docker system Networks, Stack-owned Networks, Networks with connected
Containers, Networks referenced by a Service, or Networks whose inventory is
stale. Docker performs the final check if usage changes concurrently.

## Manage Secrets And Configs

Platform Write permission is required for all mutations. Select **Add** on the
Secrets or Configs page to create a resource. Citadel does not store a copy of
a Secret's value, and Docker does not return it after creation. Keep a secure
copy if you may need the value later.

On a Config page, **Edit** opens a dialog and loads the content from the
connected Swarm manager only when the user has Platform Inspect permission.
The editor is read-only; Citadel does not persist the content in its inventory
projection or publish it through SignalR. On a Secret page, **Edit** opens the
same compact dialog but shows labels only because Docker does not disclose the
stored value.

The indicator beside each resource name reports usage:

- green means one or more observed Services reference the resource;
- gray means the resource is unused.

Use the row checkbox to select one or more resources. **Delete** is enabled only
when every selected resource is unused and its observation is current. Citadel
checks the same rule again in the API immediately before deletion. Docker is
the final concurrency guard: if a Service starts using a resource between the
check and delete, Docker rejects the operation and Citadel reconciles the
result.

Docker supports changing Secret and Config labels in place. Editing the Secret
value or Config data is not available because payload changes require a new
version and coordinated Service replacement. Delete and recreate manually only
after reviewing every consumer.

## Manage A Swarm Service

Open Citadel's main **Services** page, select **Add**, then choose a Docker
Swarm Platform. Configure the image, task command and environment, replicated
or global mode, networks,
published ports, storage, Swarm Secrets and Configs, placement, resources,
health, restart, rolling-update policy, and Docker Service labels. Enter labels
as `KEY=value`, one per line. The `com.citadel.*` namespace is reserved for
Citadel ownership and operation metadata. Saving creates Citadel desired
state; it does not contact Docker until you select **Deploy**.

The Environment editor supports Citadel variables and environment-delivered
secrets. Use `KEY` to inject a binding with the same name, or
`KEY=${OTHER_KEY}` to map another binding. Unknown references are marked in the
editor and Apply rejects them before contacting Docker. Only referenced
bindings are injected into the Service tasks. The Apply progress sheet reports
when binding resolution starts and summarizes the variables and secrets used;
secret values are never displayed.

Do not add Docker's reserved `ingress` Network to a Service. Swarm manages it
automatically for ports published in Ingress mode. Citadel excludes it from
new Network selections; if an older saved Service contains it, remove it from
the configuration before applying.

After the first Deploy, use:

- **Duplicate Config** to open a new Service form prefilled from the current
  configuration; review and save it as a separate Service. Service-scoped
  variables and secret references are copied when it is saved, while global
  bindings remain inherited;
- **Apply** to send the complete saved configuration to Docker;
- **Scale** to change the replica count of a replicated Service;
- **Restart Tasks** to recreate the Service's Tasks without changing its image
  or configuration;
- **Check for updates** to compare an applied external tagged image with its
  Registry digest;
- **Delete** to remove the Docker Service and then its Citadel record.

Global Services cannot be scaled by replica count. Scheduling mode cannot be
changed after the first successful Deploy; create another Service when you
need to change between replicated and global mode.

### Scale A Replicated Service To Zero

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
but an authorized manual check remains available after the first Deploy.
Digest-pinned and build-produced images do not support Registry update checks.
Auto deploy requires the Operational Guardrails license capability.

An external tagged Service can also expose a resource-owned webhook. GitHub,
GitLab, and Generic / CI callers use the same listener infrastructure as other
Citadel resources. A Service webhook runs the normal digest check: **Notify
only** reports a changed digest, while **Auto deploy** starts the durable Apply
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
button. Create a new managed Service from Citadel's main **Services** page.

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
owning Stack's Config and Apply workflow.

## View Service And Task Logs

Open a Service or Task detail page to view its latest Docker log tail. Log
access for inventory resources requires Platform Read plus Logs. A managed
Service requires Service Read plus Service Logs and visibility of its Platform.
The Platform must be online.

The current viewer is a snapshot, not a follow stream. Citadel requests at most
200 lines and materializes at most 1 MiB for one response. It closes the Docker
response stream on completion, cancellation, or failure. A truncation notice
appears when the byte limit is reached.

Docker Service and Task logs are only available for logging drivers supported
by Docker's Service/Task logs endpoints. If retrieval fails, use the logging
driver's external destination.

## Open A Task Terminal

Open a running Task and select **Terminal**. Terminal access requires Platform
Read plus the Terminal permission. Docker exec is node-local, so Citadel opens
the terminal through the exact owning Node data source. A Task on an uncovered,
offline, stale, or unsupported Node remains visible but cannot open a terminal.

The same Task selector is available in a managed Service's **Runtime** tab.
Running Tasks on covered Nodes can be selected; Tasks without a usable owning
Node data source are shown but disabled.

## Permissions

- Platform Read: view the Swarm overview and persisted inventory.
- Platform Read plus Inspect: inspect live Node, Service, and Task data and
  load Config content into the read-only editor.
- Platform Read plus Logs: retrieve Service and Task logs.
- Platform Read plus Terminal: open a terminal for a running Task on a covered
  Node.
- Platform Execute plus Manage Node Agents: install, repair, upgrade, or
  remove the privileged node data plane.
- Platform Write: register or edit the Platform connection and create, edit
  labels on, or delete unused Secrets and Configs.
- Swarm Service Write plus Platform visibility: create or edit managed Service
  desired state.
- Swarm Service Read plus Apply and Platform visibility: Deploy, Apply, or
  restart a managed Service's Tasks.
- Swarm Service Write plus Apply and Platform visibility: Scale a replicated
  managed Service.
- Swarm Service Execute plus Platform visibility: delete a managed Service.
- Stack Write plus Platform Read and Inspect: import an external Docker Stack
  namespace without changing Docker.
- Existing Stack Apply, Releases, and Execute permissions plus Platform
  visibility: Apply, rollback, or delete a managed Swarm Stack.

All checks are enforced by the API. Hidden or disabled UI controls are not the
authorization boundary.

## Troubleshooting

### The node is not an active Swarm manager

The selected endpoint is inactive or is a worker. Point the Local connection or
Agent at an active manager and test again.

### This cluster is already registered

Another Citadel Platform already uses the same Docker Cluster ID. Use that
Platform or update its manager endpoint.

### Inventory is stale or the manager is offline

Workloads may continue on workers while Citadel cannot manage or refresh the
cluster. Restore access to a manager in the same cluster. The next relevant
daemon event or bounded reconciliation repairs the persisted projection.

### Logs are unavailable

Confirm the Platform is online, your role includes Logs permission, the
Service or Task still exists on the selected Platform, and the Docker logging
driver supports retrieval.

## Current Limits

The current Swarm milestone does not provide:

- Swarm initialization, join/leave, token, CA, unlock, or quorum management;
- Node role changes/removal and Task mutations;
- overlay Network updates after creation;
- in-place Secret value or Config data replacement;
- Citadel Deployments on Swarm Platforms;
- rollback of a release whose required versioned Secret or Config is no longer
  retained;
- cluster-wide Image distribution or general-purpose destructive node-local
  Image/Volume/local Network mutations;
- live-follow Service or Task logs;
- automatic failover between manager endpoints.

The implementation roadmap and safety rules for these future milestones live
in the internal Docker Swarm v1 specification.


