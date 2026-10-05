---
title: "Build Pools"
description: "Configure dedicated builder connections and monitor their availability."
---

A Build Pool connects Citadel to dedicated builder infrastructure. Select a pool
in a [Build Project](/docs/resources/builds) when builds should run separately
from your managed application Platforms.

Creating, testing, and managing a pool is available in Community. Running builds
through a pool requires **Elastic Build Execution**. Webhook-triggered pool
builds also require **Automated Operations**. Existing in-flight builds may finish
or be cancelled after a license change; queued builds cannot acquire capacity
without the required capability.

## Configure a pool

Use a Build Pool when builds should run on builder infrastructure rather than on one of the Docker platforms managed by Citadel.

For a self-managed VM pool, choose one connection mode.

### Inbound Agent endpoint

Use this when Citadel Core can reach the builder host directly. Run a dedicated Citadel Agent on the builder host:

```bash
docker run -d \
  --name citadel-agent-build \
  --restart=always \
  -p 9001:9000 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e HUB_PUBLIC_KEY="..." \
  ghcr.io/citadel-p/citadel.agent:1.2.3
```

Then create the pool:

```text
Build Pools -> Add Build Pool
Provider: Self-managed VM / Static VM
Connection mode: Inbound Agent endpoint
Endpoint: http://<builder-host>:9001
```

For local Docker testing where Citadel Core also runs in Docker, use:

```text
http://host.docker.internal:9001
```

Click **Test** on the Build Pool before selecting it in a build project. A ready pool confirms that Citadel can reach the Agent and that the Agent can reach Docker.

### Edge Agent

Use this when the builder host should connect outbound to Citadel Core and should not expose a build Agent port.

Create and save the build pool first:

```text
Build Pools -> Add Build Pool
Provider: Self-managed VM / Static VM
Connection mode: Edge Agent
```

After the pool is saved, open the pool's configuration and generate an Edge Agent enrollment token from the **Edge Agent enrollment** section. Citadel shows a Docker command for that build pool. Run it on the builder host.

The build pool Edge Agent is scoped to the build pool itself. You do not need to create a Platform resource, and there is no Edge Agent platform dropdown for build pools.

The generated command uses the Edge Agent gRPC endpoint. Its container port is
`8001`; the supplied Compose host mapping is `127.0.0.1:18001`. A remote builder
needs a reachable, secured gRPC address rather than the browser port or a local-only address.

Click **Test** on the Build Pool before selecting it in a build project. A ready edge pool confirms that the pool-scoped Edge Agent is connected, advertises build capabilities, and can reach Docker.

## Build pool connection and availability

For Edge Agent pools, the pool list shows the agent connection status separately
from the latest Docker build capability check. Connection changes update through
realtime notifications. The pool activity history records one **Connected** or
**Disconnected** event per transition, including disconnects detected by heartbeat
expiry and binding revocation.

The **Build Pool Unavailable** system rule raises a warning after an enabled
self-managed pool fails its capability checks for at least 90 seconds. This covers
both a disconnected agent and a connected agent whose Docker builder is unusable.
Checks run periodically, so an alert can appear after the grace period on the next
check. The outage start is retained across Core restarts.

The alert resolves when a capability check succeeds. Disabled, archived, and
never-connected Edge pools do not raise availability alerts. Disabling or archiving
a pool clears its availability incident. Configure notification channels on the
rule; advanced alerting also allows changing the grace period and limiting the
rule to selected build pools.

## Use a pool in a Build Project

After saving and testing the pool, choose **Build Pool** as the project's runner
and select the pool. The project defines the repository, build context, Registry,
and output image; the pool supplies the execution host.

If a build cannot start, check the pool's connection and Docker capability status,
then see [Build Project troubleshooting](/docs/resources/builds#troubleshooting).
