# Agent protocol

This is the developer reference for Core/Agent transport, authentication and
Swarm-node restrictions. For image builds and local setup, see
[Agent development](DEVELOPMENT.md#agent-development).

## Transports

| Transport | Connection | Authentication |
| --- | --- | --- |
| Direct | Core calls the Agent's gRPC listener | Signed requests verified with `HUB_PUBLIC_KEY` |
| Edge | Agent connects to Core's Edge gRPC listener | Enrollment and persisted session identity |

Both transports expose the same 68 operations. Edge carries commands through
`EdgeAgentService/Connect`, a bidirectional stream from
`citadel.edge.v1.AgentEnvelope` to `citadel.edge.v1.CoreEnvelope`. Its protocol
version is **2**. Command number 0 and unknown command numbers are rejected.

The Agent runs without PostgreSQL. It maps requests onto shared Docker and
external-tool implementations; Core owns persisted resource and user state.

## Image metadata comparisons

`DistributionInspect` accepts an optional target OS/architecture/variant. With that
field, current Agents read only the matching OCI manifest/config metadata and
return its configuration and manifest digests. Requests without it retain the
existing top-level distribution descriptor behavior. Image inspection identifies
the digest type so Core compares images correctly across Docker image stores.
Checks read metadata only. Missing comparison fields from older Agents yield an
unavailable comparison in Core. No operation numbers or Swarm-node allowlists
change; node-local registry checks remain unavailable.

## Network inventory

Network list responses include attached endpoints in the additive shared
`Network.containers` field (17). Local and Agent runtimes derive these from the
container inventory already fetched for usage counts; configured networks without
an endpoint are not reported as attachments. Core still filters relationships and
endpoint details by actor permissions. Direct and Edge share this mapping and the
existing operation numbers, cancellation bounds, and Swarm-node allowlists.
Older Agents omit these endpoints, so workload maps can show incomplete evidence
until the Agent is updated; network inspection remains available.

## Deployment profiles

All profiles use the same Agent image. Build Pools do not have a separate transport.

| Profile | Mode / profile setting | Core resource | Listener | Template |
| --- | --- | --- | --- | --- |
| Direct | Default mode | Platform | Direct RPCs and health on `0.0.0.0` | [Direct](../deploy/.env.agent.example) |
| Direct Build Pool | Default mode | BuildAgentPool | Direct RPCs and health on `0.0.0.0` | [Build Pool](../deploy/.env.build-agent.example) |
| Edge | `edge` / `edge-agent` (default) | Platform | Health on loopback only | [Edge](../deploy/.env.edge.example) |
| Edge Build Pool | `edge` / `edge-build-agent` | BuildAgentPool | Health on loopback only | [Edge Build Pool](../deploy/.env.edge-build-agent.example) |
| Swarm-node | `edge` / `swarm-node` | Platform and node identity | Health on loopback only | Injected by Core's node-agent installer |

`CITADEL_AGENT_MODE` selects the transport; `CITADEL_EDGE_AGENT_PROFILE` selects
an Edge profile. Swarm-node credentials cannot enroll as an ordinary Edge Agent.
The default listener port is 9000. `/health` is unsigned process liveness, not
proof of Docker reachability or an established Core session.

## Configuration

Read configuration once at startup. Docker can load the templates with
`--env-file`; the Agent does not load dotenv files implicitly.

| Variable | Default / requirement | Applies to |
| --- | --- | --- |
| CITADEL_AGENT_MODE | case-insensitive `edge` selects Edge; otherwise Direct; whitespace is not trimmed | all |
| CITADEL_AGENT_PORT | 9000 when absent/blank; otherwise integer 1–65535 | all; health listener in Edge |
| HUB_PUBLIC_KEY | required base64 raw Ed25519 public key | Direct |
| CITADEL_AGENT_TLS_MODE | Disabled; accepts Disabled or Direct after trimming, case-insensitive; reject numeric/unknown values | Direct |
| CITADEL_AGENT_TLS_CERTIFICATE_PATH | required for Direct TLS; reject when TLS disabled | Direct |
| CITADEL_AGENT_TLS_PRIVATE_KEY_PATH | required for Direct TLS; reject when TLS disabled | Direct |
| CITADEL_CORE_URL | required absolute HTTP(S) origin; no credentials, nonroot path, query or fragment | Edge |
| CITADEL_EDGE_ENROLLMENT_TOKEN | required to enroll; persisted identity permits reconnection | ordinary Edge / Edge Build Pool |
| CITADEL_EDGE_AGENT_PROFILE | edge-agent; accepts edge-agent, edge-build-agent, swarm-node after trimming, case-insensitive | Edge |
| CITADEL_EDGE_CORE_CA_CERTIFICATE_PATH | optional PEM CA; HTTPS only; additive trust with hostname verification | Edge |
| CITADEL_EDGE_AGENT_KEY_PATH | /app/data/edge-agent.key when unset | Edge |
| CITADEL_EDGE_IDENTITY_PATH | /app/data/edge-agent.identity.json when unset | Edge |
| CITADEL_EDGE_BOOTSTRAP_FILE | required bootstrap file | Swarm-node |
| CITADEL_PLATFORM_ID | required platform identity | Swarm-node |
| CITADEL_SWARM_SERVICE_ID | required service identity | Swarm-node |
| CITADEL_SWARM_TASK_ID | required task identity | Swarm-node |
| CITADEL_SWARM_NODE_ID | required node identity | Swarm-node |
| CITADEL_SWARM_NODE_HOSTNAME | required node hostname | Swarm-node |
| CITADEL_SWARM_CLUSTER_ID | required cluster identity | Swarm-node |
| DOCKER_HOST | unix:///var/run/docker.sock; Unix sockets and TCP/HTTP endpoints are supported by the Linux Agent | all |
| CITADEL_HOST_ROOT | /host; optional host filesystem mount for disk usage | all |
| RUST_LOG | citadel_agent=info; optional tracing filter | all; Rust host diagnostics |

Edge ignores Direct TLS settings and does not require `HUB_PUBLIC_KEY`. Profile,
bootstrap metadata and optional CA inputs are trimmed. Key and identity paths use
defaults only when unset; an explicit empty value is not a default.

The selected Docker endpoint applies to requests, streams, exec and Docker CLI
work. Unsupported transports fail configuration; there is no fallback to another
daemon. The Linux Agent does not support Docker HTTPS endpoints or Windows named
pipes. This is separate from HTTPS support for Core/Agent connections.

## Direct authentication

Requests are signed with Ed25519. The signature covers the timestamp, nonce,
method path and SHA-256 hash of the serialized request. Binary metadata has fixed
lengths: timestamp 8 bytes, nonce 16, body hash 32 and signature 64.

The Agent accepts a timestamp skew of ±60 seconds. Replay protection is atomic,
retains nonces through the last accepted timestamp second, and is capped at
65,536 entries. A full cache rejects new requests rather than evicting a nonce
that could still be replayed.

For interactive Exec, the first signed message must be `Open` and arrive within
10 seconds. Authentication completes before Docker execution. Preserve the
verified message when handing the stream to the handler; resize-first input
must not bypass authentication.

Implementation: [direct/auth.rs](../src/agent/src/direct/auth.rs).

Interactive terminal execution shares Docker process ownership with Local Core:
the Docker adapter records the terminal's container-namespace PID and start time,
and cleans up that process session on disconnect, cancellation or idle expiry.
An idle terminal has no input or output for 15 minutes; resize does not count as
activity. Cleanup remains tracked during shutdown and never stops the application
container. This does not change the signed Exec messages, operation numbers, or
Swarm-node allowlists. It requires `/bin/sh`, `sleep`, and Linux procfs in the target image.
Existing Agent versions must be upgraded to gain this cleanup and idle timeout.

Non-interactive `ExecBinary` uses the same identity-checked process-session cleanup
on cancellation, stream drop and shutdown. It preserves binary stdout/stderr,
exit status, environment, working directory and the execution user. A private
startup record is removed before forwarding output. It also requires `/bin/sh`, `sleep`,
and readable Linux procfs in the target image; no extra privileges are granted.
Binary commands retain their caller's operation deadline rather than the terminal
inactivity timeout. Helper-container removal remains a separate cleanup layer.
A launcher waits for identity-checked approval before running the requested
command. If startup times out or its response is lost, an unapproved launcher
terminates itself after 10 seconds. Approval failures after identity is known
use the normal cleanup path. Descendant identities are collected before signals
are sent, so a parent exiting during cleanup does not abandon the remaining
captured children. Docker outages can still prevent cleanup of an approved
command; upgraded Agents are required for this behavior.

## Edge lifecycle and limits

Ordinary Edge and Edge Build Pool enroll once and persist a private key and
identity file. Keep both files across restarts. Corrupt state fails startup and
is left untouched. A changed enrollment-token fingerprint resets ordinary
identity and key state; Swarm-node identity comes from its bootstrap credential.

HTTPS validates trusted roots and Core's hostname. An optional CA file adds
trust without disabling verification. Core's URL must target its Edge gRPC
listener and be reachable from the Agent host.

| Limit | Value |
| --- | --- |
| Envelope payload | 16 MiB |
| Active commands per session | 16 |
| Input queue per command | 256 messages, with a separate byte bound |
| Outgoing queue | 512 messages |
| Heartbeat interval | 30 seconds |
| HTTP/2 keepalive | 30 seconds, with a 10-second timeout |
| Reconnect backoff | Starts at 1 second; base delay caps at 60 seconds, with jitter |

Commands have deadlines and cancellation. Disconnect or shutdown cancels work
owned by the session. SIGTERM and Ctrl+C stop listeners and outbound work with
a ten-second shutdown limit. These limits are implementation constants, not
additional deployment settings.

Implementation: [edge/](../src/agent/src/edge/).

## Swarm-node restrictions and capabilities

Ordinary Edge and Edge Build Pool use the full command dispatcher. Swarm-node
uses an explicit allowlist and validates the configured node identity before
Docker calls. The operation table below distinguishes:

- **allowed**: available to the restricted profile, subject to request validation;
- **denied**: unavailable to Swarm-node Agents;
- **helper-only**: only validated Citadel helper containers and commands;
- **restore-only**: only owned backup-restore volumes, with safe removal checks.

Helper requests validate ownership, image, mounts, privileges and exact commands.
Binary exec inspects the helper and executes by immutable container ID. Restore
volume deletion checks for container references, including stopped containers.
Inspection checks normalize Docker's default bridge network and omitted false
mount flags; they still require read-only backup sources and isolated host-path
probes. Named-volume restores mount the destination at `/target/source` or
`/target/data`, matching the validated snapshot root, and restore the complete
snapshot into `/target`. This preserves the volume root's numeric ownership and
permissions. These child mount paths are not allowed for bind-mount helpers.

Bind-mount backups use separately validated helper mounts and exact shared shell
templates for path probes, backup and restore. Host-root probes have no repository
credentials, a read-only root filesystem and no network; they can only validate
existing paths without symlink components. They cannot execute Restic. Data
helpers mount only the selected validated host path with nested mounts disabled.
Arbitrary shell commands, script changes, unsafe paths, credential-environment
injection and mismatched backup/restore mount directions are rejected.

Current-data workload copying uses a separate exact helper shape: two distinct
plain local named volumes, read-only `/source` and an owned writable `/target/source`,
network mode `none`, bounded lifetime and resource limits. Binary exec accepts
only the shared temporary-snapshot/restore script and its two fixed-purpose
Restic environment entries. The repository stays inside the helper. The guard
rechecks source content access and matches the destination restore-owner receipt
to the helper's copy-job ID before creation and execution. No general shell or
volume-management permissions are added to the restricted profile.

Capability advertisements are discovery hints, not authorization. Ordinary Edge
and Edge Build Pool advertise the same capabilities. Swarm-node advertises a
smaller set; guarded restore-volume operations are intentionally available without
advertising general volume creation/deletion.

The current strings live in [edge/observation.rs](../src/agent/src/edge/observation.rs).
Policy and helper guards live in [edge/swarm_policy.rs](../src/agent/src/edge/swarm_policy.rs)
and [edge/swarm_guard.rs](../src/agent/src/edge/swarm_guard.rs).

## Verification

Run the contract and configuration checks from the repository root:

```bash
cargo test --locked -p citadel-contracts
cargo test --locked -p citadel-agent --test configuration
cargo test --locked -p citadel-agent --lib
```

Contract tests compare the operation table with compiled protobuf descriptors:
RPC names, message types, streaming shape, Edge command numbers and completeness.
They also check environment-template coverage. Agent unit tests compare the
Swarm-node policy with every operation row.

Registration and unit tests do not establish live interoperability. Use the
[Agent image, compatibility and acceptance runners](DEVELOPMENT.md#agent-development)
for packaged runtime behavior. Mixed-version verification additionally requires
a released Agent image.

When changing the protocol, update protobufs, mappings, this inventory and affected
tests together. Preserve existing field numbers and command numbers unless making
an explicitly coordinated protocol change.

## Operation reference

The protobuf sources are authoritative:
[src/infrastructure/contracts/proto/](../src/infrastructure/contracts/proto/).
Direct handlers live in [src/agent/src/direct/](../src/agent/src/direct/).
Runtime reference paths below are relative to `src/infrastructure/adapters/src/`.

The final column identifies the behavior checks that apply. Every operation also
requires G0 and G1; Swarm-node operations additionally require G8.

- **G0 — Docker endpoint:** use the configured endpoint for HTTP, streams, exec and CLI work; reject unsupported transports and never fall back to another daemon.
- **G1 — Agent boundaries:** preserve protobuf presence, defaults and complete responses; enforce authentication, deadlines, cancellation and error translation.
- **G2 — Lists and filters:** preserve request filters, limits, Swarm scope, empty lists and volume warnings/usage.
- **G3 — Exec:** preserve binary framing, TTY, environment, user, resize, half-close and exit status; bound input and output.
- **G4 — Streams and sampling:** honor sampling intervals, bounded queues, cancellation and stream completion; propagate failure without reporting success.
- **G5 — Images/builds:** validate context paths and build options; isolate credentials, redact output and support independent build/push operations.
- **G6 — Apply:** execute deployment and stack operations without a database; preserve resource settings, registry auth, secret files, progress and cleanup.
- **G7 — Swarm mutations:** enforce version concurrency, labels, restart updates and managed/system-service constraints.
- **G8 — Swarm-node guards:** enforce the allowlist before Docker calls; validate helper ownership, images, privileges, mounts and commands; remove only unused owned restore volumes.

<!-- operations -->
| RPC | Request | Response | Stream | Edge command | Number | Swarm | Runtime reference | Checks |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| citadel.containers.v1.ContainerService/List | citadel.containers.v1.ListContainersRequest | citadel.containers.v1.ListContainersResponse | unary | CONTAINER_LIST | 10 | allowed | connectors/docker/finite.rs:list_container_models | G2 |
| citadel.containers.v1.ContainerService/Start | citadel.containers.v1.ContainerIds | google.protobuf.Empty | unary | CONTAINER_START | 14 | allowed | connectors/docker/finite.rs:start_container | G1 |
| citadel.containers.v1.ContainerService/Stop | citadel.containers.v1.ContainerIds | google.protobuf.Empty | unary | CONTAINER_STOP | 15 | allowed | connectors/docker/finite.rs:change_container_state | G1 |
| citadel.containers.v1.ContainerService/Pause | citadel.containers.v1.ContainerIds | google.protobuf.Empty | unary | CONTAINER_PAUSE | 16 | allowed | connectors/docker/finite.rs:change_container_state | G1 |
| citadel.containers.v1.ContainerService/Unpause | citadel.containers.v1.ContainerIds | google.protobuf.Empty | unary | CONTAINER_UNPAUSE | 17 | allowed | connectors/docker/finite.rs:change_container_state | G1 |
| citadel.containers.v1.ContainerService/Restart | citadel.containers.v1.ContainerIds | google.protobuf.Empty | unary | CONTAINER_RESTART | 18 | allowed | connectors/docker/finite.rs:change_container_state | G1 |
| citadel.containers.v1.ContainerService/Delete | citadel.containers.v1.DeleteContainerRequest | google.protobuf.Empty | unary | CONTAINER_DELETE | 19 | allowed | connectors/docker/finite.rs:delete_container_with_options | G1 |
| citadel.containers.v1.ContainerService/Inspect | citadel.containers.v1.InspectContainerRequest | citadel.shared_models.v1.InspectContainerResponse | unary | CONTAINER_INSPECT | 12 | allowed | connectors/docker/transport/mod.rs:inspect_container_document | G1 |
| citadel.containers.v1.ContainerService/Create | citadel.containers.v1.CreateContainerRequest | citadel.containers.v1.CreateContainerResponse | unary | CONTAINER_CREATE | 13 | helper-only | connectors/docker/finite.rs:create_container | G1 |
| citadel.containers.v1.ContainerService/Exec | citadel.containers.v1.ExecClientMessage | citadel.containers.v1.ExecServerMessage | bidi | CONTAINER_EXEC | 22 | allowed | connectors/docker/transport/terminal.rs:execute_terminal | G3 |
| citadel.containers.v1.ContainerService/ExecBinary | citadel.containers.v1.ExecBinaryRequest | citadel.containers.v1.ExecServerMessage | server | CONTAINER_EXEC_BINARY | 23 | helper-only | connectors/docker/transport/binary_exec.rs:execute_binary | G3 |
| citadel.containers.v1.ContainerService/StreamContainerLogs | citadel.containers.v1.ContainerLogRequest | citadel.containers.v1.ContainerLogResponse | server | CONTAINER_LOGS_STREAM | 11 | allowed | connectors/docker/transport/mod.rs:open_logs | G4 |
| citadel.containers.v1.ContainerService/StreamContainersStats | citadel.containers.v1.StreamContainersStatsRequest | citadel.containers.v1.ContainersStatsResponse | server | CONTAINERS_STATS_STREAM | 21 | allowed | connectors/docker/local_sampler.rs:sample | G4 |
| citadel.containers.v1.ContainerService/StreamContainerStats | citadel.containers.v1.StreamContainerStatsRequest | citadel.shared_models.v1.ContainerMessage | server | CONTAINER_STATS_STREAM | 20 | allowed | connectors/docker/transport/mod.rs:container_stats | G4 |
| citadel.deployments.v1.DeploymentService/Apply | citadel.deployments.v1.ApplyDeploymentRequest | citadel.deployments.v1.ApplyDeploymentResponse | unary | DEPLOYMENT_APPLY | 80 | denied | connectors/docker/deployments.rs:apply_container_config | G6 |
| citadel.images.v1.ImageService/Get | citadel.images.v1.GetImageRequest | citadel.shared_models.v1.ImageReply | unary | IMAGE_GET | 30 | denied | connectors/docker/finite.rs:list_images | G1 |
| citadel.images.v1.ImageService/List | citadel.images.v1.ListImagesRequest | citadel.shared_models.v1.ListImageResponse | unary | IMAGE_LIST | 31 | allowed | connectors/docker/finite.rs:list_images | G1 |
| citadel.images.v1.ImageService/Delete | citadel.images.v1.DeleteImageRequest | citadel.images.v1.DeleteImageResponse | unary | IMAGE_DELETE | 33 | denied | connectors/docker/finite.rs:delete_image | G1 |
| citadel.images.v1.ImageService/Inspect | citadel.images.v1.InspectImageRequest | citadel.images.v1.InspectImageResponse | unary | IMAGE_INSPECT | 32 | allowed | connectors/docker/finite.rs:inspect_image | G1 |
| citadel.images.v1.ImageService/History | citadel.images.v1.HistoryImageRequest | citadel.images.v1.HistoryImageResponse | unary | IMAGE_HISTORY | 34 | denied | connectors/docker/finite.rs:image_history | G1 |
| citadel.images.v1.ImageService/GetExposedPorts | citadel.images.v1.GetExposedPortsRequest | citadel.images.v1.GetExposedPortsResponse | unary | IMAGE_EXPOSED_PORTS | 35 | denied | connectors/docker/finite.rs:inspect_image | G1 |
| citadel.images.v1.ImageService/DistributionInspect | citadel.images.v1.DistributionInspectRequest | citadel.images.v1.DistributionInspectResponse | unary | IMAGE_DISTRIBUTION_INSPECT | 36 | denied | connectors/docker/finite.rs:distribution_inspect_authenticated | G1 |
| citadel.images.v1.ImageService/CheckBuildHost | google.protobuf.Empty | citadel.images.v1.CheckBuildHostResponse | unary | IMAGE_CHECK_BUILD_HOST | 40 | denied | connectors/docker/transport/mod.rs:version | G5 |
| citadel.images.v1.ImageService/Pull | citadel.images.v1.PullImageRequest | citadel.images.v1.PullImageResponse | server | IMAGE_PULL_STREAM | 37 | denied | connectors/docker/transport/mod.rs:pull_image_with_options | G5 |
| citadel.images.v1.ImageService/Build | citadel.images.v1.BuildImageRequest | citadel.images.v1.ImageBuildResponse | server | IMAGE_BUILD_STREAM | 38 | denied | external/builds/runtime.rs:build | G5 |
| citadel.images.v1.ImageService/Push | citadel.images.v1.PushImageRequest | citadel.images.v1.ImageBuildResponse | server | IMAGE_PUSH_STREAM | 39 | denied | external/builds/runtime.rs:push | G5 |
| citadel.networks.v1.NetworkService/List | citadel.networks.v1.ListNetworksRequest | citadel.networks.v1.ListNetworksResponse | unary | NETWORK_LIST | 60 | allowed | connectors/docker/finite.rs:list_networks_filtered | G2 |
| citadel.networks.v1.NetworkService/Create | citadel.networks.v1.CreateNetworkRequest | citadel.networks.v1.CreateNetworkResponse | unary | NETWORK_CREATE | 62 | denied | connectors/docker/finite.rs:create_network | G1 |
| citadel.networks.v1.NetworkService/Delete | citadel.networks.v1.DeleteNetworkRequest | citadel.networks.v1.DeleteNetworkResponse | unary | NETWORK_DELETE | 63 | denied | connectors/docker/finite.rs:delete_network | G1 |
| citadel.networks.v1.NetworkService/Inspect | citadel.networks.v1.InspectNetworkRequest | citadel.networks.v1.InspectNetworkResponse | unary | NETWORK_INSPECT | 61 | allowed | connectors/docker/finite.rs:inspect_network | G1 |
| citadel.platforms.v1.PlatformService/GetPlatformInfo | google.protobuf.Empty | citadel.shared_models.v1.PlatformInfoResponse | unary | PLATFORM_GET_INFO | 2 | allowed | connectors/docker/finite.rs:info | G1 |
| citadel.platforms.v1.PlatformService/CheckHealth | google.protobuf.Empty | citadel.platforms.v1.CheckHealthResponse | unary | PLATFORM_CHECK_HEALTH | 1 | allowed | connectors/docker/transport/mod.rs:ping | G1 |
| citadel.platforms.v1.PlatformService/Prune | citadel.platforms.v1.PruneRequest | citadel.platforms.v1.PruneResponse | unary | PLATFORM_PRUNE | 5 | denied | connectors/docker/finite.rs:prune_resources | G1 |
| citadel.platforms.v1.PlatformService/StreamPlatformStats | citadel.platforms.v1.PlatformStatsRequest | citadel.platforms.v1.PlatformStatsResponse | server | PLATFORM_STATS_STREAM | 3 | allowed | connectors/docker/local_sampler.rs:sample | G4 |
| citadel.platforms.v1.PlatformService/StreamDaemonEvent | google.protobuf.Empty | citadel.platforms.v1.DaemonEventResponse | server | PLATFORM_DAEMON_EVENTS_STREAM | 4 | allowed | connectors/docker/transport/mod.rs:events | G4 |
| citadel.stacks.v1.StackService/Apply | citadel.stacks.v1.StackApplyRequest | citadel.stacks.v1.StackApplyResponse | server | STACK_APPLY_STREAM | 70 | denied | external/stacks.rs:apply | G6 |
| citadel.swarm.v1.SwarmService/ListNodes | citadel.swarm.v1.ListSwarmNodesRequest | citadel.swarm.v1.ListSwarmNodesResponse | unary | SWARM_NODE_LIST | 90 | denied | connectors/docker/finite.rs:list_swarm_nodes_filtered | G2 |
| citadel.swarm.v1.SwarmService/InspectNode | citadel.swarm.v1.InspectSwarmNodeRequest | citadel.swarm.v1.SwarmNodeMessage | unary | SWARM_NODE_INSPECT | 91 | denied | connectors/docker/finite.rs:inspect_swarm_node | G1 |
| citadel.swarm.v1.SwarmService/UpdateNode | citadel.swarm.v1.UpdateSwarmNodeRequest | google.protobuf.Empty | unary | SWARM_NODE_UPDATE | 115 | denied | connectors/docker/finite.rs:update_swarm_node | G7 |
| citadel.swarm.v1.SwarmService/ListServices | citadel.swarm.v1.ListSwarmServicesRequest | citadel.swarm.v1.ListSwarmServicesResponse | unary | SWARM_SERVICE_LIST | 92 | denied | connectors/docker/finite.rs:list_swarm_services_filtered | G2 |
| citadel.swarm.v1.SwarmService/InspectService | citadel.swarm.v1.InspectSwarmServiceRequest | citadel.swarm.v1.SwarmServiceMessage | unary | SWARM_SERVICE_INSPECT | 93 | denied | connectors/docker/finite.rs:inspect_swarm_service | G1 |
| citadel.swarm.v1.SwarmService/CreateService | citadel.swarm.v1.CreateManagedSwarmServiceRequest | citadel.swarm.v1.SwarmServiceMutationResponse | unary | SWARM_SERVICE_CREATE | 111 | denied | connectors/docker/finite.rs:create_swarm_service | G7 |
| citadel.swarm.v1.SwarmService/UpdateService | citadel.swarm.v1.UpdateManagedSwarmServiceRequest | citadel.swarm.v1.SwarmServiceMutationResponse | unary | SWARM_SERVICE_UPDATE | 112 | denied | connectors/docker/finite.rs:update_swarm_service | G7 |
| citadel.swarm.v1.SwarmService/RestartService | citadel.swarm.v1.RestartSwarmServiceRequest | google.protobuf.Empty | unary | SWARM_SERVICE_RESTART | 114 | denied | connectors/docker/finite.rs:update_swarm_service | G7 |
| citadel.swarm.v1.SwarmService/DeleteService | citadel.swarm.v1.DeleteManagedSwarmServiceRequest | google.protobuf.Empty | unary | SWARM_SERVICE_DELETE | 113 | denied | connectors/docker/finite.rs:delete_swarm_service | G7 |
| citadel.swarm.v1.SwarmService/GetServiceLogs | citadel.swarm.v1.SwarmLogsRequest | citadel.swarm.v1.SwarmLogsResponse | unary | SWARM_SERVICE_LOGS | 102 | denied | connectors/docker/transport/mod.rs:open_resource_logs | G4 |
| citadel.swarm.v1.SwarmService/ListTasks | citadel.swarm.v1.ListSwarmTasksRequest | citadel.swarm.v1.ListSwarmTasksResponse | unary | SWARM_TASK_LIST | 94 | denied | connectors/docker/finite.rs:list_swarm_tasks_filtered | G2 |
| citadel.swarm.v1.SwarmService/InspectTask | citadel.swarm.v1.InspectSwarmTaskRequest | citadel.swarm.v1.SwarmTaskMessage | unary | SWARM_TASK_INSPECT | 95 | denied | connectors/docker/finite.rs:inspect_swarm_task | G1 |
| citadel.swarm.v1.SwarmService/GetTaskLogs | citadel.swarm.v1.SwarmLogsRequest | citadel.swarm.v1.SwarmLogsResponse | unary | SWARM_TASK_LOGS | 103 | denied | connectors/docker/transport/mod.rs:task_logs | G4 |
| citadel.swarm.v1.SwarmService/ListNetworks | citadel.swarm.v1.ListSwarmNetworksRequest | citadel.swarm.v1.ListSwarmNetworksResponse | unary | SWARM_NETWORK_LIST | 96 | denied | connectors/docker/finite.rs:list_networks_filtered | G2 |
| citadel.swarm.v1.SwarmService/InspectNetwork | citadel.swarm.v1.InspectSwarmNetworkRequest | citadel.swarm.v1.SwarmNetworkMessage | unary | SWARM_NETWORK_INSPECT | 97 | denied | connectors/docker/finite.rs:inspect_network | G1 |
| citadel.swarm.v1.SwarmService/ListSecrets | citadel.swarm.v1.ListSwarmSecretsRequest | citadel.swarm.v1.ListSwarmSecretsResponse | unary | SWARM_SECRET_LIST | 98 | denied | connectors/docker/finite.rs:list_swarm_secrets_filtered | G2 |
| citadel.swarm.v1.SwarmService/InspectSecret | citadel.swarm.v1.InspectSwarmSecretRequest | citadel.swarm.v1.SwarmSecretMessage | unary | SWARM_SECRET_INSPECT | 99 | denied | connectors/docker/finite.rs:inspect_swarm_secret | G1 |
| citadel.swarm.v1.SwarmService/CreateSecret | citadel.swarm.v1.CreateSwarmSecretRequest | citadel.swarm.v1.SwarmResourceCreateResponse | unary | SWARM_SECRET_CREATE | 104 | denied | connectors/docker/finite.rs:create_swarm_material | G7 |
| citadel.swarm.v1.SwarmService/UpdateSecretLabels | citadel.swarm.v1.UpdateSwarmResourceLabelsRequest | google.protobuf.Empty | unary | SWARM_SECRET_UPDATE | 105 | denied | connectors/docker/finite.rs:update_swarm_material | G7 |
| citadel.swarm.v1.SwarmService/DeleteSecret | citadel.swarm.v1.DeleteSwarmSecretRequest | google.protobuf.Empty | unary | SWARM_SECRET_DELETE | 106 | denied | connectors/docker/finite.rs:delete_swarm_secret | G7 |
| citadel.swarm.v1.SwarmService/ListConfigs | citadel.swarm.v1.ListSwarmConfigsRequest | citadel.swarm.v1.ListSwarmConfigsResponse | unary | SWARM_CONFIG_LIST | 100 | denied | connectors/docker/finite.rs:list_swarm_configs_filtered | G2 |
| citadel.swarm.v1.SwarmService/InspectConfig | citadel.swarm.v1.InspectSwarmConfigRequest | citadel.swarm.v1.SwarmConfigMessage | unary | SWARM_CONFIG_INSPECT | 101 | denied | connectors/docker/finite.rs:inspect_swarm_config | G1 |
| citadel.swarm.v1.SwarmService/GetConfigData | citadel.swarm.v1.InspectSwarmConfigRequest | citadel.swarm.v1.SwarmConfigDataResponse | unary | SWARM_CONFIG_DATA | 110 | denied | connectors/docker/finite.rs:inspect_swarm_config | G7 |
| citadel.swarm.v1.SwarmService/CreateConfig | citadel.swarm.v1.CreateSwarmConfigRequest | citadel.swarm.v1.SwarmResourceCreateResponse | unary | SWARM_CONFIG_CREATE | 107 | denied | connectors/docker/finite.rs:create_swarm_material | G7 |
| citadel.swarm.v1.SwarmService/UpdateConfigLabels | citadel.swarm.v1.UpdateSwarmResourceLabelsRequest | google.protobuf.Empty | unary | SWARM_CONFIG_UPDATE | 108 | denied | connectors/docker/finite.rs:update_swarm_material | G7 |
| citadel.swarm.v1.SwarmService/DeleteConfig | citadel.swarm.v1.DeleteSwarmConfigRequest | google.protobuf.Empty | unary | SWARM_CONFIG_DELETE | 109 | denied | connectors/docker/finite.rs:delete_swarm_config | G7 |
| citadel.swarm.v1.SwarmService/CreateSystemService | citadel.swarm.v1.CreateSystemSwarmServiceRequest | citadel.swarm.v1.SwarmServiceMutationResponse | unary | SWARM_SYSTEM_SERVICE_CREATE | 116 | denied | connectors/docker/finite.rs:create_swarm_service | G7 |
| citadel.swarm.v1.SwarmService/UpdateSystemService | citadel.swarm.v1.UpdateSystemSwarmServiceRequest | citadel.swarm.v1.SwarmServiceMutationResponse | unary | SWARM_SYSTEM_SERVICE_UPDATE | 117 | denied | connectors/docker/finite.rs:update_swarm_service | G7 |
| citadel.volumes.v1.VolumeService/List | citadel.volumes.v1.ListVolumesRequest | citadel.volumes.v1.ListVolumesResponse | unary | VOLUME_LIST | 50 | allowed | connectors/docker/finite.rs:list_volume_models | G2 |
| citadel.volumes.v1.VolumeService/Inspect | citadel.volumes.v1.InspectVolumeRequest | citadel.shared_models.v1.VolumeResponse | unary | VOLUME_INSPECT | 51 | allowed | connectors/docker/finite.rs:inspect_volume | G1 |
| citadel.volumes.v1.VolumeService/Create | citadel.volumes.v1.CreateVolumeRequest | citadel.shared_models.v1.VolumeResponse | unary | VOLUME_CREATE | 52 | restore-only | connectors/docker/finite.rs:create_volume | G1 |
| citadel.volumes.v1.VolumeService/Remove | citadel.volumes.v1.RemoveVolumeRequest | citadel.volumes.v1.RemoveVolumeResponse | unary | VOLUME_DELETE | 53 | restore-only | connectors/docker/finite.rs:delete_volume | G1 |
<!-- /operations -->

Container inventory observations include mount points and an explicit
`mountsObserved` flag. Core persists these observations with the container's
owner-node identity and freshness fence for backup coverage reads. An observed
empty mount list is distinct from missing inventory; coverage reads do not issue
additional Container inspection commands.

### Container health observations

Container collection can request health metadata with the additive
`ListContainersRequest.observe_health` flag. Local, Direct and Edge collection
use bounded read-only inspection of the exact container on its owning daemon.
`ContainerMessage.healthObservation` contains typed summary evidence, never probe
commands or output. Older Agents omit it, which Core presents as Unknown.

Health/OOM notifications trigger a targeted observation; ordinary lifecycle
notifications keep their state-only fast path. Background container observations
run through the existing bounded resource lanes every minute; evidence expires
after three minutes. Core preserves the inspection timestamp, fences stream gaps, and
retains existing generation and Edge-session checks. Expired or future-dated
health evidence is Unknown; Agent and Core clocks should remain synchronized. Statistics and heartbeats
cannot renew health evidence. No new Agent command or Swarm-worker allowlist entry
is needed.

Explicit inspection adds effective health-check settings and at most five recent
probe results (4,096 characters per output). Core applies Inspect authorization
and redaction; list, realtime summary, and retained health history omit this
sensitive detail. `healthMetadataObserved` distinguishes absent metadata from a
confirmed absent check on older Agents.

The browser uses the existing `container-info` stream for current health. The
on-demand `container-health:<container UUID>` group sends bounded persisted history
as `ReceiveContainerHealthHistory` (`resourceId`, `entries`), including on join and
reconnect. It requires the owning Platform's Read permission, matching the HTTP
history endpoint; workload-only access to container-info does not grant this read.
Committed container invalidations refresh the snapshot; statistics do not.
Neither stream performs Docker inspection or includes probe commands/output.
Explicit probe details remain an Inspect-authorized HTTP read on user request.

Core stores actual periodic health observations for enabled checks at most once
per UTC minute, in addition to immediate transitions. NoCheck, Disabled, and
Unknown configurations do not generate periodic samples; current observations
and configuration/lifecycle transitions remain persisted. The browser skips the
history group for a currently confirmed NoCheck or Disabled status. Statistics, unchanged replay timestamps,
and stale or future observations cannot generate periodic samples. Retention is
seven days, up to 10,080 samples and 200 transitions per container, with the
existing shared 100,000-record installation cap. This uses an additive Core
migration and requires no Agent wire change. Earlier transitions are preserved;
no periodic samples are backfilled.

History snapshots keep the latest hour and retained transitions exact. Older
samples are grouped into five-minute buckets for the latest day and thirty-minute
buckets before that, partitioned by state, check configuration, health result,
OOM and exit evidence. Each entry includes `sampleCount` and `firstObservedAt`
(Unix seconds); `observedAt` is the latest actual observation in the group. These
bounds describe observations, not continuous coverage. No healthy grouping can
absorb an unhealthy sample. Grouping occurs in the persistence reader before HTTP
or realtime serialization; range changes in the browser need no additional reads.
