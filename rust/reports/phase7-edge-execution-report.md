# Phase 7: Edge execution follow-up

Status: implemented portions below; **Phase 7 remains open**.

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

## Still required

- Remaining Build execution/source/trigger parity and released-Agent
  build/push acceptance. AWS
  provisioning is not implemented in the .NET execution reference either.
- Remaining node-local detail/mutation/browsing routes and released-Agent
  installation/repair/upgrade acceptance.
- Interactive logs/terminal wiring and its permission/transport acceptance tests.
- Remaining Git/image update producers.
- Stack/Service webhook dispatch, webhook audit-event parity,
  and the complete .NET test mapping.
- Real Local/Agent/Edge external-service acceptance, multi-node backup/restore,
  Citadel-system exact-node execution, and interrupted-operation recovery.

The pre-existing local `citadel-rust-phase7:local` image lacks the Restic binary;
it predates the current Dockerfile's Restic installation. It cannot serve as a
passing candidate for the external-service acceptance matrix without rebuilding.
