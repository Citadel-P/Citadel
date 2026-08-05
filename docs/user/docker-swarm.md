# Docker Swarm

Docker Swarm support is included with Citadel and does not require a feature
flag. The current release provides a read-only cluster experience: Citadel can
register an existing manager, show persisted cluster inventory, inspect
resources, and retrieve bounded Service and Task logs.

Citadel does not yet create, update, import, or delete Swarm resources, and it
does not yet Apply Deployments or Stacks to Swarm. Those actions remain blocked
until their transactional recovery paths are implemented.

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
  and ownership classification.
- **Tasks** shows current and recent scheduler attempts, including Service,
  Node, desired/current state, image, and Docker error details.
- **Networks** shows Swarm-scoped Network metadata and attached Services.
- **Secrets** shows Secret metadata only. Docker never returns stored Secret
  values.
- **Configs** shows Config metadata and attached Services. Config content is
  not exposed in the current read-only milestone.

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

Each resource includes its last observation time. Treat stale data as
diagnostic only; do not assume it is safe to act on outside Citadel.

Citadel uses Docker daemon events for prompt refreshes after Service, Node,
Network, Secret, or Config changes. A bounded reconciliation also runs every
30 minutes to recover from missed events, reconnects, and partial event data.
Multiple events for one Platform are coalesced so they do not create an
unbounded queue of refresh jobs.

## Understand Service Ownership

The Services page distinguishes:

- **Unmanaged** Services;
- Services belonging to an **External Docker Stack**;
- Services carrying stale Citadel ownership labels with no verified Citadel
  owner.

Observed Docker labels never grant Citadel write ownership by themselves. A
Service with unverified Citadel labels is kept read-only and displays an
orphaned-metadata diagnostic. Citadel Deployment and Stack ownership will only
become writable after those aggregates have durable Swarm runtime links in the
future workload milestone.

## View Service And Task Logs

Open a Service or Task detail page to view its latest Docker log tail. Log
access requires Platform Read plus the Logs permission, and the Platform must
be online.

The current viewer is a snapshot, not a follow stream. Citadel requests at most
200 lines and materializes at most 1 MiB for one response. It closes the Docker
response stream on completion, cancellation, or failure. A truncation notice
appears when the byte limit is reached.

Docker Service and Task logs are only available for logging drivers supported
by Docker's Service/Task logs endpoints. If retrieval fails, use the logging
driver's external destination.

## Permissions

- Platform Read: view the Swarm overview and persisted inventory.
- Platform Read plus Logs: retrieve Service and Task logs.
- Platform Write: register or edit the Platform connection.

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

The current read-only Swarm milestone does not provide:

- Swarm initialization, join/leave, token, CA, unlock, or quorum management;
- Node, Service, Network, Secret, Config, or Task mutations;
- Swarm Deployment or Stack create/Apply/delete operations;
- Service or Docker Stack import/adoption;
- cluster-wide image distribution, volume semantics, backup, or restore;
- live-follow Service or Task logs;
- automatic failover between manager endpoints.

The implementation roadmap and safety rules for these future milestones live
in the internal Docker Swarm v1 specification.
