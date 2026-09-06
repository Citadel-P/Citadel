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

## Still required

- Remaining Build Project/Pool lifecycle and tag-filter parity, automatic Pool
  health/recovery monitoring, and released-Agent build/push acceptance. AWS
  provisioning is not implemented in the .NET execution reference either.
- Node-agent installation/bootstrap APIs and the
  remaining node-local detail/mutation/browsing routes.
- Interactive logs/terminal wiring and its permission/transport acceptance tests.
- Remaining Git/image update producers.
- Remaining Automation HTTP progress, tag/filter and configuration/permission
  parity and their complete .NET integration-test mapping.
- Real Local/Agent/Edge external-service acceptance, multi-node backup/restore,
  Citadel-system exact-node execution, and interrupted-operation recovery.

The pre-existing local `citadel-rust-phase7:local` image lacks the Restic binary;
it predates the current Dockerfile's Restic installation. It cannot serve as a
passing candidate for the external-service acceptance matrix without rebuilding.
