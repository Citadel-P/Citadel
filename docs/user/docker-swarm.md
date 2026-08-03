# Docker Swarm

Docker Swarm support is included with Citadel and does not require a feature
flag. Citadel can currently register and validate a Swarm manager, but it does
not yet create or Apply Swarm Deployments or Stacks. Citadel rejects those
workload operations until the Swarm runtime and recovery path are available.

Docker Swarm lets Citadel manage Services distributed across multiple Docker
nodes. Use it when Docker's scheduler should place and replace workload Tasks
across a cluster.

Use Docker Standalone instead when one Docker host owns the Containers, Images,
and Volumes. A Citadel Deployment represents one workload on either Platform
type: Docker Standalone runs it as one Container, while Docker Swarm runs it as
one Service with scheduler-managed Tasks. A Stack represents a multi-workload
Compose application on either Platform type.

## Before You Begin

Citadel connects to an existing Swarm. It does not create, initialize, join, or
repair one.

You need:

- a Docker Swarm that is already active;
- an active manager node that Citadel can reach;
- Docker Engine API v1.41 or newer;
- Platform Write permission to add the Platform;
- registry-hosted images that every eligible worker can pull.

Do not connect Citadel to a worker. Workers can run Tasks but cannot serve the
cluster-management API Citadel needs.

For a remote host, install the regular or Edge Agent as described in
`docs/user/agent.md` or `docs/user/edge-agent.md`. The Agent must run on a Swarm
manager.

## Add A Swarm Platform

Open **Platforms**, select **Add**, and set **Platform type** to **Docker
Swarm**. Then follow the flow for the selected Connector.

### Local Or Agent

1. Choose **Local** for the Docker socket mounted into Citadel Core, or
   **Agent** for a regular Agent running on the manager.
2. Enter the connection details.
3. Select **Test connection**.
4. Review the detected Cluster ID, connected manager, Node/manager counts,
   leader, and negotiated Docker API version.
5. Save the Platform.

### Edge Agent

1. Choose **Edge Agent** and save the offline Platform placeholder.
2. Generate its enrollment command from the saved Platform.
3. Run the Edge Agent on an active Swarm manager and complete enrollment.
4. Wait for Citadel to validate the connected daemon.
5. Review the detected Cluster ID, connected manager, Node/manager counts,
   leader, and negotiated Docker API version.

An Edge Platform cannot be tested before it is saved and enrolled. It remains
offline and cannot run operations until the Agent proves that its Docker daemon
is an active manager.

Connection validation rejects the following. For Local/Agent this happens during
Test and Save; for Edge it happens after enrollment before the Platform becomes
operational:

- an inactive Docker daemon;
- a Swarm worker;
- a Docker Standalone/Swarm type mismatch;
- an unsupported Docker API version;
- a Cluster ID that is already registered in Citadel.

The Platform type cannot be changed after creation. If a manager endpoint is
replaced, edit the connection details. Citadel accepts the new endpoint only if
it belongs to the same Cluster ID.

## Understand The Swarm Navigation

A Swarm Platform contains:

- **Overview**: cluster, quorum, Node, Service, Task, and alert summary.
- **Nodes**: managers and workers.
- **Services**: desired workloads managed by the Swarm scheduler.
- **Tasks**: individual scheduled attempts for Services.
- **Networks**: cluster overlay Networks.
- **Secrets**: write-only sensitive data delivered to Services.
- **Configs**: non-secret files delivered to Services.
- **Deployments**: Citadel single-workload applications deployed as Swarm
  Services.
- **Stacks**: Citadel Web Editor or Git applications deployed to the Swarm.
- **Connected manager**: Containers, Images, and Volumes visible on the manager
  Citadel is connected to.

The Connected manager section is not a cluster inventory. An Image present on
the manager may be absent from a worker. A local-driver Volume with the same
name on two Nodes can contain different data.

## Monitor Cluster Health

The Overview shows:

- the connected manager and API version;
- manager count, reachable managers, leader, and quorum risk;
- ready, down, active, paused, and drained Nodes;
- healthy, progressing, degraded, failed, and unknown Services;
- desired and running Task totals;
- paused or failed updates and recent Task failures.

A Swarm needs a majority of managers to maintain quorum. Citadel warns when the
number of reachable managers is only equal to the required majority and marks
the cluster critical when there is no leader or quorum is lost.

The last known state remains visible when the manager is unavailable. Check the
**Observed** time before acting on offline data.

## Choose A Workload Platform

When adding a Deployment or Stack, select its Platform first. Citadel shows
Standalone configuration for a Docker Standalone Platform and Swarm
configuration for a Docker Swarm Platform.

Creating the resource locks that Platform type. On Edit, the Platform dropdown
shows only Platforms of the same type. Changing a Standalone workload to Swarm,
or a Swarm workload to Standalone, requires creating or duplicating a new
resource so the different runtime configuration can be reviewed explicitly.

Changing the Platform of an existing Deployment or Stack is not currently
supported, even when both Platforms have the same type. Duplicate the workload
on the target Platform, review it, and remove the original only after the new
runtime has been verified.

## Create A Swarm Deployment

Open **Deployments** and select **Add Deployment**, then choose a Docker Swarm
Platform. **Add** from the Platform's Services inventory opens this same form
with the Platform already selected.

### General

Provide a unique name and an image. Select a Citadel Registry when credentials
are required. Configure command, arguments, working directory, user, variables,
and labels only when the image defaults are not enough.

The image must be available from a registry. An image that exists only on the
connected manager is not sufficient because the scheduler may place a Task on
another Node. For that reason, **Local image** is not available for a Swarm
Deployment. Tagged external images keep Citadel's Check for updates and
automatic-update options; digest-pinned images cannot be checked for a newer
tag digest.

Citadel Build images are usable after the Build has pushed the image to a
Registry. See `docs/user/builds.md` and `docs/user/registries.md`.

### Mode

- **Replicated** runs the requested number of Tasks. It defaults to one replica.
- **Global** runs one eligible Task on every eligible active Node.

Swarm job modes may be inspected when they already exist, but cannot be created
in Citadel v1.

### Networking

Attach one or more overlay Networks.

Published ports support:

- **Ingress**: the routing mesh exposes the port on every Swarm Node and routes
  traffic to a running Task.
- **Host**: the port is opened only on a Node running the Task. A fixed Host port
  can prevent multiple Tasks from sharing a Node and can conflict with another
  workload.

Use VIP endpoint mode for the normal load-balanced path. Use DNSRR only when
the consuming application performs its own endpoint selection.

### Storage

Swarm scheduling does not make ordinary Docker volumes cluster-wide.

- A bind path must exist with suitable content and permissions on every Node
  where the Task may run.
- A local-driver named Volume is local to a Node. Rescheduling can attach a
  different Volume with the same name.
- Use placement constraints when a stateful workload must remain on Nodes with
  the required storage.

Citadel v1 does not provide cluster-wide backup or restore for Swarm local
volumes.

### Scheduling And Resources

Use placement constraints to require Node properties or labels. For example:

```text
node.labels.storage == ssd
node.role == worker
```

Reservations tell the scheduler what a Task needs. Limits cap what a running
Task may consume. An impossible constraint or reservation leaves Tasks pending;
the Task error explains why no suitable Node was found.

### Restart And Rollout

The default rolling update changes one Task at a time and pauses on failure.
Review update order, delay, monitor window, failure action, and rollback
settings before using a Service in production.

### Secrets And Configs

Select existing Swarm Secrets and Configs and choose their target paths. Secret
values are mounted as files; Citadel does not automatically convert them to
environment variables.

After Apply, Citadel reports two stages:

- **Accepted** means the manager accepted the desired Service definition.
- **Converged** means the required Tasks reached the expected running state.

A successful API response is not proof that the image pulled, a Node matched,
or the application started.

## Manage A Swarm Deployment

A Swarm Deployment has **Config**, **Tasks**, **Bindings**, and **Activities**
tabs. Tasks includes the Service rollout and replica summary, Task history, and
logs. It does not show the Standalone **Container** tab or Container-only
Inspect, Exec, terminal, statistics, adoption, or local-volume backup controls.

The Platform Services page remains the inventory of all observed Services.
Available actions depend on ownership:

- **Citadel Deployment** Services link to their Deployment, where they can be
  edited, applied, scaled, force-updated, rolled back, and deleted.
- **Citadel Stack** Services are configured through Stack Apply or release
  Rollback. They are inspect/log-only from the Services page in v1.
- **External Docker Stack** and unmanaged Services are inspect-only in v1.

### Scale

Scaling changes the desired replica count of a Replicated Service. It is not
available for Global or job Services.

A Service intentionally scaled to zero is shown as Healthy with a **Scaled to
zero** detail. Citadel does not treat the absence of Tasks as a failure in that
case.

### Force Update

Force Update replaces Tasks without otherwise changing the Service definition.
Use it to restart the Service through the scheduler. Individual Tasks cannot be
restarted directly.

### Roll Back

Swarm Deployment rollback asks Docker to return its Service to the previous
Service spec. Stack rollback is different: Citadel reapplies a previous
complete Stack release.

### Concurrent Changes

Docker versions every Service. If another user or tool changes the Service
while your form is open, Citadel reapplies only your changed fields once. If the
Service changes again, Citadel returns a conflict and asks you to refresh rather
than overwriting the newer definition.

## Manage Nodes

The Nodes page shows status, role, availability, manager reachability, labels,
resources, and assigned Tasks.

Citadel v1 supports:

- **Active**: the scheduler may assign new Tasks.
- **Pause**: existing Tasks keep running, but no new Tasks are assigned.
- **Drain**: new Tasks are not assigned and eligible Service Tasks are replaced
  on other Nodes.
- adding, changing, and removing Node labels.

Drain can move production workloads. Review the affected Tasks and available
capacity before confirming it.

Citadel v1 does not promote/demote managers, remove Nodes, change join tokens,
or repair quorum.

## Inspect Tasks And Logs

A Task is one scheduler-owned attempt to run a Service on a Node. When a Task
fails, Docker keeps the historical Task and may create a replacement.

Use Tasks to diagnose:

- image pull or Registry authentication failures;
- impossible placement constraints;
- insufficient CPU or memory;
- port conflicts;
- missing bind paths;
- application exits and health-check failures.

Tasks are read-only. To restart work, force-update the Service. To move work,
drain a Node or change Service placement.

Service and Task logs are available only for compatible Docker logging drivers.
Log history and follow streams are bounded and stop when you close the viewer
or lose the connection.

## Manage Networks

Create an overlay Network when Services on different Nodes must communicate.
The simple form can make it attachable, internal, or encrypted and can configure
supported IPAM options.

Citadel blocks deletion when the Network is ingress, referenced by a Service,
or owned by a Stack. Bridge Networks shown under the connected manager are not
cluster Networks.

## Manage Secrets

Docker does not return a Secret value after creation. Citadel therefore shows
Secret metadata only.

To rotate a Secret:

1. create a new versioned Secret;
2. update the selected Swarm Deployments to use it at the same target;
3. wait for the Services to converge;
4. delete the old Secret after no Service references it.

The create field is cleared after submission. Secret values are not stored in
activity details, logs, error messages, browser drafts, local/session storage,
or cached query/submission data. A failed request does not restore the value;
enter it again after correcting the error.

Citadel variables and secret providers remain the source of application
secrets. A Swarm Secret is one secure file-delivery target. See
`docs/user/variables-and-secrets.md`.

## Manage Configs

Configs deliver non-secret files to Services. Authorized users can inspect
their content. Platform Read shows Config metadata; Platform Read plus Inspect
is required to open the content.

Configs are immutable in Docker. Replace one by creating a new version,
updating Services, waiting for convergence, and deleting the old unreferenced
Config. Stack-owned Configs are changed through Stack Apply.

## Deploy A Stack To Swarm

Citadel uses the same Web Editor and Git Stack sources on Docker Standalone and
Docker Swarm. Create the Stack with a Swarm Platform to use Swarm runtime
behavior. An existing Standalone Stack cannot be switched to Swarm in place;
duplicate or create a new Stack, select the Swarm Platform, and review its
compatibility report before Apply.

The Stack tabs remain **Config**, **Services**, **Releases**, **Bindings**, and
**Activities**. On Swarm, Services shows Swarm Services and Tasks rather than
Standalone Containers.

Before Apply, Citadel checks the Compose files and reports incompatibilities.
For Swarm:

- every Service needs a registry-pullable image;
- `build` requires a Citadel Build binding that has pushed an image;
- Git Stacks preserve their selected Compose-file order and repository env-file
  precedence; Citadel supplies resolved interpolation values to the deploy
  process because Swarm does not load Compose `.env` files automatically;
- v1 supports one selected authenticated private Registry per Stack; public
  images may use other registries, but a second private credential is rejected;
- external Networks, Secrets, Configs, and Volumes must exist;
- bind paths and local volumes produce portability warnings;
- unsupported Compose behavior detected by the compatibility matrix fails
  before Docker is changed;
- pre-deploy/post-deploy commands, Destroy Before Deploy, and Stack
  Start/Stop/Pause/Resume/Restart actions are unavailable.

For interpolation, explicit Citadel variables and bindings override selected
repository env files; later selected env files override earlier ones. Citadel
does not load an unselected `.env` file or inherit variables from the Core/Agent
host. A required unresolved Compose variable stops preflight.

The compatibility report shows the Docker Stack namespace. Citadel refuses to
apply when that namespace belongs to an external Stack, an orphaned Stack, or a
different Citadel Stack; Apply never adopts or takes over it implicitly.
The namespace becomes fixed on first Apply: renaming the Citadel Stack changes
only its display name, and `ProjectName` cannot be changed while the Swarm
runtime exists.

Apply stages the resolved Compose files and a final Citadel ownership override,
then sends them to the validated manager through a fixed `docker stack deploy`
command. Registry credentials are isolated to that run. Citadel then watches
the resulting Services and Tasks through the Docker API and removes transient
Compose, environment, credential, and Secret files.

The command completing successfully means the Stack definition was accepted.
The release becomes Healthy only after the Services converge. A rejected Task,
bad image, impossible constraint, manager disconnect, or timeout is shown as a
degraded, failed, timed-out, or unknown release with the relevant Task error.
An unexpected Docker CLI ignored-option or compatibility error also fails the
release and triggers reconciliation because Docker may already have accepted
part of the definition.
If the CLI fails or the progress connection closes after Docker has changed the
cluster, Citadel still reconciles the namespace in the background. Do not submit
another Apply while the release is **Timed out** or **Unknown**; refresh and wait
for the recovered result.

Stack rollback reapplies a previous immutable Citadel release. It does not roll
back each Service independently. Citadel reuses the exact retained Docker
Secret and Config versions from that release; it does not substitute a secret
provider's current value. If a required retained resource is missing, rollback
stops before changing Docker.

By default, Citadel keeps the required Swarm Secret/Config resources for the 10
newest rollback-eligible successful releases. Older releases remain visible for
audit, but their Rollback action is disabled after those resources leave the
configured retention window.

Deleting a Swarm Stack first removes its owned Services, then removes
unreferenced owned Networks, Secrets, and Configs. Ordinary named or local
Volumes are retained to avoid deleting application data. Citadel deletes the
Stack record only after runtime cleanup is confirmed. If cleanup is interrupted,
the Stack remains visible as Deleting or Unknown and a retry resumes the
remaining work. To reuse a retained Volume in a future Stack, declare its exact
name as an external Volume and pass the compatibility check.

For source-specific setup, see `docs/user/web-editor-stacks.md` and
`docs/user/git-stacks.md`.

## Permissions

Cluster inventory and standalone Docker resources inherit access from their
Platform:

- Platform Read: list and view cluster resources.
- Platform Read plus Inspect: inspect a resource and open Config content.
- Platform Read plus Logs: view unmanaged or Stack Service and Task logs.
- Platform Write: create Networks, Secrets, and Configs; change Node
  availability or labels.
- Platform Execute: delete eligible cluster resources.

Citadel-managed Swarm Services use their existing Deployment permissions plus
visibility of the target Platform. Deployment Read opens the Deployment and
Tasks, Logs opens its logs, Write edits/scales/force-updates it, Apply applies
or rolls it back, and Execute deletes it and its Service.

Stack Apply, release history, and rollback continue to use the existing Stack
permissions. Citadel checks permissions on the server after resolving the
resource to its Platform; a hidden or disabled button is not the security
boundary.

## Import Existing Swarm Workloads

Importing existing Swarm workloads is planned after Swarm v1 and is not
available in the current release. It will use the Swarm resource that owns the
desired state rather than a Task container:

- **Import Service** will be offered for an unmanaged Service that is not part
  of a Docker Stack. It will create a Swarm Deployment associated with that
  Service.
- **Import Stack** will be offered for an unmanaged Docker Stack namespace. It
  will import all Services in the namespace as one Citadel Stack.

A Service carrying a Docker Stack namespace cannot be imported independently;
use **Import Stack** from the namespace or any of its Services. Task containers
cannot be imported because Swarm may replace or reschedule them at any time.

Import will not change Docker immediately. The user will review a sanitized
draft, image and Registry configuration, unsupported settings, and the effects
of the first Apply. Stack import will require a matching Web Editor or Git
Compose source because Docker does not retain the original Compose files or
their interpolation inputs. Existing Citadel ownership labels will route to a
recovery flow instead of normal import.

## Troubleshooting

### The node is not an active Swarm manager

The endpoint is inactive or is a worker. Connect the Agent/Core Docker socket
to an active manager and test again.

### This cluster is already registered

Another Citadel Platform points to a manager in the same Cluster ID. Use that
Platform or edit its manager endpoint instead of creating a duplicate.

### The Service was accepted but is not healthy

Open its Tasks. Check the newest rejected, failed, pending, or restarting Task
for image, credentials, placement, resource, port, mount, or application
errors.

### No suitable node

No active Node satisfies all constraints, reservations, platform requirements,
mount requirements, and port availability. Correct the Service definition or
add capacity; do not repeatedly force-update it.

### Registry authentication failed

Verify the selected Citadel Registry credentials and that every eligible worker
can reach the Registry. An image cached on the manager does not prove workers
can pull it.

### Logs are unavailable

The Service logging driver may not support Docker Service/Task log retrieval.
Inspect the configured driver and use the driver's external log destination
when necessary.

### The operation timed out

Timeout does not prove Docker rejected the mutation. Citadel inspects and
reconciles before offering a retry. Refresh the Service or Stack state and
avoid submitting the same mutation repeatedly while its outcome is unknown.

### The manager is offline

Citadel shows the last observation as stale. Workload Tasks can continue on
workers, but cluster management needs a reachable manager. Restore manager
access or edit the Platform to another active manager in the same Cluster ID.

### Quorum is at risk or lost

Citadel reports the condition but does not change manager roles. Follow Docker's
Swarm administration and disaster-recovery guidance before making topology
changes.

## Swarm V1 Limits

Citadel Swarm v1 does not provide:

- Swarm initialization, join/leave, tokens, CA, or unlock management;
- manager role or quorum changes;
- Task mutation;
- editing, deleting, or running runtime actions against Stack-owned,
  unmanaged, or external Stack Services;
- cluster-wide Image cache management;
- cluster-wide ordinary Volume management or backup;
- in-place conversion of a Deployment or Stack between Docker Standalone and
  Docker Swarm;
- Container Inspect, Exec, terminal, manager-local statistics, adoption, or
  local-volume backup for a Swarm Deployment;
- importing an unmanaged Service or Docker Stack namespace as a Citadel
  Deployment or Stack; these are planned post-v1 workflows;
- Swarm job creation;
- more than one authenticated private Registry in one Stack;
- arbitrary commands around Swarm Stack Apply.
