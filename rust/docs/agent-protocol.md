# Agent protocol and deployment profiles

This document records the Direct/Edge operation contract and restricted Swarm-node
policy. Contract and Agent tests validate the inventory against generated descriptors
and the runtime allowlist.

## Operations

There are 68 Direct RPCs and 68 corresponding nonzero Edge command kinds.
`EdgeAgentService/Connect` is a separate bidirectional connection lifecycle:
`citadel.edge.v1.AgentEnvelope` to `citadel.edge.v1.CoreEnvelope`, protocol 2.
Core intake, Agent connection lifecycle and all ordinary/Build Pool command kinds
are implemented. An exhaustive Rust match and decoder coverage test enforce the
command mapping independently of the advertised capabilities.
`EDGE_COMMAND_KIND_UNSPECIFIED = 0` and unknown command numbers must be rejected.

The table is checked against compiled protobuf descriptors by
`citadel-contracts/tests/agent_protocol.rs`, including message types, streaming,
command numbers, missing and duplicate entries. Runtime references below are
relative to `crates/infrastructure/adapters/src`; they identify reusable pieces,
with the behavior checks described below. Every operation also requires G0,
G1 and, in the Swarm-node profile, G8.

Ordinary Edge and Edge Build Pool use the full dispatcher. The Swarm column
records the reference command policy: allowed, denied, helper-only or
restore-only. Capability advertisements are **not** the authorization allowlist.

<!-- operations -->
| RPC | Request | Response | Stream | Edge command | Number | Swarm | Reuse | Gap |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| citadel.containers.v1.ContainerService/List | citadel.containers.v1.ListContainersRequest | citadel.containers.v1.ListContainersResponse | unary | CONTAINER_LIST | 10 | allowed | connectors/docker/finite.rs:list_container_models | G2 |
| citadel.containers.v1.ContainerService/Start | citadel.containers.v1.ContainerIds | google.protobuf.Empty | unary | CONTAINER_START | 14 | allowed | connectors/docker/finite.rs:start_container | G1 |
| citadel.containers.v1.ContainerService/Stop | citadel.containers.v1.ContainerIds | google.protobuf.Empty | unary | CONTAINER_STOP | 15 | allowed | connectors/docker/finite.rs:change_container_state | G1 |
| citadel.containers.v1.ContainerService/Pause | citadel.containers.v1.ContainerIds | google.protobuf.Empty | unary | CONTAINER_PAUSE | 16 | allowed | connectors/docker/finite.rs:change_container_state | G1 |
| citadel.containers.v1.ContainerService/Unpause | citadel.containers.v1.ContainerIds | google.protobuf.Empty | unary | CONTAINER_UNPAUSE | 17 | allowed | connectors/docker/finite.rs:change_container_state | G1 |
| citadel.containers.v1.ContainerService/Restart | citadel.containers.v1.ContainerIds | google.protobuf.Empty | unary | CONTAINER_RESTART | 18 | allowed | connectors/docker/finite.rs:change_container_state | G1 |
| citadel.containers.v1.ContainerService/Delete | citadel.containers.v1.DeleteContainerRequest | google.protobuf.Empty | unary | CONTAINER_DELETE | 19 | allowed | connectors/docker/finite.rs:delete_container_with_options | G1 |
| citadel.containers.v1.ContainerService/Inspect | citadel.containers.v1.InspectContainerRequest | citadel.shared_models.v1.InspectContainerResponse | unary | CONTAINER_INSPECT | 12 | allowed | connectors/docker/transport.rs:inspect_container_document | G1 |
| citadel.containers.v1.ContainerService/Create | citadel.containers.v1.CreateContainerRequest | citadel.containers.v1.CreateContainerResponse | unary | CONTAINER_CREATE | 13 | helper-only | connectors/docker/finite.rs:create_container | G1 |
| citadel.containers.v1.ContainerService/Exec | citadel.containers.v1.ExecClientMessage | citadel.containers.v1.ExecServerMessage | bidi | CONTAINER_EXEC | 22 | allowed | connectors/docker/transport.rs:execute_terminal | G3 |
| citadel.containers.v1.ContainerService/ExecBinary | citadel.containers.v1.ExecBinaryRequest | citadel.containers.v1.ExecServerMessage | server | CONTAINER_EXEC_BINARY | 23 | helper-only | connectors/docker/transport/binary_exec.rs:execute_binary | G3 |
| citadel.containers.v1.ContainerService/StreamContainerLogs | citadel.containers.v1.ContainerLogRequest | citadel.containers.v1.ContainerLogResponse | server | CONTAINER_LOGS_STREAM | 11 | allowed | connectors/docker/transport.rs:open_logs | G4 |
| citadel.containers.v1.ContainerService/StreamContainersStats | citadel.containers.v1.StreamContainersStatsRequest | citadel.containers.v1.ContainersStatsResponse | server | CONTAINERS_STATS_STREAM | 21 | allowed | connectors/docker/local_sampler.rs:sample | G4 |
| citadel.containers.v1.ContainerService/StreamContainerStats | citadel.containers.v1.StreamContainerStatsRequest | citadel.shared_models.v1.ContainerMessage | server | CONTAINER_STATS_STREAM | 20 | allowed | connectors/docker/transport.rs:container_stats | G4 |
| citadel.deployments.v1.DeploymentService/Apply | citadel.deployments.v1.ApplyDeploymentRequest | citadel.deployments.v1.ApplyDeploymentResponse | unary | DEPLOYMENT_APPLY | 80 | denied | connectors/docker/deployments.rs:apply_container_config | G6 |
| citadel.images.v1.ImageService/Get | citadel.images.v1.GetImageRequest | citadel.shared_models.v1.ImageReply | unary | IMAGE_GET | 30 | denied | connectors/docker/finite.rs:list_images | G1 |
| citadel.images.v1.ImageService/List | citadel.images.v1.ListImagesRequest | citadel.shared_models.v1.ListImageResponse | unary | IMAGE_LIST | 31 | allowed | connectors/docker/finite.rs:list_images | G1 |
| citadel.images.v1.ImageService/Delete | citadel.images.v1.DeleteImageRequest | citadel.images.v1.DeleteImageResponse | unary | IMAGE_DELETE | 33 | denied | connectors/docker/finite.rs:delete_image | G1 |
| citadel.images.v1.ImageService/Inspect | citadel.images.v1.InspectImageRequest | citadel.images.v1.InspectImageResponse | unary | IMAGE_INSPECT | 32 | allowed | connectors/docker/finite.rs:inspect_image | G1 |
| citadel.images.v1.ImageService/History | citadel.images.v1.HistoryImageRequest | citadel.images.v1.HistoryImageResponse | unary | IMAGE_HISTORY | 34 | denied | connectors/docker/finite.rs:image_history | G1 |
| citadel.images.v1.ImageService/GetExposedPorts | citadel.images.v1.GetExposedPortsRequest | citadel.images.v1.GetExposedPortsResponse | unary | IMAGE_EXPOSED_PORTS | 35 | denied | connectors/docker/finite.rs:inspect_image | G1 |
| citadel.images.v1.ImageService/DistributionInspect | citadel.images.v1.DistributionInspectRequest | citadel.images.v1.DistributionInspectResponse | unary | IMAGE_DISTRIBUTION_INSPECT | 36 | denied | connectors/docker/finite.rs:distribution_inspect_authenticated | G1 |
| citadel.images.v1.ImageService/CheckBuildHost | google.protobuf.Empty | citadel.images.v1.CheckBuildHostResponse | unary | IMAGE_CHECK_BUILD_HOST | 40 | denied | connectors/docker/transport.rs:version | G5 |
| citadel.images.v1.ImageService/Pull | citadel.images.v1.PullImageRequest | citadel.images.v1.PullImageResponse | server | IMAGE_PULL_STREAM | 37 | denied | connectors/docker/transport.rs:pull_image_with_options | G5 |
| citadel.images.v1.ImageService/Build | citadel.images.v1.BuildImageRequest | citadel.images.v1.ImageBuildResponse | server | IMAGE_BUILD_STREAM | 38 | denied | external/builds/runtime.rs:build | G5 |
| citadel.images.v1.ImageService/Push | citadel.images.v1.PushImageRequest | citadel.images.v1.ImageBuildResponse | server | IMAGE_PUSH_STREAM | 39 | denied | external/builds/runtime.rs:push | G5 |
| citadel.networks.v1.NetworkService/List | citadel.networks.v1.ListNetworksRequest | citadel.networks.v1.ListNetworksResponse | unary | NETWORK_LIST | 60 | allowed | connectors/docker/finite.rs:list_networks_filtered | G2 |
| citadel.networks.v1.NetworkService/Create | citadel.networks.v1.CreateNetworkRequest | citadel.networks.v1.CreateNetworkResponse | unary | NETWORK_CREATE | 62 | denied | connectors/docker/finite.rs:create_network | G1 |
| citadel.networks.v1.NetworkService/Delete | citadel.networks.v1.DeleteNetworkRequest | citadel.networks.v1.DeleteNetworkResponse | unary | NETWORK_DELETE | 63 | denied | connectors/docker/finite.rs:delete_network | G1 |
| citadel.networks.v1.NetworkService/Inspect | citadel.networks.v1.InspectNetworkRequest | citadel.networks.v1.InspectNetworkResponse | unary | NETWORK_INSPECT | 61 | allowed | connectors/docker/finite.rs:inspect_network | G1 |
| citadel.platforms.v1.PlatformService/GetPlatformInfo | google.protobuf.Empty | citadel.shared_models.v1.PlatformInfoResponse | unary | PLATFORM_GET_INFO | 2 | allowed | connectors/docker/finite.rs:info | G1 |
| citadel.platforms.v1.PlatformService/CheckHealth | google.protobuf.Empty | citadel.platforms.v1.CheckHealthResponse | unary | PLATFORM_CHECK_HEALTH | 1 | allowed | connectors/docker/transport.rs:ping | G1 |
| citadel.platforms.v1.PlatformService/Prune | citadel.platforms.v1.PruneRequest | citadel.platforms.v1.PruneResponse | unary | PLATFORM_PRUNE | 5 | denied | connectors/docker/finite.rs:prune_resources | G1 |
| citadel.platforms.v1.PlatformService/StreamPlatformStats | citadel.platforms.v1.PlatformStatsRequest | citadel.platforms.v1.PlatformStatsResponse | server | PLATFORM_STATS_STREAM | 3 | allowed | connectors/docker/local_sampler.rs:sample | G4 |
| citadel.platforms.v1.PlatformService/StreamDaemonEvent | google.protobuf.Empty | citadel.platforms.v1.DaemonEventResponse | server | PLATFORM_DAEMON_EVENTS_STREAM | 4 | allowed | connectors/docker/transport.rs:events | G4 |
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
| citadel.swarm.v1.SwarmService/GetServiceLogs | citadel.swarm.v1.SwarmLogsRequest | citadel.swarm.v1.SwarmLogsResponse | unary | SWARM_SERVICE_LOGS | 102 | denied | connectors/docker/transport.rs:open_resource_logs | G4 |
| citadel.swarm.v1.SwarmService/ListTasks | citadel.swarm.v1.ListSwarmTasksRequest | citadel.swarm.v1.ListSwarmTasksResponse | unary | SWARM_TASK_LIST | 94 | denied | connectors/docker/finite.rs:list_swarm_tasks_filtered | G2 |
| citadel.swarm.v1.SwarmService/InspectTask | citadel.swarm.v1.InspectSwarmTaskRequest | citadel.swarm.v1.SwarmTaskMessage | unary | SWARM_TASK_INSPECT | 95 | denied | connectors/docker/finite.rs:inspect_swarm_task | G1 |
| citadel.swarm.v1.SwarmService/GetTaskLogs | citadel.swarm.v1.SwarmLogsRequest | citadel.swarm.v1.SwarmLogsResponse | unary | SWARM_TASK_LOGS | 103 | denied | connectors/docker/transport.rs:task_logs | G4 |
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

## Behavior checks

- **G0 — Docker endpoint:** use the configured endpoint for HTTP, streams, exec and CLI work; reject unsupported transports and never fall back to another daemon.
- **G1 — Agent boundaries:** preserve protobuf presence, defaults and complete responses; enforce authentication, deadlines, cancellation and error translation.
- **G2 — Lists and filters:** preserve request filters, limits, Swarm scope, empty lists and volume warnings/usage.
- **G3 — Exec:** preserve binary framing, TTY, environment, user, resize, half-close and exit status; bound input and output.
- **G4 — Streams and sampling:** honor sampling intervals, bounded queues, cancellation and stream completion; propagate failure without reporting success.
- **G5 — Images/builds:** validate context paths and build options; isolate credentials, redact output and support independent build/push operations.
- **G6 — Apply:** execute deployment and stack operations without a database; preserve resource settings, registry auth, secret files, progress and cleanup.
- **G7 — Swarm mutations:** enforce version concurrency, labels, restart updates and managed/system-service constraints.
- **G8 — Swarm-node guards:** enforce the allowlist before Docker calls; validate helper ownership, images, privileges, mounts and commands; remove only unused owned restore volumes.

## Deployment profiles

| Deployment | Mode selection | Profile selection | Wire profile | Core resource | Listener | Template |
| --- | --- | --- | --- | --- | --- | --- |
| Regular inbound | unset/default | unused | none | Platform | inbound Direct RPCs and health | `.env.agent.example` |
| Inbound Build Pool | unset/default | unused | none | BuildAgentPool | same Direct RPCs and health | `.env.build-agent.example` |
| Edge | edge | edge-agent/default | Ordinary | Platform | loopback health only | `.env.edge.example` |
| Edge Build Pool | edge | edge-build-agent | Ordinary | BuildAgentPool | loopback health only | `.env.edge-build-agent.example` |
| Swarm-node | edge | swarm-node | SwarmNode | Platform plus node identity | loopback health only | generated service environment/bootstrap |

The four templates are preserved at the Rust workspace root and checked for
required assignments and separation of credentials. They describe deployment
inputs. There is no `build` transport mode. Edge Build Pool shares the ordinary
connection lifecycle and command dispatcher, and persists `BuildAgentPool` session
identity. Swarm-node never enrolls as
an ordinary standalone Edge Agent.

Configuration tests cover all five profiles, invalid combinations, persisted
Platform/Build Pool identity and injected Swarm identity.

## Environment

Read configuration once at startup. Templates are passed with Docker
`--env-file`; the Agent must not add implicit dotenv loading. Blank/default
behavior is part of the configuration contract; validation changes need tests.

| Variable | Default / requirement | Applies to |
| --- | --- | --- |
| CITADEL_AGENT_MODE | only case-insensitive `edge` selects Edge; otherwise Direct; no trimming in reference | all |
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
| DOCKER_HOST | platform default: unix:///var/run/docker.sock on Linux; named pipe on Windows | all |
| CITADEL_HOST_ROOT | /host; optional host filesystem mount for disk usage | all |
| RUST_LOG | citadel_agent=info; optional tracing filter | all; Rust host diagnostics |

Edge ignores Direct TLS fields and does not require `HUB_PUBLIC_KEY`. Profile,
bootstrap metadata and optional CA inputs are trimmed. Key/identity paths use
unset-only defaults; explicit empty values must not silently select defaults.
Changing the ordinary enrollment-token fingerprint resets its persisted identity
and key; Swarm bootstrap remains authoritative for Swarm-node identity.
`ASPNETCORE_ENVIRONMENT` is intentionally absent: it configures a runtime that the
Rust Agent does not use. Internal queue sizes are not operator template settings.

## Authentication and lifecycle baseline

Direct signatures cover timestamp, nonce, method path and the SHA-256 of the
serialized request. Required binary metadata lengths are signature 64, timestamp
8, nonce 16 and hash 32 bytes; timestamp skew is ±60 seconds. The signed first
Exec message must be `Open` and arrive within 10 seconds before Docker work
begins. The verified opening message must then be replayed to the handler.
Resize buffering must not allow an unauthenticated resize-first Direct request.

Reference nonce retention expires at signed timestamp plus skew plus one second.
The Rust implementation must add the specification's explicit cache bound without
prematurely evicting accepted nonces inside the replay window. Test saturation,
replays, changed payloads/methods and invalid first stream messages.

Edge limits: 16 MiB payload, 16 active commands per session, input queue 256,
outgoing queue 512, heartbeat 30 seconds, keepalive 30 seconds with 10-second
timeout. Reconnect starts at 1 second, caps at 60 seconds and uses jitter. Test
queue saturation, session changes, cancellation, disconnected streams and
identity persistence. These are implementation limits, not new environment knobs.

## Capability advertisements

Ordinary Edge and Edge Build Pool advertise the same 49 command capabilities.
Swarm-node advertises 19. Preserve these strings until deliberate protocol review;
they are discovery hints, not a substitute for dispatcher authorization.

Ordinary / Edge Build Pool:

```text
platform.checkHealth
platform.getInfo
platform.stats
platform.events
platform.prune
containers.list
containers.logs
containers.inspect
containers.create
containers.patch
containers.delete
containers.stats
containers.exec
containers.execBinary
images.get
images.list
images.inspect
images.delete
images.history
images.exposedPorts
images.distributionInspect
images.pull
images.build
images.push
images.checkBuildHost
volumes.list
volumes.inspect
volumes.create
volumes.delete
networks.list
networks.inspect
networks.create
networks.delete
stacks.apply
deployments.apply
swarm.nodes.list
swarm.nodes.inspect
swarm.services.list
swarm.services.inspect
swarm.services.logs
swarm.tasks.list
swarm.tasks.inspect
swarm.tasks.logs
swarm.networks.list
swarm.networks.inspect
swarm.secrets.list
swarm.secrets.inspect
swarm.configs.list
swarm.configs.inspect
```

Swarm-node:

```text
platform.checkHealth
platform.getInfo
platform.stats
platform.events
containers.list
containers.logs
containers.inspect
containers.patch
containers.delete
containers.stats
containers.exec
containers.createHelper
containers.execBinaryHelper
images.list
images.inspect
volumes.list
volumes.inspect
networks.list
networks.inspect
```

The reference dispatcher permits guarded backup-restore volume creation/deletion
without advertising general `volumes.create`/`volumes.delete` in Swarm-node mode.
It also permits constrained backup helpers in addition to volume-browser helpers.
Preserve these guards; do not derive an unrestricted allowlist from capability
names or remove working backup behavior based on the shorter specification list.

## Direct implementation

`agent/src/direct` implements all 68 RPCs in the table above. G0/G1 authentication
and wire boundaries are active for Direct mode: Ed25519, timestamp skew, body hash,
atomic replay rejection, map-field byte preservation, size limits and deadlines.
Health remains unsigned. HTTP/2 works with the shared health listener over HTTP
and TLS; transport does not add a database dependency.

Direct handlers map G2–G7 onto the shared runtime: optional values and 64-bit
fields, filtered inventory, statistics/events, logs, interactive/binary exec,
image build/pull/push, deployment observation, Compose/Swarm apply and Swarm
mutations. Private staging and credential files have drop cleanup; successful
Compose mounts retain their required secret files. Swarm conversion failures
restore Compose without deleting volumes.

The automated suite checks registration/authentication for every RPC, signed
Core-client map requests, representative wire behavior, transport rejection,
stream cleanup and TLS. The opt-in real Docker fixture covers Core platform info,
container inventory, statistics/events, container lifecycle, both exec modes,
logs, image inspection, network/volume operations and deployment apply.
The compatibility fixture exercises live Swarm-cluster mutations and an
authenticated disposable registry with the packaged Agent. Mixed-version binary
interoperability remains an explicit gate; registration tests alone do not
establish it.

The Swarm-node dispatcher implements G8 with an explicit allowlist tested against every row above.
Node IDs must match the configured identity. Helper creation requires platform
ownership and exact mount, privilege and command constraints. Binary exec verifies
the inspected helper configuration and executes by immutable container ID. Mount
driver options, conflicting ownership labels and arbitrary shell/environment
overrides are rejected. Backup exec accepts Core's supported S3 restic commands
and flags; restore-volume creation/deletion checks ownership and container use.
The packaged Rust helper passed live Docker listing, binary streaming, symlink
rejection and cleanup tests through the restricted dispatcher. This test uses a
node identity fixture, not a live Swarm cluster.

