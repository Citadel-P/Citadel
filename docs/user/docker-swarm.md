# Docker Swarm

Docker Swarm support is included with Citadel and does not require a feature
flag. Citadel can register an existing manager, show persisted cluster
inventory, inspect resources, retrieve bounded Service and Task logs, manage
standalone Swarm Secrets and Configs, and create first-class managed Swarm
Services.

Task and Node scheduler mutations remain unavailable. Citadel Deployments stay
Docker Standalone workloads, and Citadel does not yet Apply Stacks to Swarm.

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

## Use The Swarm Pages

Selecting a Swarm Platform opens its Swarm navigation:

- **Overview** shows connection/freshness health and Node, manager, Service,
  and running/desired Task counts.
- **Nodes** shows manager/worker role, readiness, availability, reachability,
  Engine details, labels, and Task counts.
- **Services** shows mode, image, replica counts, update state, ports, labels,
  and ownership classification. Select **Add** to create a Citadel-managed
  Service on the current Swarm.
- **Tasks** shows current and recent scheduler attempts, including Service,
  Node, desired/current state, image, and Docker error details. A running Task
  can open a terminal when its container is on the connected manager.
- **Networks** shows Swarm-scoped Network metadata and attached Services.
- **Secrets** uses one Inspect tab for details, referencing Services, and
  labels. Docker never returns stored Secret values, so the Edit dialog exposes
  labels only. Citadel can create Secrets, edit their labels, and delete unused
  Secrets.
- **Configs** uses one Inspect tab for details, referencing Services, and
  labels. The Edit dialog loads current content directly from Docker into a
  read-only editor and keeps labels editable. Citadel can create Configs, edit
  their labels, and delete unused Configs.

Containers, Images, and Volumes visible through the same manager remain
manager-local Docker resources. They are not cluster inventory. An Image on
the manager may be absent from a worker, and same-named local Volumes on two
Nodes may contain different data.

## Understand Health And Freshness

Citadel stores the last successful Swarm observation so inventory remains
available when the manager disconnects.

- **Healthy** means the manager is online, exposes Swarm control, and the
  persisted inventory is current.
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

Open a Swarm Platform, select **Services**, then select **Add**. Configure the
image, task command and environment, replicated or global mode, networks,
published ports, storage, Swarm Secrets and Configs, placement, resources,
health, restart, and rolling-update policy. Saving creates Citadel desired
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

The Config tab stores desired state. Runtime shows the current Service summary,
bounded Tasks, logs, Task terminal access, and inspect data. Bindings and
Activities use the same Citadel controls as other managed resources. Service
statistics are not shown because a manager cannot truthfully provide aggregate
worker-node statistics.

External tagged images support **Disabled**, **Notify only**, and **Auto
deploy** update behavior. Disabled Services are not scanned in the background,
but an authorized manual check remains available after the first Deploy.
Digest-pinned and build-produced images do not support Registry update checks.
Auto deploy requires the Operational Guardrails license capability.

Scale and Restart Tasks keep the currently applied image even if its source tag
has moved. Apply resolves the current tag digest and performs the image update.

## Understand Service Ownership

The Services page distinguishes:

- **Unmanaged** Services;
- Services belonging to an **External Docker Stack**;
- **Citadel Service** observations linked to a managed Service;
- Services carrying stale Citadel ownership labels with no verified Citadel
  owner.

Observed Docker labels never grant Citadel write ownership by themselves. A
Service with unverified Citadel labels is kept read-only and displays an
orphaned-metadata diagnostic. Only a verified first-class Citadel Service can
be changed through the managed Service actions. Stack-owned Services remain
read-only until Swarm Stack management is implemented.

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
Read plus the Terminal permission. Docker exec is node-local, so Citadel can
open the terminal only when the Task is running on the manager connected to the
Platform. Tasks on another manager or worker remain visible, but their terminal
cannot be opened through this connection.

The same Task selector is available in a managed Service's **Runtime** tab.
Tasks on the connected manager can be selected; running Tasks on worker nodes
are shown but disabled.

## Permissions

- Platform Read: view the Swarm overview and persisted inventory.
- Platform Read plus Inspect: inspect live Node, Service, and Task data and
  load Config content into the read-only editor.
- Platform Read plus Logs: retrieve Service and Task logs.
- Platform Read plus Terminal: open a terminal for a running Task on the
  connected manager.
- Platform Write: register or edit the Platform connection and create, edit
  labels on, or delete unused Secrets and Configs.
- Swarm Service Write plus Platform visibility: create or edit managed Service
  desired state.
- Swarm Service Read plus Apply and Platform visibility: Deploy, Apply, or
  restart a managed Service's Tasks.
- Swarm Service Write plus Apply and Platform visibility: Scale a replicated
  managed Service.
- Swarm Service Execute plus Platform visibility: delete a managed Service.

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
- Node, Network, or Task mutations;
- in-place Secret value or Config data replacement;
- Swarm Deployment or Stack create/Apply/delete operations;
- unmanaged Service or Docker Stack import/adoption;
- cluster-wide image distribution, volume semantics, backup, or restore;
- live-follow Service or Task logs;
- automatic failover between manager endpoints.

The implementation roadmap and safety rules for these future milestones live
in the internal Docker Swarm v1 specification.
