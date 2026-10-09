---
title: "Containers"
description: "Inspect containers, read logs, open a terminal, and understand runtime actions."
---

Open a **Platform**, then **Containers** to see its Docker workloads, including
containers created outside Citadel. You need access to the Platform or owning
workload; individual tools also require the relevant permissions.

To create an application, use a [Deployment](/docs/resources/deployments) for one
container or a [Stack](/docs/resources/stacks) for Compose. To manage an existing
application's configuration, use [adoption](/docs/guides/adopting-existing-workloads).

## Inspect a container

Open the container's detail page and choose the information you need:

| Tab | Use it to |
| --- | --- |
| **Logs** | Read application output and follow new messages |
| **Inspect** | Examine Docker's container configuration |
| **Terminal** | Run commands inside a running container |
| **Stats** | Check CPU, memory, and network usage |

For memory accounting and host capacity, see
[Platform monitoring](/docs/operations/platform-monitoring).

## Read logs

1. Open the container and select **Logs**.
2. Reproduce the problem and watch for new messages or an error from the log stream.
3. Enable **Timestamps** when comparing output with a failed operation in **Activities**.

The viewer follows output available through Docker's logging API. If it stays
empty, check whether the application writes to standard output or standard error
and whether its Docker logging driver supports reading logs. Files written only
inside the container do not automatically appear here.

For several services together, use the Stack's logs. If multiple live views stop
updating, check the [browser connection](/docs/operations/troubleshooting#live-updates-stop).

## Open a terminal

1. Select **Terminal** on a running container.
2. Choose **bash** or **sh**, then select **Connect**.
3. Run your diagnostic commands. Select **Disconnect** when finished.

The selected shell must exist in the image. If `bash` fails, disconnect and try
`sh`. Minimal images may contain neither shell. A terminal opens inside the
container, so its files and commands are those supplied by that image.

Terminal changes to a container's writable layer can be lost when it is
recreated. Put lasting configuration changes in the Deployment, Compose file,
image, or mounted storage.

## Start, stop, and restart

Use the container's action menu or select rows for a group action. Available
actions depend on current state, ownership, permissions, and connectivity.

| Action | Effect |
| --- | --- |
| **Start** | Start a stopped container |
| **Stop** | Stop its running processes |
| **Restart** | Restart the existing container; does not apply a new image or saved workload configuration |
| **Pause** / **Resume** | Suspend or resume its processes |
| **Delete** | Remove the container, including data stored only in its writable layer |

For configuration or image changes, use **Deploy** or **Redeploy** on the owning
Deployment or Stack. Citadel system containers are protected from ordinary
container lifecycle actions. Swarm Task containers are controlled through their
owning [Service](/docs/resources/swarm-services) or [Stack](/docs/resources/stacks/swarm).

For Swarm logs, terminals, and statistics on other Nodes, check
[Cluster node coverage](/docs/resources/platforms/docker-swarm#understand-cluster-node-coverage).
If an action is disabled, see [troubleshooting](/docs/operations/troubleshooting#an-action-is-unavailable).
