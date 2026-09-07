# Phase 7: external execution completion report

Status: **Phase 7 implementation and defined exit checks complete (2026-09-07)**.
The current closure record is **Completion pass (2026-09-07)** below. Earlier
checkpoint notes are retained as history, not outstanding work. This is not a
release-readiness claim: Phase 8's full-image/browser/security/soak gates remain.

## Implemented

- The existing Edge protobuf is generated from its pinned Contracts-submodule
  source. Rust Core accepts the .NET enrollment and Ed25519 reconnect protocol.
  This is not a rewrite of the Agent or its payload contracts.
- Edge Platform creation persists an Offline resource without contacting a
  nonexistent daemon. The existing enrollment/status/revoke HTTP routes enforce
  Platform permissions and retain the existing UI payloads. Tokens are returned
  once with `no-store`; PostgreSQL stores only their hashes.
- Single-use enrollment is transactional. Expired, consumed, revoked and
  mismatched identities fail closed. Node enrollment verifies the installation,
  cluster, current manager-owned Service and Task, and exact eligible Node.
- Reconnect replaces its previous session. Late old-session heartbeat,
  disconnect and inventory writes cannot overwrite the replacement. Revocation
  and inventory persistence are ordered transactionally.
- Commands use bounded queues, deadlines and cancellation, with a shared 32 MiB
  queued-payload budget, 16 concurrent commands and 8 streams per session.
  A consumer that overflows its output allowance fails; it does not accumulate
  an unbounded backlog. Dropped commands send cancellation; if cancellation
  cannot be queued the session closes. Mutations are not replayed on reconnect.
- Ordinary Edge Platform inventory refreshes on daemon events, coalesces bursts,
  and performs a periodic full reconciliation. Full scans have a shared
  concurrency bound and successful commits publish existing realtime events.
- Platform-backed builds transport immutable Git archives and BuildKit Secrets
  through Edge, followed by canonical push commands. Build output and errors
  redact resolved Secret values and Registry credentials.
- S3 Volume backup/restore helpers can execute on an exact Node Edge session.
  The connected Local/regular-Agent/ordinary-Edge manager is used only after
  verifying its live Node ID. No worker fallback to another daemon is allowed.
  Helper cleanup uses an independent cancellation token after execution fails.
- Alert subscriptions emit `AlertEventReceived` for newly observed authorized
  events without replaying historical notifications at connection time.
- Git synchronization no longer drops its in-flight execution when the
  scheduler's one-minute tick fires.

## Additional completion work (2026-09-05)

- Deployment Apply, Stack Apply, managed Swarm Service create/update/delete,
  and image pull now route to the selected ordinary Edge Platform. Direct and
  Edge Agents share the Deployment/Stack protobuf mapping. There is no local
  daemon fallback and no replay of a disconnected mutation.
- Stack output retention is bounded to 512 KiB while all status frames are
  drained. A failed deployment remains failed even if later rollback/cleanup
  reports success. Cancellation and a disconnected stream cannot report success.
- Edge Network/Volume reads and mutations use the existing authorized HTTP
  routes and shared Direct-Agent mapping. The task-statistics validation path
  can inspect a Task through the Edge manager.
- Node-profile sessions supervise independent inventory and statistics streams.
  Container snapshots and samples persist against `(Platform, Node, Docker ID)`;
  a worker cannot update manager statistics or another Node with the same Docker
  container ID. Snapshot time is captured before the scan, and an older snapshot
  cannot delete a container observed by a newer event. Replaced/revoked sessions
  cannot persist data. Disconnect marks only the affected Node's projections stale.
  Node Image/Volume/Network projection ingestion was added in the follow-up below.
- Build Pool Edge enrollment/status/revoke routes retain .NET operation IDs and
  permissions. Build Pool enrollment does not mount the host filesystem. Queued
  Pool builds use that Pool's session, and the short claim transaction enforces
  the persisted concurrent-build limit across workers. Pool changes publish to
  the Build Pool realtime group. See the provider-scope correction below.
- Test fixtures now use unique Docker Service IDs and Platform addresses so
  the database-backed gate can run repeatedly without fixture collisions.
- The real PostgreSQL 18 `citadel_system_recovery` test passed: create a private
  bundle with `pg_dump`, restore it into a separate clean target with
  `pg_restore`, and verify persisted state. This verifies database disaster
  recovery, not the remote Restic/RustFS or multi-node acceptance matrix.

No React feature changes or database migrations were needed for this follow-up.
OpenAPI and its generated contract inventory were regenerated.

## Running Edge locally

The REST API uses `Transport__ApiPort` (default 8000). Edge gRPC uses the separate
`Transport__EdgeGrpcPort` (default 8001), matching the existing configuration.
Set `EdgeAgent__PublicGrpcUrl` to the HTTP/2 endpoint reachable **from the Agent**;
`localhost` inside an Agent container is not Core. Configure the existing TLS or
trusted reverse-proxy mode for production. Do not expose disabled transport
security to an untrusted network.

Create an Edge Platform, request its enrollment from the Platform screen and
use the returned instructions. `CITADEL_EDGE_AGENT_IMAGE` can point to the .NET
Agent image under test. Normal .NET Agent capabilities and protobuf compatibility
are required. The development container forwards port 8001 after reopening it.

## Test mapping and limits

| .NET reference | Rust coverage |
| --- | --- |
| `EdgeAgentSessionRegistryTests` | replacement, exact-node lookup, old disconnect isolation |
| `EdgeAgentCommandRouterTests` | cancellation, deadline, concurrency/byte/queue bounds, node command restrictions |
| `EdgeAgentTests` enrollment lifecycle | authorized HTTP creation/enrollment/status/revoke and hashed, one-use PostgreSQL enrollment |
| Edge acceptance protocol client | real HTTP/2 enrollment, invalid signature rejection, signed reconnect, command completion and revocation |
| Node enrollment identity checks | PostgreSQL-backed installation/Service/Task/Node validation; stale, wrong-task and manager enrollment rejection |
| `EdgeAgentConnectorTests` execution contracts | Build/Push protobuf, binary output/exit semantics, missing unary response and pre-cancel rejection |
| backup routing and cleanup | exact-node helper cleanup on failure with a second node receiving no commands |
| `StackServiceTests` rollback and failed Apply | bounded retained output, sticky failure despite later cleanup success, stream cancellation/disconnect |
| `SwarmNodeDataPlaneJobTests.ReconciliationSnapshot_ShouldNotDeleteContainerObservedByNewerEvent` | PostgreSQL node isolation, snapshot ordering, statistics persistence, stale-session and late-disconnect rejection |
| Build Pool enrollment and Build concurrency | authorized HTTP lifecycle and two simultaneous PostgreSQL claims with a one-slot Pool |
| Container statistics realtime mapping | identical Docker IDs on different Nodes cannot overwrite each other's UI samples |

Run `rust/scripts/Test-Phase7AExternalExecution.ps1` for the repeatable gate.
These tests include real PostgreSQL and a real HTTP/2 transport, but the Agent
command responders are test peers. They do **not** prove that a released .NET
Agent, Docker, registry and RustFS perform a complete backup/restore together.

Verified after the additional completion work on 2026-09-05:

- Workspace library tests: 301 passed.
- Edge session tests: 7 passed.
- Targeted PostgreSQL/HTTP/HTTP2 suites: 38 passed (10 adapter execution and
  inventory tests, plus 28 server resource, Platform, lookup and statistics tests).
- Real PostgreSQL 18 system recovery: 1 passed using separate disposable source
  and target databases and matching `pg_dump`/`pg_restore` tools.
- Adapter/server library and test Clippy: no warnings.
- Formatting, server compilation through the test builds, generated OpenAPI
  check (292 full / 240 public operations), and baseline database-schema
  verification passed. The initial schema was not changed.

The tests used disposable databases, not the running development database.
No .NET Agent candidate image or full external-service matrix was run.

## Inventory and restore follow-up (2026-09-06)

- Node snapshots now atomically persist Containers, Images, Volumes and local
  Networks. A per-node scan watermark rejects late snapshots even after a row
  has been deleted; malformed batches roll back the whole snapshot. Stable
  image IDs, node isolation, session fencing and stale-on-disconnect behavior
  are covered against PostgreSQL. Cluster overlay Networks are not duplicated.
- Existing authorized resource lists include node projections and publish the
  existing `SwarmNodeLocalResourcesUpdated` UI event. Volume/Network inspect
  selects the exact node session; manager fallback requires a matching live
  node identity. Tests cover capabilities, denial before dispatch, missing
  worker rejection, realtime payloads and retaining manager Volumes. No React
  changes or schema migrations were required. This does not implement all
  node-local mutation, image-detail or volume-browsing endpoints.
- Incremental Build output uses bounded/redacted retention and the existing
  Build log realtime contract. Library, persistence and HTTP tests cover it;
  this is not evidence for released-Agent end-to-end Build execution.
- Real Docker + Restic 0.18.1 + PostgreSQL acceptance exposed and now covers
  volume-root restoration. Local and Agent snapshots contain `/data` and
  `/source` respectively: restore validates snapshot metadata and selects the
  recorded subtree, regardless of the target connector. Unsupported or
  ambiguous roots fail rather than restoring into an unexpected subdirectory.
  Agent command-peer tests verify both roots, exact target routing and helper
  cleanup after invalid metadata. Local acceptance verifies actual restored
  bytes, persisted results and rejection of unapproved overwrite.
- The development Docker socket proxy truncated delayed `docker run` output.
  Direct host-socket access fixes it without running the workspace as root or
  changing host socket permissions. Rebuild the development container to use
  the updated configuration. `Test-Phase7LocalBackup.ps1` also works with an
  existing workspace and creates only disposable, uniquely named fixtures.

Additional .NET references: `SwarmPersistenceTests` node-local identity tests,
`SwarmNodeDataPlaneJobTests` late-snapshot tests, `BackupRestoreRunExecutionTests`,
and the volume round trip from `SwarmBackupCompatibilityTests`. The new Local
acceptance is deliberately not claimed as the full Swarm compatibility port.

Verified on 2026-09-06:

- Workspace library tests: 308 passed.
- Targeted PostgreSQL/HTTP/HTTP2 suites: 32 passed.
- Process-runner and Edge session suites: 16 passed.
- Real Local Docker/Restic/PostgreSQL backup acceptance: 1 passed, including
  reruns through `Test-Phase7LocalBackup.ps1` with fresh disposable databases.
- Adapter/server library and test Clippy, formatting, Compose validation and
  regenerated OpenAPI check passed (292 full / 240 public operations).

No frontend feature files were changed and no Citadel images were built. The
released-Agent/provider/multi-node matrix was not run. All database fixtures
were separate from the running development database.

## Build Pool, rebind and external-process follow-up (2026-09-06)

- Build Pool providers now follow .NET: `SelfManagedVm` and `AwsEc2`, not the
  invented `GenericEdge` / `HetznerCloud` contracts. The .NET executor and Test
  handler support self-managed pools only. AWS provisioning is not existing
  .NET behavior to migrate; adding it needs a separate product slice.
- Implemented the existing pool Test, partial update, rename and metadata HTTP
  operations. Test claims are bounded and survive request cancellation; stale
  completions cannot overwrite a replacement claim. Create, update, rename,
  delete and Test activities commit with their resource changes. Activity
  snapshots use the .NET field shape. Archived pools are excluded from reads.
- Pool lists now return `pools` and capabilities, as the existing UI expects.
  HTTP and realtime pool rows share capability mapping with batched ACL reads.
  Edge revocation requires Execute, not just Write. This does not claim full
  Build Project/Pool tag-filter and background-health parity.
- Self-managed inbound checks/builds connect to the selected pool endpoint,
  retaining the configured Agent signing identity and insecure-transport policy.
  No unbounded endpoint cache or local-daemon fallback is introduced. Core still
  requires its existing configured signed-Agent transport for inbound pools.
- Node rebind requires the same signing key, daemon and cluster; current owned
  Service/Task identity; an absent old Node beyond the ten-minute grace period;
  and non-stale membership. A reconnect Hello does not change persistence.
  Membership is checked again after proof, under the binding lock, before
  committing. Late old-node sessions cannot reconnect or overwrite the new one.
  Enrollment also validates the cluster ownership label and retains observed
  hostname/role/Service/Task identity.
- Added real Deno and Shoutrrr acceptance. It exposed a persisted raw run-token
  leak, now fixed by exact-token redaction in addition to assignment redaction.
  Credential-bearing run directories are private on Unix. Tests exercise API
  calls, arguments, exit code 7, bounded output, denied filesystem access,
  timeout, cancellation/reaping, source cleanup, persisted failure alerts, and
  an actual Shoutrrr HTTP 503/retry/success round trip. Nonzero exit reasons now
  match .NET instead of treating all stderr as the error summary.
- Added typed Automation Activity domain contracts and transactional create,
  update, rename, delete, queue, start, success, failure, timeout, cancellation,
  rejection and interrupted-run events. A rejected run leaves the existing
  claim intact; late completion cannot duplicate events or raise a new failure
  alert. Configuration updates use row-version fencing, and audit snapshots
  mask webhook secrets. The metadata endpoint updates descriptions without a
  configuration Activity, matching .NET. PostgreSQL tests also prove rollback
  when Activity persistence fails. The HTTP progress stream remains open.

| .NET reference | New Rust evidence |
| --- | --- |
| `BuildAgentPoolCommandTests` capability checks | `phase7_resources_http/build_pools.rs`, `build_pool_checker` unit tests and signed `agent_build_pool` HTTP/2 test |
| `BuildEndpointTests.BuildAgentPoolEndpoints_ShouldPersistLifecycleAndEdgeEnrollment` | Test/edit/rename/metadata/archive/Edge persistence, activities, concurrency, capability and permission cases in `phase7_resources_http` (tag filtering remains open) |
| `SwarmNodeAgentLifecycleTests` reconnect/rebind cases | PostgreSQL `edge_transport` checks old membership, grace, key/daemon/cluster/Task identity, stale membership, proof-before-write and old-session rejection |
| `AutomationServicesTests` redaction/buffering/coordinator cases | Existing library tests plus real Deno output/cancellation checks |
| `AutomationActionIntegrationTests` create/update/rename/delete, metadata, cancellation, rejection and recovery cases | `resources_http` and `automation_execution` assert persisted resource/run state and typed Activities, CAS and audit rollback |
| `AutomationActionIntegrationTests` success, nonzero exit and failure-alert cases | `automation_external_acceptance` with real Deno, PostgreSQL and Shoutrrr, including terminal Activities; HTTP streaming and tag parity remain open |

Run `rust/scripts/Test-Phase7AutomationExternal.ps1` from the repository root
with the development workspace running. It extracts the actual Deno/Shoutrrr
tools from `-CoreImage`, creates its own temporary PostgreSQL fixture, and removes
its tools/fixtures afterward. It does not build an image or touch development
data. The verified local candidate supplied Deno 2.5.2. This process gate does
not substitute for released-Agent or multi-node compatibility acceptance.

Verified for this follow-up:

- Workspace library tests: 312 passed.
- PostgreSQL adapter/HTTP suites: 42 passed on a fresh disposable database.
- Signed Agent, Edge session and process-runner tests: 20 passed.
- Real Deno/Shoutrrr/PostgreSQL acceptance: 1 passed, including terminal
  Activities, process cleanup and HTTP delivery retry.
- Adapter/server/xtask library and test Clippy, formatting, generated OpenAPI
  verification (297 full / 245 public operations), and database baseline
  verification passed. No schema changes were needed.

The final database gate used a fresh fixture. Reusing the earlier test database
exposed a queued Git repository left by an earlier HTTP fixture: a global queue
claim then selected that repository rather than the new test's repository.
Run the provided fixture scripts against disposable databases, not development
data. A separate fixed-name global binding collision in `resources_http` was
removed by giving that fixture a unique name.

No React feature files or shared Agent protocol files were changed. No Citadel
images were built. These results do not close the gaps below or constitute a
complete port of every Phase 7 .NET test.

## Automation streaming and Build lifecycle follow-up (2026-09-06)

- Manual/Test Automation HTTP requests now execute the claimed run and stream
  the existing JSON progress contract. The scheduler cannot steal that claim.
  All entry points share a four-run bound. Closed or stalled consumers cancel
  and reap Deno; terminal progress waits only within a bounded deadline.
- Action lists include Tags, latest-run summaries and capabilities. Summary
  queries do not load source snapshots or retained logs. Action, Build Project,
  Build Pool and Backup Policy tag endpoints reuse the resource-tag handler;
  action/build/pool filtering accepts the existing tag-name and ID contract.
- Action configuration and scheduled/executing runs enforce Automated Operations.
  Enabling paid triggers is denied without the capability; unrelated edits and
  disabling existing triggers remain possible after license loss.
- Build Project edit/rename/metadata/archive operations preserve partial-update
  semantics and row-version fencing. Typed configuration and run Activities
  commit with their state changes. Failed Activity persistence rolls back a run
  result; stale completion cannot emit duplicate Activities or failure alerts.
- Build queue and cancellation require Read plus Apply, matching .NET.
  External-pool and automated execution recheck their license capabilities
  before invoking the executor. Webhook configuration uses the shared validator.
- Pool health monitoring reads keyset batches of 64 with one bounded check at a
  time. It does not provision capacity or replay builds. Stale claims recover
  through version-checked writes; unchanged health is refreshed at most every
  five minutes, and notifications follow successful persistence.
- A reusable streaming redactor handles split secrets and UTF-8 boundaries for
  Build and Automation output.

| .NET reference | Added Rust coverage |
| --- | --- |
| Automation manual/Test execution and process cancellation | `automation_http_execution`: real Deno, PostgreSQL and Axum; success, exit failure, timeout, authorization, busy rejection, disabled Test, body disconnect, slow-reader cancellation and cleanup |
| Automation tag/filter and license configuration cases | `resources_http`: tag creation/filter/replacement/rollback, capabilities, denied schedule enablement without mutation; Automation policy unit tests cover disabling after expiry |
| Build endpoint edit/rename/metadata/archive | `phase7_resources_http`: saved configuration, tag filters, stale edits, active-run conflicts and typed Activities |
| Build queue/cancel permissions and execution licensing | `phase7_resources_http`: Read without Apply denied, Read+Apply queue/cancel allowed, unlicensed webhook queue/config denied, previously queued automated execution fails before executor output |
| Build run queue/start/terminal/recovery | `build_execution`: transactional Activities, audit-write rollback, late-completion fencing, interruption history and retained run behavior |
| Build Pool monitoring/recovery | `phase7_resources_http/build_pools.rs`: stable-result write suppression, refresh interval, stale-version rejection, active-claim exclusion and expired-claim recovery |

Use `Test-Phase7AExternalExecution.ps1 -WorkspaceContainer citadel_devcontainer-workspace-1`
to reuse the development compiler cache while still creating a separate
disposable PostgreSQL database. Omitting the option retains the isolated build
container workflow. Neither mode uses development database data.
`Test-Phase7AutomationExternal.ps1` also runs the HTTP execution and metadata
regressions with the real Deno/Shoutrrr tools.

These changes are not completion of Phase 7. The webhook follow-up below
extends the shared listener beyond repository pulls; Stack/Service dispatch
and the full external compatibility matrix remain open.

Verified for this follow-up:

- Workspace library tests: 316 passed.
- Phase 7A PostgreSQL/HTTP/HTTP2 gate: 43 tests passed, including the
  configuration, persistence, recovery, authorization and Edge suites.
- Process runner, signed Agent and Edge session suites: 20 passed.
- Real Deno/Shoutrrr and Automation HTTP execution: both passed; the accompanying
  metadata and Build/Backup/Alert HTTP suites also passed.
- Real Local Docker/Restic/PostgreSQL backup and restore: 1 passed.
- Adapter/server/xtask library and test Clippy, formatting and OpenAPI
  verification passed (308 full / 256 public operations).

No React feature code, database schema or shared Agent protocol was changed.
No Citadel image was built. Test scripts removed their disposable databases
and temporary tool containers; development data was not modified.

## Webhook execution and node coverage follow-up (2026-09-06)

- Shared authentication now serves repository, Automation, Build and Backup
  Policy dispatch. Provider routing cannot choose a weaker authentication
  scheme than the saved configuration. Duplicate authentication headers,
  oversized bodies, invalid signed timestamps and tampered signatures fail
  before queuing. Branch and repository filters do not mutate resource state.
- Automation, Build and Backup webhooks queue durable runs. Current webhook
  configuration (or Build row version) is checked inside the queue transaction,
  so rotating/disabling a webhook while a request is in flight fences the old
  request. Duplicate delivery cannot steal the current run. Automated Backup
  execution also rechecks the current license before source planning.
- Build execution waits for the normal Git worker to finish a fresh sync and
  pins its resulting commit across Local, Agent and Edge execution. A failed
  sync cannot silently use old cached source. A webhook-specified commit stays
  pinned. Build webhook path filtering can compare the previous successful
  commit when the provider supplies no paths.
- Automation execution and scheduling share configured enablement, concurrency
  and timeout limits. Scheduler scans use keyset batches; one invalid/busy
  Action does not block subsequent Actions. Deno supports the existing work,
  cache, log-size and network settings. Filesystem permissions are restricted
  to each run directory and environment access to `NO_COLOR`/`DENO_DIR`.
- `getSwarmNodeAgentCoverage` now exposes the existing public response contract,
  Read authorization and management capability. Its bounded database snapshot
  joins bindings, node-runtime freshness and latest system-Service tasks.
  Coverage distinguishes manager-only clusters, down-but-active workers,
  architecture aliases, protocol incompatibility and system-Service drift.
  Empty inventory initializes through the selected Local/Agent/Edge manager
  connector with a 30-second bound. Persistence rechecks cluster/manager identity
  and only writes if inventory is still empty, preserving concurrent background
  reconciliation. Transport failure propagates without saving an empty snapshot.
  Installation, repair and upgrade mutations are not implemented. Removal is
  described in the follow-up below.

| .NET reference | Rust coverage added |
| --- | --- |
| `ReceiveWebhookTests` / `WebhookListenerTests` authentication and filtering | shared `resources::webhooks` unit tests, `resources_http`, `automation_http_execution/webhooks` |
| Build webhook context relevance, active/lost claim, committed queue | `phase7_resources_http/build_webhooks` through Axum and PostgreSQL; shared matcher unit tests |
| Backup webhook run-as policy and disabled/busy/license paths | `phase7_resources_http/backup_webhooks`, including entitlement loss after queuing |
| Build Git synchronization/pinning | real Git and PostgreSQL `git_repository_execution`; executor revision validation |
| `CronScheduleTests`, Automation sandbox/options | Automation unit tests and real Deno `automation_external_acceptance` |
| `SwarmNodeAgentLifecycleTests` coverage scenarios | platform policy unit tests and `platforms_http/node_coverage` |
| `GetSwarmNodeAgentCoverageTests` initialization/error propagation | Axum + Unix Docker fixture + PostgreSQL initialization, repeat read, identity mismatch and concurrent-initialization fence |
| `SwarmBackupCompatibilityTests.WorkerVolume_ShouldBackupToRustFsAndRestoreOnAnotherNode` S3 storage portion | real Docker/Restic/RustFS/PostgreSQL backup/restore and overwrite rejection; Local connector only, not multi-node routing |

These mappings describe the implemented scenarios, not a complete port of each
referenced .NET class. Released-Agent/multi-node acceptance and the remaining
operations below must pass before Phase 7 can be marked complete.

Verification of this follow-up:

- 328 workspace library tests passed.
- The Phase 7A gate passed, including 45 PostgreSQL/HTTP/HTTP2 tests,
  process/Agent/session tests, Clippy and OpenAPI verification (309 full /
  257 public contracts).
- Real Deno/Shoutrrr execution plus Automation/metadata/Build/Backup HTTP
  execution passed, including the sandbox restrictions.
- Server binary and Platform HTTP test Clippy passed.
- Real RustFS S3 volume backup/restore passed via
  `Test-Phase7LocalBackup.ps1 -UseRustFs`. It checks restored bytes, persisted
  snapshots/run results, and overwrite rejection. The fixture uses dedicated
  PostgreSQL/RustFS containers and UUID-scoped volumes, removed afterwards.
  Backup acceptance and Automation HTTP test Clippy passed.

## Node-agent removal follow-up

The Rust endpoint `DELETE /api/v1/platforms/{id}/node-agents` now streams the
existing progress contract without frontend changes. It requires Platform Execute
and ManageNodeAgents. The manager's daemon, node and cluster identities are checked
before the durable claim and before runtime deletion. Local, signed Agent and Edge
dispatch use the canonical shared contracts; an unavailable manager never causes
fallback to a different Docker daemon.

Removal checks every ownership label before deleting the Service, revokes satellite
and bootstrap credentials transactionally, disconnects the selected node sessions,
marks their projections stale, and attempts Secret/CA Config cleanup. Unowned
resources are preserved. Cleanup failures produce warnings; state volumes and the
manager binding are retained. Failed primary operations retain installation IDs
for retry. Running operations have exclusive claims and stale completions cannot
overwrite a successor. Claim/completion Activities commit with the state changes;
coverage changes use the existing realtime event. Expired operation claims are
reported as failed/retryable by coverage, so a Core crash cannot leave the existing
UI actions disabled indefinitely.

Additional .NET scenario mapping:

| .NET reference | Rust coverage |
| --- | --- |
| `LifecycleEndpoints_ShouldRequirePlatformScopedManageNodeAgentsPermission` (DELETE case) | HTTP 401/403 and Execute-without-ManageNodeAgents rejection |
| `Remove_ShouldRevokeNodeBindingsAndPersistRemovedDesiredState` | HTTP/PostgreSQL persisted removal, satellite/bootstrap revocation, manager preservation and typed lifecycle Activities |
| Lifecycle concurrency and manager-identity guards | competing claims, expired-claim replacement, stale completion/revocation rejection, mismatched daemon and transaction rollback |
| Node-agent resource ownership and exact transport | ownership label unit test, generated Unix Docker inspect/delete tests and Edge command-peer Service/Secret/Config deletion; foreign Service and missing manager rejected |
| Cancellation/partial failure safety | application unit tests for canceled removal, failed primary deletion, cleanup warnings, persistence failure and full progress channel |

This is removal, not installation/repair/upgrade completion. Command-peer tests
are not released-Agent or multi-node acceptance.

Verification: 336 workspace library tests passed. The Phase 7A gate includes
47 PostgreSQL/HTTP/HTTP2 tests, process/Agent/session tests and the Local Docker
transport tests. OpenAPI verifies 310 full / 258 public contracts; generated
Docker API 1.49 verification and targeted Clippy pass. This gate now runs the
Platform lifecycle unit tests and generated Docker transport tests automatically.

## Node-agent installation, repair and upgrade follow-up

The existing POST `node-agents/install`, `node-agents/repair` and
`node-agents/upgrade` endpoints now use one application workflow and the same
progress, permission and durable-operation contracts as removal. Local Docker,
signed Agent RPC and Edge commands use the existing shared transport surface.
There are no frontend changes or new database migrations.

The workflow verifies pinned manager identity and ownership, resolves a digest
supporting eligible Linux node architectures, stores only the hash of a random
10-minute bootstrap credential, and mounts that credential as a Docker Secret.
The global system Service excludes the connected manager, retains its node-local
state volume, and has bounded resources, log rotation and valid Secret/Config file
permissions. Optional Core CA configuration is loaded once at startup with a
1 MiB bound. Service identity is persisted before waiting for enrollment.

Successful Docker submission is not successful setup: completion requires a
completed rollout, current running Tasks on the pinned image, matching non-revoked
task bindings and live satellite sessions. Setup is bounded to five minutes;
cancellation, failure and timeout finalize the operation and revoke bootstrap
access. Repair recovers an owned Service by stable name after interrupted creation.
Old owned Secret/CA material is cleaned up only after coverage succeeds; cleanup
failures are warnings. Manager-only installation creates no bootstrap or Service.

Additional .NET scenario mapping:

| .NET reference | Rust coverage |
| --- | --- |
| `GetLinuxArchitectures_ShouldIncludeSingleManifestDescriptorPlatform` and `...ShouldUnionManifestListAndDescriptorPlatforms` | OCI descriptor/index mapping tests, architecture aliases and digest/architecture validation |
| `Install_AfterRemoval_ShouldRestoreInstalledDesiredState` | HTTP/PostgreSQL installation from Removed plus Repair/Upgrade state and typed Activities |
| `LifecycleEndpoints_ShouldRequirePlatformScopedManageNodeAgentsPermission` (three POST cases) | unauthenticated and Execute-only rejection, scoped ManageNodeAgents success |
| Bootstrap lifecycle and partial failure safeguards | PostgreSQL hash/expiry/version rotation/revocation, competing operation and stale-writer fencing; cancellation, paused rollout and stale-Task timeout unit tests |
| Local/Agent/Edge system-Service contract | generated Docker distribution/Secret/Config routes, hardened Local task spec, canonical Edge create/update commands and exact-manager/no-fallback assertions |

These are unit, HTTP/database and command-peer tests. The .NET
`ManagerConnector_ShouldInstallRouteWorkerContainersAndRecoverPartialCoverage`
released-Agent three-node acceptance scenario is still required; it is not
replaced by these tests.

Verification: 345 workspace library tests passed. The Phase 7A gate passed,
including 50 PostgreSQL/HTTP/HTTP2 tests, process/Agent/session tests and four
Local Docker transport tests. Server binary and new HTTP/transport test Clippy
passed. Generated Docker API 1.49 and OpenAPI checks passed (313 full / 261 public
contracts). The disposable PostgreSQL fixture was removed; no user workloads,
database volumes or node-agent installations were modified during verification.

## Container, Deployment and Stack live-log follow-up

The existing `container-log` and `stack-log` groups and `StartContainerLogs`,
`StartDeploymentLogs` and `StartStackLogs` invocations now stream through Local
Docker, signed Agent RPC and exact-node Edge commands. The frontend is unchanged.
Docker's TTY mode is inspected before decoding its raw or multiplexed response;
headers are not rendered as log text. Split UTF-8 lines, timestamps, Stack
container-filter prefixes and the final unterminated line are preserved.

Access requires Read plus Logs on the Platform or owning workload, not Read
alone. Subscriptions recheck access, cancel upstream work when left/disconnected,
reject stale node data and never route an unavailable worker to the manager.
Repeated starts are idempotent. Docker IDs resolve only when unambiguous; the
Deployment viewer's short-Docker-ID group remains compatible. Stack streams
follow committed inventory changes and replace only changed container streams.

The implementation limits each connection to four log groups, each Stack to
32 container streams, and the process to 128 upstream log streams. Frames and
partial lines are bounded to 1 MiB. Startup and WebSocket sends are timed out;
there is no unbounded log history or detached log-pump task in Core.

Additional .NET scenario mapping:

| .NET reference | Rust coverage |
| --- | --- |
| `StartContainerLogs_FullDockerId_UsesNormalizedSubscriberGroup` | PostgreSQL reference resolution and normalized invocation-group assertions |
| `StartContainerLogs_ResourceId_RoutesToPersistedOwningNode` | Actual WebSocket + PostgreSQL + Edge command-peer test, asserting full Docker ID, selected node and no commands to another node |
| Log subscription permission and lifecycle requirements | Read-without-Logs rejection, join-before-start, duplicate start, leave cancellation and permission-revocation disconnect |
| Existing Stack viewer framing and runtime membership | UTF-8/timestamp/prefix unit tests and committed container-replacement test without reopening unchanged streams |
| Local/Agent transport parity | Unix-socket TTY/non-TTY route test, multiplex-frame bound/cancellation tests, signed HTTP/2 stream and cancellation test |

This completes this live-log slice, not interactive terminal support, Swarm
Service/Task log endpoints, volume browsing/mutations or all remaining Phase 7
work. Released-Agent multi-node acceptance remains a separate required gate.

Verification: all 351 Rust workspace library tests pass. The Phase 7A gate
passes, including both new PostgreSQL/live-WebSocket log tests and the new
Unix-socket log transport test. Targeted test Clippy, generated Docker/OpenAPI
checks and the two existing frontend log-viewer tests pass. The disposable
PostgreSQL fixtures were removed; user databases and Docker workloads were not
modified.

### Terminal transport and authentication foundation

Code review found a pre-existing regular-Agent security gap: Core's
`HubSigningInterceptor` and Agent's `AgentVerificationInterceptor` previously only overrode
unary and server-streaming calls. `ContainerGrpcService.Exec` is bidirectional
and directly opens the Docker exec session; the inspected service registration
adds no authorization policy and direct TLS configures a server certificate,
not required client authentication. An Agent reachable by an untrusted client
therefore appeared to allow unsigned terminal requests. This was strongly inferred
from code, not a live exploit against an installed Agent.

The following implementation now closes that prerequisite in source:

- .NET Core signs the exact initial `ExecClientMessage.Open` using its existing
  signature algorithm and metadata. A failed initial write disposes the call.
- Regular Agent verifies that first message before entering the Docker handler,
  including nonce replay protection. Empty streams and signed non-Open messages
  are rejected. Opening is limited to ten seconds; this deadline does not apply
  to the authenticated interactive session.
- Rust signs the same opening message and uses the existing duplex protobuf RPC.
  Edge sends `ContainerExec` and `StreamInput` over the selected authenticated
  session. Reconnect never forwards old input to a replacement Agent session.
- Local Docker uses generated Exec routes and an upgraded Unix-socket connection,
  not a buffered process output or a polling loop.
- Input queues hold at most eight messages, each at most 16 KiB; dimensions are
  1–1000. Agent output frames are limited to 1 MiB. Local reads use 16 KiB buffers.
  Input backpressure is explicit, and dropping output cancels/releases transport
  resources. Raw Agent execution errors are not exposed to terminal viewers.

Core and regular Agent must be upgraded together for signed terminals. Old Core
terminal calls are deliberately rejected by the updated Agent; there is no
unsigned compatibility fallback. No protobuf or frontend change is necessary.

Coverage for this foundation:

| Reference / requirement | Tests |
| --- | --- |
| Agent signature and replay policy | .NET Agent: valid Open, tampering, missing headers, wrong key/method, replay, signed Resize before Open, empty stream, cancellation and silent-client opening timeout |
| `EdgeAgentConnectorTests` interactive command contract | Rust Edge: exact node/command, Open/stdin/resize/output, cancellation on disposal, reconnect isolation |
| `ExecSessionManagerTests` transport disposal requirement | Rust Local/Agent/Edge: socket/stream closure and closed input after disposal/cancellation |
| Local/Agent protocol equivalence | Unix-socket HTTP upgrade/resize test and real HTTP/2 fixture verifying the Ed25519 signature over the initial Open |
| Memory bounds | Input queue and frame bounds, invalid dimensions, closed-input rejection and safe error framing |

This is **not yet browser terminal parity**: WebSocket start/input/resize routing,
join-before-start, per-connection session ownership and resource permission
revocation tests remain to be connected and ported. Existing Rust exec groups
remain unavailable until that authorization layer is implemented. Released-Agent
multi-node acceptance has not been run for terminals.

Foundation verification: 354 Rust workspace library tests pass, as do the
Phase 7A gate (including PostgreSQL and WebSocket integration tests), the Local/
Agent/Edge transport tests, targeted Clippy and generated Docker/OpenAPI checks.
All 23 .NET Agent security tests pass, including ten new duplex-authentication
cases. The .NET Core Infrastructure project builds with existing warnings and
no errors. No frontend changes or live workload/database mutations were made.

### Browser terminal and bounded Swarm logs

The existing Container, Deployment, Stack and Swarm Task terminal invocations
now use the authenticated Rust WebSocket connection. Joining is required before
starting; input belongs to the connection that opened the session, not merely
to anyone who knows its group name. Read plus Terminal permissions are checked,
then revalidated during the session. Target replacement, stale node data,
revocation, leave and disconnect terminate the old command. Input does not
perform a live Docker Task inspection for each keystroke. The legacy output
event has no session discriminator, so each WebSocket may own one terminal.

Swarm Service, Task and managed Service log HTTP endpoints now preserve the
existing operation IDs and response shape. Tail is limited to 1–200, retained
log bytes to 1 MiB, and the request has a deadline. Task logs inspect the live
Task identity and route to its exact node's container. Local Docker, signed
Agent and Edge share frame/UTF-8 handling; no frontend workaround is required.

Additional .NET test mapping:

| Reference | Rust evidence |
| --- | --- |
| `ExecSessionManagerTests` session ownership/lifecycle | PostgreSQL + real WebSocket tests for join-before-start, another connection's input, duplicate start, permission revocation and container replacement |
| `SwarmEndpointTests.LogsEndpoint_ShouldRejectOutOfRangeTail` | Both Service and Task log routes reject invalid tail values |
| `LogsEndpoint_ShouldRequirePlatformLogsPermission` | Read-only access is denied before a runtime command |
| `LogsEndpoints_ShouldReturnBoundedConnectorResultsForProjectedResources` | Local Service and exact-node Edge Task HTTP results, UTF-8 split across frames |
| `LogsEndpoint_ShouldRejectAnIdOutsideTheSelectedPlatformBeforeCallingConnector` | Foreign Service ID returns 404 |
| `EdgeAgentConnectorTests.SwarmLogs_ShouldRouteBoundedServiceAndTaskCommands` | Canonical Service log protobuf request, tail and truncation preservation, oversized response rejection |

The Phase 7A gate passed with 33 platform HTTP tests, including these additions,
and 316 full/264 public OpenAPI contracts. `Test-Phase7AgentCandidate.ps1` also
passed against the locally built, published .NET Agent image (not a command
peer): wrong signing keys are rejected and logs, terminal input, resize and
cancellation work. This is direct-Agent acceptance, not multi-node acceptance.

The real Local Docker/Restic/RustFS round trip passed again. Inspection while
extending this to the actual Agent found two remaining remote-execution gaps:
repository operations still required Local, and retention unconditionally ran
Core's Docker CLI. S3 repository operations and retention now use the selected
Agent/Edge runtime; repository-only helpers mount no source volume. The added
Agent backup acceptance case deliberately configures an unusable Core Docker
executable, so hidden local execution cannot satisfy it.

Useful targeted commands:

```powershell
.\rust\scripts\Test-Phase7AgentCandidate.ps1 -AgentImage citadel-agent-phase7:local
.\rust\scripts\Test-Phase7LocalBackup.ps1 -UseRustFs -AgentImage citadel-agent-phase7:local
```

## Still required

### Current completion pass

The real published-Agent and Edge-Agent S3 backup/restore/retention cases now
pass with Core's Docker executable deliberately unavailable. The Edge case
exposed a protocol mismatch masked by command-peer tests: the shared .NET
contract is version 2, not version 1. Enrollment now validates the shared
constant, persists the negotiated version and rejects incompatible enrollment
without consuming its credential.

Git Stack webhook dispatch now uses the existing durable Stack queue. Reception
authenticates before dispatch, validates repository/branch/path filters and
honors the automated-operations entitlement. Queue claims and Stack Apply claims
are atomic, configuration changes invalidate queued work, duplicate pushes
coalesce, retries are bounded to three attempts, and unknown Docker outcomes
wait for reconciliation. Successful completion and known failure settle the
queue in the same transaction as the Stack result/activity. Notify-only still
depends on the remaining Git update producer; it is not an end-to-end completed
notification feature yet.

The added PostgreSQL tests also exposed two pre-existing Stack persistence
bugs: Git repository validation queried a nonexistent archive column, and
successful Apply mixed `jsonb` parameters with `json` columns in `COALESCE`.
Both have been corrected without changing the baseline schema.

Image inspection now uses the existing HTTP response and capability shape,
with Local, signed Agent and exact-node Edge implementations. Generated Docker
contracts cover details/history and container usage. Registry ports are
preserved in display references, null Docker collections are normalized, and
stale or unknown nodes cannot fall back to another daemon. Docker transport
tests, HTTP/persistence tests and the actual-Agent case passed. The subsequent
exposed-ports endpoint preserves the UI's image UUID and resolves it to the
Docker ID on the correct node. The gate passed with 35 platform HTTP tests and
318 full / 266 public contracts before adding Service update checks.

The opt-in three-node backup fixture uses isolated Docker-in-Docker daemons and
the actual Agent candidate as a global Swarm service. It targets the .NET
`WorkerVolume_ShouldBackupToRustFsAndRestoreOnAnotherNode` scenario, including
same-named volumes on different nodes, persisted source/target identity,
cleanup and failure when source-node coverage disappears. It passed against
the published Agent candidate `sha256:5239b06a2e5cb38002bba80299c4e7812cd8b96e6270e2196c45485bfac9da23`.
This is three real Swarm nodes, real worker Edge Agents and RustFS, with restored
bytes verified on the other worker. The fixture seeds node-agent installation
metadata; it does not prove the installation/repair API lifecycle.

That real-node run exposed restrictions which ordinary Edge command-peer tests
missed: repository-only backup helpers must allow no source mount, and their
Restic execution allowlist must include init/check/snapshots/forget. These are
now supported without permitting arbitrary shell, mount or dump commands.
All 41 Agent command-dispatch tests passed, and the actual candidate was rebuilt.

Service image update checks now have a separate persisted lease rather than
overwriting the last Docker operation. Cancellation and restart release only
that lease; stale deletion recovery explicitly excludes it. The schema source
was updated and `0001_initial.sql` regenerated using xtask (no manual migration).
The Service check route brings the generated catalog to 319 full / 267 public
contracts. The HTTP/concurrency tests and the complete Phase 7A gate passed.

Service webhooks now authenticate through the existing listener and use the
same update-check policy as scheduled checks. Notify-only does not Apply;
automatic Apply rechecks both license capabilities and claims the checked
configuration version atomically. Secret rotation invalidates old requests.
The two-hour scheduled scan uses bounded keyset pages. HTTP/PostgreSQL tests
cover authentication, licenses, digest equality/change, stale configuration,
automatic Apply, pagination, and disabled Services.

Git, Stack, Build and Service webhook reception records shared, typed activity
metadata without request bodies, authorization headers or credentials. An audit
write failure does not turn an already accepted dispatch into a retryable HTTP
failure. Provider-specific Git branch/commit audit fields still need parity work.

The three-node fixture also passed worker-local logs and interactive terminal
input/resize/cancellation, with a negative check against the other worker.

Container action endpoints now retain the existing UUID/short Docker ID contract
and claim selected containers plus managed parents atomically. A separate
container-operation lease fences late results from inventory updates and newer
commands. Parent recovery excludes these claims, so a timed-out container action
cannot be mistaken for Stack deletion. Unknown outcomes are inspected, never
replayed. Batches and active operations are bounded; HTTP disconnect does not
abandon committed work. The initial schema was regenerated through xtask.
The catalog now contains 325 full / 273 public routes. Local Docker route/304
tests, HTTP/persistence/parent-fencing tests and cancellation/capacity unit tests
passed. The real three-node Agent/Edge fixture also passed the container actions,
logs and terminal checks. The latest complete gate is being rerun after the
additional volume routes.

Volume directory listing and download now preserve the existing frontend API
and permission bits across Local Docker, signed Agent and exact-node Edge.
Helpers use a read-only volume mount, no network, bounded memory/PIDs and a
bounded execution lifetime. Metadata and Docker multiplex frames are bounded;
downloads remain streaming. Cancellation and unconsumed response bodies retain
cleanup ownership, and only successfully completed downloads record a success
activity. The generated catalog checks pass at 327 full / 275 public routes.

The HTTP/PostgreSQL tests cover permission denial before helper creation, node
identity, missing coverage, invalid paths, symlinks, binary contents, interrupted
streams and cleanup without deleting the source volume. The actual three-node
fixture passed again (191.95 seconds), including Local, manager Agent and worker
Edge volume reads, same-named volumes with different contents, streamed directory
archives and worker-to-worker RustFS restore. This run used Agent candidate
`sha256:497239d27239bba37c7d9a3db9996cfe432eb4fd80dc9c9a50eb8a06a3c50bbc`.
The shared helper now exits after 31 minutes rather than idling forever.

`Test-Phase7AgentBuild.ps1` passes against an isolated Docker daemon and Registry
through signed Direct Agent (25.40 seconds) and Edge Agent (24.12 seconds). It
verifies committed-source archiving, build/push, Registry digest, container
contents and persisted Build result/logs. Candidate:
`sha256:a2740eb3c3c0778edf203f8018799f79a07cdb034492861ae9cbe706e9aad086`.
The Docker 400 was caused by missing anonymous `X-Registry-Auth`; the shared push
path now sends encoded `{}` without credentials. Both Agent/Core shared-contract
copies were updated. Rejected hijacks dispose connections and retain bounded
diagnostics. Selected .NET helper, Build image and transport tests pass (18).

The complete Phase 7A gate passed before the latest Deployment producer work.
Frontend unit tests passed: 105 files / 368 tests, without frontend source edits.
Deployment HTTP/PostgreSQL checks pass for digest comparison, authorization,
cancellation, concurrent claims, stale lease recovery, configuration fencing and
Notify/license behavior. Automatic Apply outcome coverage, durable Build consumer
propagation and provider audit metadata are being verified; those gates remain open.

Volume security follow-up remains: the inherited helper checks symlinks before
opening paths, which does not yet prove safety against concurrent replacement.
Also, an ambiguous helper creation followed by a disconnected Edge session can
leave a never-started helper; its process lifetime limit cannot clean that case.

```powershell
.\rust\scripts\Test-Phase7LocalBackup.ps1 -UseRustFs -AgentImage citadel-agent-phase7:local -UseEdgeAgent
.\rust\scripts\Test-Phase7LocalBackup.ps1 -UseRustFs -AgentImage citadel-agent-phase7:local -MultiNode
```

### Completion pass (2026-09-07)

This section supersedes the historical open-item lists above. The database and
transport gate, actual node-agent lifecycle/multi-node backup matrix, and real
external-service acceptance below passed. Ignored acceptance tests were explicitly
executed with disposable infrastructure; their presence alone is not evidence.

- Deployment and Stack image/Git update producers now emit update-available,
  successful automatic Apply and failed automatic Apply Alerts. Notification
  does not mutate Docker. Automatic operations recheck entitlements and the
  checked configuration version. Service-only Stack updates select only the
  changed services, without bringing down unrelated services. A partial Apply
  cannot advance the whole Git Stack baseline.
- Build completion uses a durable, per-consumer/per-project queue. A second
  Build project on the same Stack cannot overwrite the first project's receipt.
  Resolved provenance advances independently of applied provenance; only
  opted-in services are redeployed. Interrupted claims are fenced rather than
  replayed blindly. Queued builds retain their builder target, build arguments,
  secret references and tag templates when the project is edited later.
- Git repository, Build, Stack and managed Service webhooks use the shared
  authentication and audit path. Audit fields describe the dispatched branch
  and known commit; request bodies and credentials are never audit payloads.
- Node-local browsing/mutations and logs/terminal use existing HTTP/realtime
  contracts and permissions. No frontend feature edits were needed.
- Linux volume reads now walk and hold directory descriptors without following
  symlinks. FIFO/device paths fail without blocking. A bounded cleanup worker
  removes expired, ownership-verified volume helpers on their exact node,
  including helpers whose ambiguous creation left them never started.
- The three-node fixture now invokes the real Install, Repair and Upgrade
  workflow, instead of seeding installation records. It uses an actual signed
  manager Agent, global worker Edge Agents and an isolated Registry. This
  exposed missing optional OCI platform normalization in the Agent and the
  initial `None` rollout-state mismatch. Repair additionally exposed a stale
  binding Task ID; authenticated reconnect now revalidates and records a new
  Task on the same Node without requiring node-ID rebind grace.
- Real Forgejo/Vault acceptance exposed a stale-cache Apply: Git-backed Apply
  now synchronizes unpinned branches through the existing repository worker and
  materializes the resulting immutable commit. Pinned releases do not move.
- Vault provider connection/reference test endpoints retain the .NET routes,
  permissions and response shapes. Empty token edits preserve stored credentials.
  Successful runtime output is redacted before streaming or failure persistence;
  webhook credentials are masked in all persisted configuration activity snapshots.

The schema source was extended and `0001_initial.sql` regenerated through
xtask: 84 tables. There is no handwritten `0002` migration. Development databases
created from an older unreleased baseline must be recreated. Generated contract
checks cover 331 full / 279 public operations.

#### Additional .NET scenario mapping

These are scenario ports, not a claim that every .NET test file has been
translated line-for-line. The earlier tables cover permissions, API lifecycles,
realtime, cancellation, webhook validation and inventory ordering.

| .NET reference | Rust test / additional regression |
| --- | --- |
| `DeploymentAutoUpdateJobTests` | `server/tests/deployments_http/updates.rs`: Notify, entitlement fallback, automatic success/failure Alerts, leases and stale-version rejection |
| `ManualStackAutoUpdateTests`, `GitStackWatchPathMatcherTests` | `stacks/src/updates.rs`, `server/tests/stacks_http/updates.rs`, `adapters/tests/git_repository_execution/stack_updates.rs`: image baselines, selected-service Apply, real Git changed paths, pinned commits |
| `BuildRunStartTests.ExecuteQueuedBuildRun_ShouldUseQueuedRunTargetSnapshot_WhenProjectBuilderChangesAfterQueue` | `server/tests/phase7_resources_http/build_completion.rs`: queue-time target/arguments/secret-reference/tag snapshot survives project edits |
| `BuildRunCoordinatorTests`, `BuildRunCleanupServiceTests`, Build-image resolver tests | `builds/src/jobs/completion_tests.rs`, `server/tests/phase7_resources_http/build_completion.rs`: two projects/one Stack, coalescing, retained artifacts, Apply failure and interrupted-claim fencing |
| `StackApplyRecoveryTests` | `adapters/tests/stack_persistence.rs`, `server/tests/stacks_http/updates.rs`: persisted partial selection, stale completion rejection and inspection-based recovery |
| `ReceiveWebhookTests`, `WebhookListenerTests`, `StackWebhookDeployQueueRepositoryTests` | `server/tests/phase7_resources_http/{git_webhooks,build_webhooks,stack_webhooks}.rs`, `server/tests/swarm_services_http/webhooks.rs`, `adapters/tests/stack_webhooks.rs`: authentication, dispatch/audit metadata, licenses, coalescing and recovery |
| `SwarmNodeAgentLifecycleServiceTests`, `SwarmNodeAgentLifecycleTests`, `SwarmNodeAgentCompatibilityTests` | `platforms/src/node_agents/setup_tests.rs`, `adapters/tests/edge_transport.rs`, `adapters/tests/backup_restic_acceptance/node_setup.rs`: initial rollout state, signed same-node Task replacement, install/repair/upgrade and bootstrap rotation |
| `VolumeHelperCommandTests` | shared helper's real non-root Linux tests (9 passed), plus Rust volume HTTP/transport tests: traversal, concurrent symlink replacement, FIFO and descriptor cleanup |
| `SwarmBackupCompatibilityTests.WorkerVolume_ShouldBackupToRustFsAndRestoreOnAnotherNode` | `adapters/tests/backup_restic_acceptance/multinode.rs`: same-named volumes, exact-worker bytes, missing coverage, logs/terminal and expired helper cleanup |
| `ControlPlaneRecoveryTests` database-state recovery | `adapters/tests/citadel_system_recovery.rs`: real private bundle, `pg_dump`/`pg_restore` into a separate clean database, restored state; full release-image restart remains Phase 8 |
| `BuildAgentPoolEndpoints_ShouldPersistLifecycleAndEdgeEnrollment` tag follow-up | `server/tests/phase7_resources_http.rs`: create/replace/read persisted Pool tags and positive/negative tag-name filters; passed with the complete resource lifecycle and lint |
| `ForgejoPush_ShouldUpdateAndDeployGitStack`, `VaultKvV2_ShouldValidateResolveInjectAndRedactSecret` | `server/tests/external_integrations_acceptance.rs`, `external_integrations/vault.rs`: real private Forgejo repository and signed delivery, current-commit Docker Apply, Vault valid/invalid/stored tokens, missing path/key/version, encrypted persistence, runtime injection and audit/progress redaction |

#### Verification

- Complete Phase 7A gate **passed** with the node reconnect, Git synchronization,
  provider validation and audit/progress redaction fixes, including all 39
  Platform HTTP tests. Generated contracts verified: 331 full / 279 public.
- Additional Resources/Contracts unit tests: **34 passed**. Clippy passed for
  Resources, Server and Adapters across all targets; formatting and diff checks
  passed.
- Frontend unit tests: **105 files / 368 tests passed**, no frontend feature edits.
- Actual current Agent candidate:
  `sha256:790469de9028060d0acc3868946dc6a8a896553599e5275d2c81c1dff00c5c53`.
  Signed Direct-Agent logs/terminal passed; real Direct and Edge build/push
  passed against an isolated Docker daemon and Registry.
- Real Git Stack changed-path/pinned-commit scanner passed.
- Real private Forgejo push → Stack Apply with Vault KV v2 resolution:
  **passed (25.13 seconds)**. This includes the provider/reference HTTP tests,
  stored-token preservation, runtime secret injection, current-commit container
  contents, and credential-free progress/audit records. The earlier failures
  exposed and fixed stale Git materialization and two redaction gaps.
- Real Deno execution and Shoutrrr HTTP retry delivery, with the accompanying
  Automation/Build/Backup HTTP and PostgreSQL suites, passed.
- Real PostgreSQL control-plane backup/restore passed through
  `Test-Phase7SystemRecovery.ps1`, as an unprivileged runner with separate source
  and target databases. Citadel-system bundles are Core-local by design; worker
  data volumes are backed up through their exact node, not by moving Core's
  database credentials to a satellite.
- Real three-node Install/Repair/Upgrade plus exact-worker browsing and
  worker-to-worker RustFS backup/restore: **passed (296.36 seconds)**. This also
  verifies bootstrap revocation, immutable helper image selection, streaming
  logs/terminal and expired-helper cleanup on the intended Node.

AWS Build Pool provisioning is excluded because the .NET reference also rejects
it; it is not an implemented feature being silently dropped. Full release-image
installation/restart, exhaustive browser acceptance, packaging/security review
and multi-day memory soak remain Phase 8, not evidence supplied by this gate.

Repeatable commands (all test infrastructure is disposable):

```powershell
.\rust\scripts\Test-Phase7AExternalExecution.ps1 -WorkspaceContainer citadel_devcontainer-workspace-1
.\rust\scripts\Test-Phase7AutomationExternal.ps1
.\rust\scripts\Test-Phase7AgentCandidate.ps1 -AgentImage citadel-agent-phase7:local
.\rust\scripts\Test-Phase7AgentBuild.ps1 -AgentImage citadel-agent-phase7:local
.\rust\scripts\Test-Phase7AgentBuild.ps1 -AgentImage citadel-agent-phase7:local -UseEdgeAgent
.\rust\scripts\Test-Phase7LocalBackup.ps1 -UseRustFs -AgentImage citadel-agent-phase7:local -MultiNode
.\rust\scripts\Test-Phase7SystemRecovery.ps1
.\rust\scripts\Test-Phase7Integrations.ps1
```
