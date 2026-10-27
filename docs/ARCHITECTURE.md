# Citadel architecture

This guide defines where code belongs and the boundaries to preserve when changing
Citadel. For setup and commands, see [Development](DEVELOPMENT.md). For the wire
contract between Core and Agent, see [Agent protocol](agent-protocol.md).

## System overview

Citadel Core serves the HTTP API, frontend and realtime connections. It stores
configuration, authorization, activities, operation claims and runtime projections
in PostgreSQL. Background workers collect observations and execute queued work.

Core reaches Docker through one of three paths:

| Path | Connection | Execution owner |
| --- | --- | --- |
| Local | Core connects to Docker | Core's Docker adapter |
| Direct Agent | Core calls an Agent over gRPC | Agent |
| Edge Agent | Agent maintains an outbound gRPC stream to Core | Agent |

Agents execute Docker and external-tool operations without a database.
The `citadel-docker` crate implements feature ports for local execution; it is not
foundational infrastructure. Agent normal dependencies exclude Core adapters,
SQLx, database migrations and identity cryptography. Core keeps Direct/Edge
routing, target selection and database-aware build and backup orchestration. Core owns
persistence, user authorization, target selection and operation scheduling. Build
Pools use the same Agent executable; Swarm-node Agents use a restricted profile.

## Repository layout

| Location | Responsibility |
| --- | --- |
| `src/server/` | HTTP/realtime presentation, composition, startup and workers |
| `src/agent/` | Agent configuration, authentication, Direct/Edge dispatch and lifecycle |
| `src/features/` | Business models, use cases, typed errors and consumed interfaces |
| `src/infrastructure/adapters/` | PostgreSQL, remote Agent routing, filesystem and security implementations |
| `src/infrastructure/docker/` | Shared local Docker, Compose/build execution and transport mapping for Core and Agent |
| `src/infrastructure/database/` | Declarative schema, migrations and migration runner |
| `src/infrastructure/automation/` | Shared private script workspace and owned cleanup adapter for Core and Agents |
| `src/infrastructure/contracts/` | Core/Agent protobufs and generated bindings |
| `src/infrastructure/docker-api/` | Generated Docker protocol client and pinned source schema |
| `src/infrastructure/processes/` | Bounded subprocess execution, cancellation and cleanup |
| `src/infrastructure/runtime/` | Task supervision, queues, I/O budgets and metrics |
| `src/frontend/` | React application and generated API client |
| `src/tools/` | Generators, build scripts and development tooling |
| `src/.sqlx/` | Offline SQLx query metadata |
| `test/` | Cross-system fixtures, test runners and performance tools |
| `deploy/` | Compose files, environment templates and local settings |
| `schema/` | Generated full and public OpenAPI documents |

Crate-specific tests stay beside their crates. Cargo's workspace and toolchain
files remain at the repository root.

## Dependency rules

- Features may depend on other features, but never on Server, Agent or infrastructure.
- Infrastructure implements feature interfaces and may depend on features or other
  infrastructure crates. It must not depend on Server or Agent.
- Server and Agent compose features and infrastructure. They must not depend on
  each other.
- HTTP types, `utoipa` schemas and API views belong in Server. Generated Docker and
  Agent protocol types stay at infrastructure/transport boundaries.
- Cross-feature dependencies must be acyclic and have a clear owner. Prefer an
  existing narrow interface over a new shared framework.

`primitives` owns small shared concepts such as actor IDs, permissions, audit
metadata and patch vocabulary. `execution` owns process request/result types and
the `ProcessRunner` interface. Concrete process execution belongs in infrastructure.

The architecture tests inspect Cargo dependencies as well as source boundaries:
[resource_structure.rs](../src/server/tests/resource_structure.rs) and
[resource_architecture.rs](../src/server/tests/resource_architecture.rs).
They discover every workspace crate, including target-specific, renamed, build and
development dependencies. Features cannot import database, HTTP or protocol
implementations. Foundational infrastructure stays independent of business features;
the process runner's dependency on the Execution port is intentional.
Feature task checks parse production items and supported macro token groups,
including `select!`, and resolve ordinary imported task aliases. Comments and
string literals are not executable dependencies. Definitely test-only items are
excluded; potentially production `cfg` branches remain checked. These checks do
not expand macros or perform compiler name resolution. Injected task-owner ports
remain allowed. The sole feature-owned blocking-task exception is Identity's
`CredentialWork`, whose semaphore and cancellation lifetime have behavioral tests.
Request cancellation is verified through HTTP/Edge cleanup, not guard spelling.

Source checks cover all features, HTTP/realtime handlers, workers and Agent code,
and keep SQL execution and row mapping in PostgreSQL persistence modules. Test-only
fixtures may use concrete implementations. These guards detect dependency regressions;
they do not replace review of authorization, transactions or business ownership.

## Organizing a feature

Use responsibility-based files within the owning feature. Add only the roles the
feature needs; split a large file into a module when that makes navigation clearer.

| Role | Contents |
| --- | --- |
| `model` | Resource state, value objects and invariants |
| `commands` | Transport-neutral mutation inputs |
| `service` | Use-case orchestration |
| `repository` | Durable persistence interfaces and atomic operations |
| `read_models` | Summaries, configuration results, filters and previews |
| `runtime` | Interfaces for Docker, Agent or external execution |
| `permissions` | Named operation requirements |
| `jobs`, `adoption`, `webhooks` | Feature-specific workflows |

Keep crate roots and module facades focused on declarations and deliberate exports.
Use `name.rs` for a single-file module. When a source module needs child files,
keep its entry point at `name/mod.rs` alongside those children, including private
unit tests in `name/tests.rs`. A cohesive implementation can live in that entry
point; do not add a forwarding file solely to keep `mod.rs` empty. Larger modules
should split by responsibility. Integration tests retain Cargo's `tests/name.rs`
entry points and may keep their supporting modules in `tests/name/`.
Do not introduce empty services or generic base entities just to make every
feature look identical.

### Models and persistence

Use the business resource name, such as `Deployment` or `BackupPolicy`. Resources
may include tags and related metadata loaded efficiently by their repository;
a separate `*Details` wrapper is not mandatory. Use smaller projections when a
list, worker or lookup only needs a subset of the data.

Reuse shared vocabulary where semantics match. Resource-specific state and
validation stay with the feature. Decode persisted enum values and JSON into
feature types at the repository boundary.

Use `Repository` for durable business resources, `Store` for projections, samples
or transient state, and `Reader` for read-only queries. Keep claims, row-version
checks, idempotency and multi-resource transactions in cohesive operations.
Do not replace one atomic operation with several unrelated repository calls.

Worker dependency construction lives in `server/src/composition/workers.rs`.
Executable background-task startup lives in `server/src/composition/background.rs`;
worker implementations remain in `server/src/workers/`. HTTP extraction and
validation belong to `server/src/api/request_validation/`. Citadel release
notifications live in `server/src/release_updates/`, separate from workload updates.
Running workers receive feature persistence interfaces or typed notification sources,
never SQLx pools, rows or listeners. PostgreSQL notification decoding and dedicated
LISTEN connections live in the persistence adapters; reconnect events trigger resync
and durable queues retain their fallback scans. Edge observation persistence keeps
session fencing inside the adapter. Alert sample flushing preserves its advisory
lock and tuple-revision acknowledgement as one adapter operation, while threshold
and version evaluation belongs to the Alerts feature.

SQLx remains available to the Server composition root and database test fixtures.
Do not move SQL into hosting code to avoid adding a narrow feature interface.

SQL and row mapping live under `adapters/src/persistence/postgres/`. Repositories
return feature models, not API views. Catalog queries apply authorization in SQL;
loading an unrestricted catalog and filtering it in memory is not acceptable.
Infrastructure routers may compose persistence helpers from the same adapter crate,
but those helpers return typed data rather than SQLx rows. Do not add a feature port
solely to wrap a private query shared by infrastructure implementations.

The Resource Copies feature coordinates snapshot-backed workload duplication.
Its repository atomically records the reviewed plan and child restores, then
creates the undeployed destination through the existing transactional workload
creation helpers only after every restore succeeds. Backup workers own existing-
backup restore execution. For policy-independent current data, the copy plan stores
per-volume jobs, which the copy worker claims atomically before execution through
its storage port. The runtime adapter takes an encrypted temporary Restic snapshot
and restores an owned fresh volume using a bounded, network-isolated helper.
Current-data jobs require Platform Download authorization and an explicit source
consistency acknowledgement. Cancellation and permission changes cancel the owned
execution token. The worker owns separate reconciliation and execution futures
under the same shutdown token. Current-data execution is bounded to one helper globally, so
long copies do not block other parents' finalization, cancellation or cleanup.
The execution lane retains completed results until their idempotent database
write succeeds. Before claiming new work it fails abandoned claims, including
ambiguous claim commits, without replay; queued jobs remain claimable. Both paths
share finalization and volume ownership receipts.
Copy plans retain separate source and destination identities. The authorized
SQL catalog limits destinations to the source platform type. Existing-backup
copies may restore to another platform or Swarm node using S3-compatible storage;
current-data copies remain on the original daemon/node. Destination authorization,
routing identity and remapped Swarm references are fenced at review, start and
finalization. Snapshot selection always uses source identity; restore children,
volume ownership receipts and cleanup use destination identity. Swarm references
are resolved by name in destination inventory without copying secret/config values.
Optional destination fields preserve older same-platform durable plans.
The copy worker reconciles durable parent state. Runtime storage
checks and ownership-verified cleanup use the Platform runtime router, outside
database transactions. Destination creation commits before resource invalidation.

Copy progress uses an actor-owned `resource-copy:<id>` realtime group.
Persistence publishes operation IDs on `citadel_resource_copies` inside the
transaction that changes a parent or child restore. A supervised listener
invalidates only the affected group; reconnects request a resync. Group reads
reuse the copy service's initiating-actor authorization and persisted progress.

### HTTP and realtime presentation

`src/server/src/api/routes/` owns endpoint registration, extraction and HTTP responses.
`src/server/src/api/resources/` owns shared HTTP/WebSocket public views and mappings,
plus resource-owned HTTP request DTOs and patch handling. Both transports reuse
these representations and actor-specific enrichment.
Keep routes thin: authorize, call the feature, then map the result.

Use `From` for context-free projections and named mapping functions when actor
permissions or other context are required. Preserve field names, nullability and
secret redaction. Feature errors retain their meaning until Server maps them to
sanitized HTTP errors; unexpected failures are logged internally.

Realtime consumes feature interfaces and shared presentation mappings. It does
not import HTTP route modules, request states or HTTP error rendering, execute SQL,
or select Local, Direct and Edge transports itself. WebSocket upgrades remain in
realtime; HTTP mutation-notification middleware lives in
`api/mutation_notifications.rs` and calls the existing realtime publication boundary.
The WebSocket write buffer accommodates the bounded outbound queue's largest
message, including framing overhead. Group snapshots remain capped at 4 MiB;
large supported snapshots must not disconnect the shared connection and force other
resource subscriptions to reload. Container process lists load through HTTP and
refresh only for subscribed Inspect viewers on the existing scoped statistics/state
events. Each event shares one bounded runtime read per container after actor-specific
authorization. A process event read that exceeds the configured realtime read timeout
is cancelled and clears only that process panel; other subscriptions stay connected.
Authorization failures still fail closed. Process snapshots are transient; no database
history or separate polling worker is involved. Health history loads through its HTTP query;
the authenticated health group sends small invalidation notifications on join,
reconnect and committed changes, rather than carrying the retained records.
Automation run history loads through HTTP, then an action-authorized realtime
group replaces the latest 50-run window on join, reconnect and committed run or
cleanup changes. Live snapshots update the frontend query cache and cancel older
HTTP reads. Resource subscriptions own synchronization; execution, cancellation
and cleanup controls do not invalidate those queries. Disconnected views retain
their last data until reconnection or an ordinary HTTP refresh.

## Composition and lifecycle

Server parses and validates configuration before constructing services.
`src/server/src/composition/` builds typed components once and passes narrow handles
to routes and workers. Constructors receive settings explicitly rather than
reading environment variables themselves. Composition constructs health/Git/Edge
adapters and one shared backup-coverage service before route assembly. HTTP coverage
remains available when realtime is disabled. The router assembles endpoint groups
and middleware from these constructed dependencies.

Core applies migrations and acquires its background-job lease before running
workers. Only one Core process may own that lease for a database. Shutdown cancels
listeners and workers, drains tracked work, then closes database resources.
Every spawned task needs an owner, cancellation policy and shutdown behavior.

Build, Backup and Restore workers retain known execution results in their current
slot while retrying completion storage failures, with cancellation-aware backoff
capped at five seconds. They never repeat execution or cleanup to retry persistence.
Each Build claim persists a fresh attempt UUID in the same transaction as its
Preparing transition. Completion and streamed log writes require that attempt,
an active run, and the Project's current-run ownership. A stale controller cannot
write into a newer attempt of the same run. This token fences database writes;
it does not by itself reserve executor capacity or confirm remote cleanup.
A rejected guarded transition is read back: recovered terminal state wins, while
superseded or removed claims end the attempt. Terminal progress follows confirmed
storage. Build failure observations are optional and enter the durable alert queue
in the successful completion transaction. Shutdown may leave an unresolved claim
for existing recovery; the in-memory result does not survive a Core crash.

Agent build admission is shared across command contexts and replacement Edge
sessions. A slot covers the build, its declared pushes and confirmed local
cleanup. Proven pre-execution failures release slots after confirmed cleanup;
uncertain execution retains occupancy. Core defers only an explicit
pre-start capacity refusal (including its own Edge transfer admission), preserving the resolved commit and original queue
age with bounded backoff. Before remote dispatch, Core persists the resolved
commit and an attempt-owned
reservation keyed by Docker daemon identity. Platform/Pool aliases share capacity;
a second Agent authority cannot take over a daemon with outstanding reservations.
Completion releases capacity transactionally for success, verified pre-start
refusal, or a persisted same-authority Agent cleanup receipt. Cleanup checks fence
delayed admission before confirming absence or abandoning a pending push; they
never clear active or uncertain daemon work. Receipt persistence and a system run
log commit together, and receipt persistence alone does not release occupancy.
Startup recovery consumes that evidence; otherwise failures/restarts quarantine
reservations. Project serialization, pool policy and retention include unresolved
reservations. The shared claim query blocks replacement work across all builder
kinds, including Local, before source preparation. A supervised cleanup worker
sweeps terminal reservations in pages of 16, with five-second Agent checks and
30 seconds between pages. It uses the original run target and persisted authority,
never the Project's newly selected builder. Cleanup evidence, reservation release
and a system log commit atomically; the run outcome remains unchanged. Paging
advances past unavailable Agents, and existing receipts can settle offline attempts.
Active work, uncertain execution and changed authorities stay held. Human administrators
can resolve terminal holds after verifying/stopping old work and restarting the original
Agent. A bounded host check must reach the same daemon with a different admission
authority. The release transaction rechecks current administrator access, fences the
exact terminal attempt, and commits an activity and run log with the operator's notes.
Operator acknowledgment is never stored as an Agent cleanup receipt. Run outcome and
completion time remain unchanged; no execution is replayed. Unreachable or replaced
daemons cannot be manually released through this path. Core-local/data-operation
admission remains outstanding.

Interactive streams normally end with their caller. Durable operations use
persisted claims and may continue after a browser disconnect. Recovery inspects
and settles interrupted work; it must not blindly replay ambiguous mutations.
PostgreSQL notifications are wakeups, while claims and queue rows remain authoritative.

Platform and Build Pool connection transitions publish typed events on
`citadel_connection_events` within the transaction that persists the transition
and its activity. PostgreSQL delivers them after commit. The shared
`DatabaseNotificationHub` forwards these to `ConnectionEventHub`; consumers use
`notifications.connections.subscribe()` independently. The realtime subscriber
publishes resource invalidations, and authorized readers load the current state.
Repeated health observations and obsolete Edge sessions do not emit transitions.
Listener reconnects and subscriber lag trigger a resync of both resource types.
This bounded stream is for notifications; durable activities and alert work remain
in PostgreSQL. Adding a subscriber does not require wiring callbacks into stores.

Workload statistics snapshot ownership when samples are admitted. Deployment and
standalone Stack samples are stored transactionally in `workloadstats`, independently
of the container projection; Swarm keeps its existing `swarmservicestats` history.
Queries use the Deployment ID or Stack ID plus service and replica identity, never
an unscoped container name. The same seven-day bounded retention applies to both.
Raw container statistics remain tied to their individual container projection.

## Runtime routing and observations

`PlatformRuntimeRouter` resolves persisted Platform identity or an exact Swarm
node and exposes the capability needed by the caller. Registered Direct consumers
share a connection registry; Edge owns a separate session registry. Registration
validates proposed addresses before they become persisted routing targets.

Keep information, health, statistics, inventory and mutation interfaces narrow.
A container collector should not enumerate images or volumes. Native Swarm
operations validate manager identity, ownership and the whole selection before
mutation. Confirmed successful removals remain committed even if a later item fails.

Preserve these rules in event and recovery work:

- Known container lifecycle events update state without fetching full metadata.
  Metadata changes and deletion use their own observation paths.
- Batches are bounded and scoped to the Platform/node. Generation, timestamp,
  row-version and Edge-session checks reject superseded observations.
- Daemon events must not overwrite operation claims. Operation completion owns
  final parent status, activity and claim release.
- Publish realtime changes after persistence commits, scoped to affected resources.
- Reconnect and periodic reconciliation repair missing observations. Health probes
  do not trigger full inventory collection while a Platform is stable.

Statistics use bounded shared writers and batch database writes separately from
sampling. Realtime can deliver a sample before its history is flushed. Telemetry
retries have finite retention and explicit discard metrics; durable operation
claims and alert observations must not use that lossy policy.

## Authorization and secrets

Identity owns permission evaluation and its process-local cache. Permission
mutations acquire the cache mutation fence **before** a database connection and
publish invalidation through the commit path. Readers retain the matching fence
through query and cache publication. Rollbacks do not publish a successful change.

Known-resource permission checks can be batched. Realtime connections bind their
permission decisions to an actor generation; invalidation closes affected
connections and streams. Raw observations may be shared across subscribers, but
permission decisions and presentation remain actor-specific. Manual ACL changes
outside these mutation paths require cache invalidation, such as restarting Core.

Credential files and Git/Automation workspaces belong to infrastructure adapters.
Use private staging, bounded subprocesses and cleanup on success, failure and
cancellation. Git SSH requires verified host keys; do not disable host verification.
API responses, logs and activity snapshots must not disclose stored secrets.

## Frontend and generated contracts

The frontend uses the full Rust OpenAPI schema. Server owns `utoipa` definitions;
feature crates do not depend on them. Generate both API documents and the TypeScript
client together; see [API changes](DEVELOPMENT.md#api-changes). Do not hand-edit
schemas or generated client types.

Core advertises `WebSocketV1`; the frontend connects only when that capability is
enabled. HTTP queries and live snapshots retain distinct ownership, with explicit
reconnect reconciliation. Swarm HTTP and live overview payloads share the server
projection of persisted quorum, platform control and manager/node-local freshness.
Event-scoped aggregate reads are shared across recipients; capabilities and group
authorization remain actor-specific.

Shared resource pages and UI components own layout, status badges, loading states
and actions. Appearance uses CSS variables and shared density/radius tokens.
Semantic health colors keep their meaning regardless of the selected accent.

Frontend API modules and UI primitives must not import resource screens or the
application router. Features may consume shared resource page templates and their
types; `pages/` is not exclusively a routing layer. The frontend architecture test
checks static imports, re-exports and literal dynamic imports using the existing
TypeScript parser, including relative paths and the `@/` alias.

The Agent protobuf inventory and Docker API generation inputs have their own
contract checks. Changes to either require verification of the generated output
and affected runtime mappings, not just successful compilation.

## Making a change

1. Put business behavior in its feature, external I/O in its adapter and wire
   presentation in Server or Agent.
2. Preserve authorization, transaction, cancellation and recovery boundaries.
3. Reuse an existing type or interface only when its semantics match.
4. Run focused tests, architecture checks and relevant schema verification.
   Use disposable external-service fixtures for integration tests.

Test runners and fixture requirements are listed in [test/README.md](../test/README.md).

### Execution capacity and renewable backup ownership

Build dispatch uses bounded controllers with a separate preparation semaphore.
A host reservation precedes Git preparation; `dispatchstarted` is committed with
the pinned input before transport submission. Preparation failures can release
an undispatched reservation; dispatched ambiguity requires authenticated cleanup.
Preparation ownership follows the actual Direct request or Edge outbound frame,
not the remote build stream. Completion retries retain the same result.

Backup/restore claims have attempt UUIDs. Repository and source leases renew
while execution is polled, and late owners cannot renew or finish a replacement
attempt. A resolved source plan is persisted before acquiring its complete,
sorted source set. Contention rolls back partial acquisitions and defers work.
Recovery interrupts lost owners but never frees storage based only on expiry.
Maintenance and credential validation use the same renewable repository locks.
Each backup/restore queue has one dispatcher and bounded, owned execution tasks.
The shared tracking permit travels with the durable claim through completion
persistence. Initial ownership is verified before execution starts; subsequent
lease renewals stay concurrent with execution while their database requests are
pending. Backup and restore results share the same bounded
completion retry policy. Restic dependencies are reusable; ownership and mutable
admission/cleanup state belong to an explicit operation context for each run.


Data reservations and build reservations share one authenticated daemon authority.
Agent data admission covers an item/restore/maintenance operation across helper
creation, execution and cleanup; the Agent heavy-work budget is shared with
builds. Core's backup tracking limit bounds controllers, independently of host
capacity. Local build/backup/staging work shares one process-wide heavy-work
budget. Local Docker execution also records daemon/authority reservations; the
Core process has one authority shared by its build and backup executors. A new
Core authority cannot overwrite occupancy retained by an old process or Agent.
Automation actions select Core (default), a specific Agent, or automatic placement
within an explicit eligible Agent set. Core Deno slots and remote tracking slots
are independent. Remote Deno processes share Agent heavy capacity with builds and
backups; Core persists ownership before sending the script over signed Direct or
authenticated Edge transport. Both require HTTPS for credential-bearing work.
Run-as tokens are bound to the actor, run and attempt, and stop authenticating after
cancellation, deferral or completion. Cross-host resource requests go through Core's
existing authorized HTTP API, never directly from one Agent to another.
Only a definitive pre-start refusal is deferred; ambiguous dispatch is not replayed.
Cleanup receipts fence delayed dispatch. Retained capacity after an Agent restart
requires an audited human-administrator acknowledgment and verification of a new
Agent authority on the original daemon. This is single-action execution, without
workflow orchestration. Manual, test, scheduled
and webhook runs all enter the same durable FIFO queue; only a worker claims a
run after acquiring the applicable local or remote tracking slot. Source, arguments and run-as identity are frozen
at enqueue. HTTP progress observers use a bounded broadcast and cannot cancel or
backpressure execution. Explicit cancellation is persisted, including during the
queued-to-running transition, and workers observe it before starting a process.
Queued runs survive restart; interrupted executions fail without being replayed.

Before the first external operation, a busy/offline backup host or busy restore
source returns the run to its original queue position with a 15-second eligibility
delay. This releases tracking and business leases, preserves the resolved source
plan, and never defers a partially executed backup. After work starts, the parent
retains storage ownership while its sequential items wait for their next host.

An Agent records helper names before submitting Docker create. Cancellation of
that request is not a definitive create result: a later 404 alone cannot release
its reservation. Confirmed helper identity/removal or a definitive create response
is required. Same-authority reconciliation fences delayed commands. Restarted
Agents and uncertain local helpers require manual operator review.
