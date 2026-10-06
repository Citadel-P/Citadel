---
title: "Swarm Stacks"
description: "Deploy, import, and operate Compose applications on Docker Swarm."
---

A Swarm Stack deploys a Compose application as native Docker Swarm Services.
Choose a [Swarm Platform](/docs/resources/platforms/docker-swarm), then author
its definition in the [Web Editor](/docs/resources/stacks/web-editor) or select
files from [Git](/docs/resources/stacks/git).

## Create and deploy a Swarm Stack

Open **Stacks**, select **Add Stack**, and choose a Docker Swarm Platform. The Stack
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

Select **Deploy** after reviewing a valid draft. The progress sheet reports
preflight, Docker CLI output, and rollout observation. Citadel supports this
flow through Local, regular Agent, and Edge Agent connections.

Swarm owns the lifecycle of a Stack's Tasks, so Swarm Stacks do not show the
container-oriented **Start**, **Stop**, **Pause**, **Resume**, or **Reconcile
drift** actions used by Docker Standalone Stacks. Manually changing an
individual Task container is temporary because Swarm recreates it to restore
the Service's desired state. Change the Compose configuration, including
`deploy.replicas`, then use **Deploy** or **Redeploy** instead.

Docker accepting the Stack definition does not by itself mark the release
healthy. Citadel observes the Stack Services and their Tasks and reports a
failed Task error when the rollout cannot converge. A deployment process is
limited to five minutes and the initial rollout observation is limited to two
minutes. If the connection or progress stream is interrupted after dispatch,
the release remains **Unknown** or **Timed out** and background Swarm
reconciliation can recover it after Docker's state becomes observable.

Stack-owned Services remain visible from the Swarm Platform's **Services** and
**Tasks** pages. After the first deployment, the Stack's **Services** tab also shows
only the Services owned by that Stack, with their current Tasks as expandable
rows. The table follows the live Swarm inventory stream. Select a Service to
use the bounded log viewer, inspect its Docker definition, or open a terminal
for a running Task on a covered Node. Citadel does not show aggregate Stack
statistics.

Rollback reapplies a selected healthy release's saved configuration through
the same deployment checks. It does not restore application data. An unpinned
Git source can resolve to a newer branch commit; see the
[Git rollback limitations](/docs/resources/stacks/git#rollback) before using it.
Citadel records owned Secret and Config IDs after a healthy deployment, but
the current Rust rollback path does not restore those recorded IDs or manage
a configurable retention window for them. Keep required Docker Secrets and
Configs available and review their references before rollback.

Deleting a Swarm Stack first verifies namespace ownership, removes its owned
Services, waits for their absence, and then removes only unreferenced owned
Networks, Secrets, and Configs. Named and local Volumes are retained. The Stack
record is removed only after runtime cleanup is confirmed, so a retry resumes
from Docker's observed state.

Native Docker Secrets and Configs can be referenced in the Compose definition.
Citadel's **Mounted file** binding option is not connected to the current Rust
Stack deployment path. Use supported Compose secret references or
environment-variable bindings; see
[secret delivery modes](/docs/guides/variables-and-secrets#delivery-mode).

For Agent and Edge Agent connections, a Git source snapshot is rejected before
dispatch when it exceeds 512 files or 12 MiB. Reduce the Stack deployment
source or move large application data out of the repository before retrying.

## Import an existing Stack or Compose project

On a Swarm manager,
Citadel can import an existing Docker Stack namespace or a regular Docker
Compose project. A Docker Stack is imported as one complete namespace from
**Services** or its parent row
in **Containers**. A regular Compose project is imported from its parent row in
**Containers** and converted on its first explicit deployment. Swarm Task containers
never expose individual adoption actions.

Importing a regular Compose project does not mutate Docker. Citadel validates
the authoritative Web Editor or Git source against both the running project and
Swarm compatibility rules. On the first deployment, Citadel stops the Compose
project without deleting named volumes, then deploys the reviewed source as a
native Docker Stack under the same project name. This transition has downtime.
If Compose shutdown is incomplete or Docker rejects the `docker stack deploy`
command, Citadel restores the Compose project from the reviewed source and
reports both outcomes in the progress sheet. If Docker accepts the command but
the later rollout fails, the workload remains a Swarm Stack so its failure can
be inspected and corrected.

## Image checks and automatic updates

For Web Editor stacks, **Check Updates** compares each supported image tag with
the image digest recorded in its Swarm Service on the manager. Tasks do not need
to run on the manager, and node-agent coverage is not required for this check.
Select a Registry with access to the images. If a deployed Service has no resolved
digest, deploy the stack with registry access before checking again. Images pinned
to a digest and services linked to Citadel Builds are excluded from tag checks.

| Auto Update option | Behavior |
| --- | --- |
| **Disabled** | No scheduled checks; manual checks remain available. |
| **Notify Only** | Detect updates and report them without deploying. Configure an Alert Rule to receive notifications. |
| **Auto Deploy Stack** | Deploy the complete stack when an update is found, using Swarm's rollout configuration. Requires the automation and operational guardrails entitlements. |
| **Auto Deploy Services** | Unavailable for Swarm stacks; service-scoped Stack deployment is not supported. |

Git stacks check changes to their tracked source rather than registry image tags.
Use **Notify Only** or **Auto Deploy Stack** for that source. A stack pinned to a
Git commit cannot follow new commits automatically. Build-linked services use
their build completion settings separately from registry image checks.

## Permissions

Importing an external Docker Stack requires Stack Write plus Platform Read and
Inspect. Deploy, rollback, and delete use the Stack Apply, Releases, and Execute
permissions respectively, together with Platform visibility.

## Related resources

- [Swarm Services](/docs/resources/swarm-services) explains Task logs and terminals.
- [Cluster node coverage](/docs/resources/platforms/docker-swarm#understand-cluster-node-coverage) enables runtime operations beyond the connected manager.
- [Backups](/docs/resources/backups#docker-swarm) describes Swarm Volume backup and restore limits.
