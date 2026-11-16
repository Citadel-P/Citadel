# Citadel architecture

This is the reference for ownership, dependencies and lifecycle invariants. Keep it
about the current design; individual fixes, benchmark results and work in progress
belong in change descriptions or investigation reports.

For commands, use [Development](DEVELOPMENT.md) and [Tests](../test/README.md).
For wire contracts, use [Agent protocol](agent-protocol.md). Installation and
recovery procedures live in the [operator guide](content/docs/operations/upgrade-and-rollback.mdx).

## System overview

Core serves HTTP, the frontend and realtime; PostgreSQL stores configuration,
authorization, activities, durable claims and observed resource projections.
Core owns authorization, scheduling, target selection and persistence. Docker and
Agents execute operations. Agents have no database.

Core accesses local Docker through `citadel-docker`, Direct Agents through
signed requests, and Edge Agents through authenticated outbound sessions.
Build Pools use the same Agent executable; Swarm-node Agents expose a restricted
profile. Core, Agents and frontend target one current build.

## Repository layout

| Location | Responsibility |
| --- | --- |
| `src/features/<feature>/` | Business models, use cases, permissions and repository/runtime interfaces |
| `src/infrastructure/adapters/` | Persistence, Docker/Agent connectors, filesystem and security implementations |
| `src/infrastructure/database/` | Declarative schema, migration catalog and verification |
| `src/infrastructure/docker/` | Docker transport and runtime clients |
| `src/infrastructure/contracts/`, `docker-api/` | Agent protobuf and Docker wire contracts/generated clients |
| `src/infrastructure/processes/`, `runtime/` | Bounded subprocesses, task ownership, supervision and concurrency |
| `src/infrastructure/scripts/` | Script SDK and private execution workspaces |
| `src/server/src/api/` | Thin routes and shared HTTP/WebSocket resources |
| `src/server/src/composition/`, `workers/` | Dependency construction and background-job lifecycle |
| `src/agent/` | Agent authentication, execution and Direct/Edge sessions |
| `src/frontend/` | Resource screens, shared UI, query/realtime integration |
| `src/tools/` | Development, build, generation and release tools |
| `src/.sqlx/`, `schema/` | Checked-in SQLx metadata and generated API contracts |
| `test/`, `deploy/` | Test harnesses and deployment templates |

## Dependency rules

- Features may depend on other features without cycles, but never on Server,
  Agent, SQLx, Axum or concrete infrastructure. Infrastructure implements feature ports.
- Server and Agent compose dependencies and never depend on each other. Agent
  production dependencies exclude Core persistence, migrations and identity services.
- Keep SQL, transactions, row mapping and query authorization in PostgreSQL
  adapters. Repositories return feature types, not SQLx rows or HTTP views.
- Keep Docker/protobuf types in infrastructure and HTTP presentation in Server.
  SQLx in Server composition is for wiring, not business queries.
- Shared frontend API/UI primitives must not import resource screens or routing.
  Features may consume shared resource page templates.

Architecture tests enforce these boundaries. Existing narrow exceptions, such as
process runners implementing Execution ports and bounded blocking credential work,
do not justify introducing generic `domain`, `application` or `resources` layers.

## Organizing a feature

Use an equivalent implemented feature as the reference before adding modules.
Create only the parts the feature needs:

| Module | Owns |
| --- | --- |
| `model` | Business resources and invariants |
| `commands` | Use-case inputs and patch semantics |
| `service` | Business operations and orchestration |
| `repository` | Persistence interfaces |
| `read_models` | Consumer-specific projections when the domain resource is insufficient |
| `runtime` | Narrow external-execution interfaces |
| `permissions` | Feature permission definitions |

A standalone module uses `name.rs`; a module with children uses `name/mod.rs`.
Keep `lib.rs` and `mod.rs` to declarations/exports, with implementation in siblings
named by responsibility. Integration tests follow `tests/name.rs` plus
`tests/name/` helpers. Avoid empty scaffolding and automatic `*Details` wrappers.

### Models and persistence

A resource may carry metadata it owns. Introduce a smaller projection when a
consumer genuinely needs less data. Readers perform authorized queries; stores
persist projections; repositories implement business transactions.

Claims, concurrency checks, related mutations and activities that form one
operation commit atomically. Do not replace one atomic operation with unrelated
writes. Carry requested resource identities into queries rather than reading an
entire catalog and filtering afterward.

The schema source is `src/infrastructure/database/src/schema/schema.sql`.
Schema changes append a numbered migration and refresh the catalog. Preserve
applied checksums and stored data; refreshing a baseline is not an upgrade.
See [Database schema changes](DEVELOPMENT.md#database-schema-changes).

### HTTP and realtime presentation

Routes extract, authorize, invoke the use case and map the response. Server API
resources own public HTTP/WebSocket views, input validation and presentation
mappings. Features do not depend on `utoipa` or HTTP types.

Persist changes before publishing scoped invalidations. Notifications are hints;
durable state remains authoritative. Shared reads and raw observations may be
reused across recipients, but capabilities and authorization remain actor-specific.
The explicit exception is loss-tolerant telemetry: realtime may precede its
bounded history flush, never a durable operation or alert commit.

## Composition and lifecycle

Server validates configuration, constructs typed dependencies once and passes
narrow handles to routes and workers. Constructors receive settings explicitly.
`composition/` owns wiring; `workers/` owns execution and shutdown, not SQL queries.

Startup checks retained update/recovery ownership before ordinary migrations.
Core applies migrations and acquires one background-job lease per database before
starting workers. Every spawned task has an owner, cancellation policy and shutdown
behavior. Shutdown cancels listeners/workers, drains tracked work, then closes
database resources. Interactive streams normally end with their caller; a browser
disconnect does not cancel an accepted durable operation.

One shared PostgreSQL notification hub wakes workers. Queue rows and claims remain
authoritative; reconnect scans and bounded fallback polling repair lost wakeups.
Uncached negative queue hints may skip idle recovery transactions, never the
transactional ownership checks when work exists. Dispatchers own bounded execution
tasks and retain admission through cleanup and completion persistence.

Instance owns Jobs diagnostic models and ports; Runtime maintains bounded,
process-local observations; the PostgreSQL adapter owns retained history. Core
owns reporting, retention and compaction under the existing background-job lease.
Diagnostics are best effort and never control execution, claims or recovery.
Missing observations mean unknown state, and a successful check does not prove
that the underlying operation succeeded.

Diagnostic writes are batched and idempotent; storage, maintenance and reads remain
bounded. Core identity and boot identity distinguish instances and counter resets.
Runtime work never waits for diagnostic storage.

The administrator-only Jobs page uses the existing authorized realtime lifecycle.
Live observations do not trigger database reads; committed history changes
invalidate history reads. Resource links retain destination-page authorization.

### Execution and recovery

Build, Backup, Restore and Script attempts use persisted ownership and concurrency
fences. Persist pinned inputs and dispatch intent before external submission.
Completion retries retain the known result; they must not repeat execution or
cleanup merely because storing the result failed. Publish terminal progress only
after confirmed persistence. Logs are redacted and written in bounded batches.

Heavy-work admission is keyed to the canonical daemon and authenticated authority,
so Platform/Pool aliases cannot multiply capacity. Preparation/tracking limits are
separate from executor capacity. Backup repository and source leases renew during
execution; acquire complete source sets atomically and in a stable order.

A terminal run, expired lease, missing reservation, CLI exit or timeout does not
prove external work stopped. Ambiguous dispatch retains ownership. Cleanup and
recovery use the original attempt, daemon, authority and helper identity, not the
resource's newly configured target. Persist creation intent before Docker create
and confirmed helper identity before start. Never replay an ambiguous restore.

Release retained capacity only with the required authenticated cleanup evidence
or the explicit, audited administrator recovery path. Changed/unreachable
authorities remain held. Operator acknowledgment is not an Agent receipt. Private
workspaces, credentials and staging cleanup remain owned through cancellation and
shutdown; age alone does not authorize deleting interrupted work.

Scheduling uses bounded candidate pages and transactional enqueue fences. Existing
Community behavior and Team entitlement checks apply at selection and dispatch;
cancellation and recovery remain available after a downgrade.

## Runtime routing and observations

`PlatformRuntimeRouter` resolves a persisted Platform or exact Swarm node and
exposes the caller's required capability. Features and realtime do not branch on
Local/Direct/Edge. Direct clients share composition-owned connection registries;
Edge has a separate session registry. Transport reuse must not cache authorization,
health, credentials or request-specific tokens.

Keep inventory, health, statistics, information and mutation interfaces narrow.
A collector must not enumerate unrelated resource types. Reduced collector
projections must preserve full inspection for consumers that need it.

- Known lifecycle events update state without full metadata inspection.
- Batches are bounded by Platform/node; generation, timestamp, revision and Edge
  session fences reject superseded observations.
- Accepted inventory refreshes advance freshness; unchanged semantic payloads
  need not be rewritten. Missing-resource removal requires the complete snapshot.
- Daemon observations cannot finalize a parent operation or overwrite its claim.
- Reconnect and periodic reconciliation repair missed events. Stable health probes
  do not trigger full inventory collection.
- Swarm mutations validate manager identity, ownership and the whole selection
  before execution. Confirmed successes remain committed after a later item fails.

## Statistics and database work

Sampling and persistence are separate. Shared bounded writers batch telemetry,
with finite retries and explicit discard accounting. Durable alert observations
and operation state must never use the lossy telemetry policy.

History retains recent raw samples and compacted older data. Minute summaries
preserve weighted gauges, counter resets, nullable values and source identity,
including removed containers/nodes. Compaction uses the background lease,
transactional watermarks and bounded work; pending alert observations block unsafe
deletion. Sealed intervals reject late writes. Sparse or oversized intervals may
stay raw rather than forcing expensive or larger summaries.

Optional fifteen-minute Platform I/O summaries cover only fully sealed compatible
blocks. Reads use them only for complete blocks at compatible resolution, falling
back to minute/raw data for partial boundaries and mixed intervals. Maintenance and
retention cover both tiers without discarding the fine history needed by other
ranges. This trades some storage/writes for less long-range aggregation work.

History queries filter sources and time ranges before aggregation, preserve
per-container counter baselines, and share bounded read admission. Query timeout
and local JIT policy apply within the transaction. Verify short, long and mixed
ranges with custom and generic plans; optimizing one preset is insufficient.
Implementation and regression cases live in
[statistics persistence](../src/infrastructure/adapters/src/persistence/postgres/platforms/statistics/)
and [statistics tests](../src/infrastructure/adapters/tests/stats_batch_persistence/).

Retention drops verified expired partitions and bounds boundary/default-partition
cleanup. It must not detach a populated default partition or delete pending alert
work. Analyze only affected parents after commit; avoid routine whole-table
rewrites. Compaction does not promise an immediate drop in container memory:
PostgreSQL buffers, backend allocations and reclaimable filesystem cache differ.

Query-pool limits are per Core; notification and background-lease connections are
additional. Admission leaves capacity for other work without reserving a dedicated
writer connection. PostgreSQL settings and container limits belong to deployment,
including when the database is external.

## Instance configuration

Instance owns typed nonsecret installation settings. Mutations reauthorize an
enabled human administrator, validate coupled settings and deployment overrides,
and commit settings/activity atomically; unchanged settings create no activity.
Explicit environment values override saved settings, which override defaults.
Saved, effective and currently running values are distinct; only settings designed
for live application change without restart.

Core restart uses durable admission shared with update maintenance, drains owned
work and releases database resources before re-execution. Boot identity fences
replays, and startup acknowledgment follows listener readiness. This does not
imply coordinated multi-Core restart support.

## Instance updates

Updates owns release selection, compatibility, reviews and operation invariants.
Adapters own bounded release HTTP access, private filesystem journals and
transactional state; Server owns scheduling and presentation. Release-check
notifications use shared realtime/reconnect flows, without a second polling system.

Review verifies completed publication, signatures, immutable image identity,
architecture/channel/protocol compatibility, migration history and registration.
Verify target trust before executing even its inspection commands.
Eligibility follows verified target metadata and the installed migration prefix,
not a manually maintained list of source versions. Synthetic fixtures alone do not
qualify a production release transition.

Supported Linux Compose installations bind the original ordered inputs, private
state, daemon, configuration, storage and key identities. Packaged installations
can register automatically after setup; CI-managed deployments retain their
explicit registration/refresh workflow. Registration does not recreate volumes or
silently accept drift. See the [operator guide](content/docs/operations/upgrade-and-rollback.mdx)
for supported profiles and setup, and [release publication](release-publication.md)
for deployment checks.

Approval claims a durable operation and transfers execution to an owned helper
that survives Core replacement. Journal intent precedes mutation. Checkpoints,
review fingerprints, installation locks and startup receipts fence retries and
replacement. Closing the browser or losing connectivity proves neither failure
nor success. The UI retains the last confirmed phase and reconciles on reconnect.

Recovery distinguishes failures before source shutdown from verified target
completion. It must prove unchanged source state or validate the exact committed
target receipt before releasing maintenance ownership. Never delete journals,
clear claims, replace keys or refresh registration merely to bypass a failure.

PostgreSQL engine updates are separately reviewed operations; Core updates retain
the database image. The current executor supports qualified PostgreSQL 18 minor
updates for its registered Alpine layout, not major upgrades, replication,
external tablespaces or externally managed databases. Review, approval and
execution recheck eligibility. Unsupported/ambiguous states preserve evidence and
ownership for explicit recovery.

## Authorization and secrets

Identity owns permission evaluation and caching. Mutations acquire the cache
mutation fence before a database connection and invalidate through the commit
path. Readers retain the corresponding fence through query/cache publication.
Realtime authorization is actor-generation-specific; invalidation closes affected
connections/streams. Out-of-band ACL edits require explicit cache invalidation.

Deployment-selected shared user IDs restrict self-service profile/password/MFA
changes and session revocation in the services, not just the UI. Shared public
accounts remain non-administrator/read-only; existing administrator recovery and
MFA requirements still apply.

Secret resolution honors effective scope precedence before fetching values;
invalid winning bindings fail rather than falling back. Reuse HTTP transports,
not grants, tokens or plaintext values. Preserve secret-use authorization, verified
Git host keys, signed Direct requests and Edge/Swarm-node allowlists. Responses,
logs and activities must not expose stored secrets.

## Frontend and generated contracts

Rust Server definitions generate OpenAPI, TypeScript resources and the script SDK;
see [API changes](DEVELOPMENT.md#api-changes). Never hand-edit generated contracts.
Agent protobuf and Docker generation changes require their contract checks and
runtime mappings across Core/Direct/Edge, not compilation alone.

Register screens through the shared resource list/form/detail factories in
`src/frontend/src/features/index.tsx`. Reuse FormShell/FormBuilder, resource pages,
shared appearance tokens and semantic health colors. Avoid bespoke routing and
refresh mechanisms where existing abstractions fit.

Core advertises realtime capability; the frontend connects only when enabled.
HTTP reads and snapshots retain distinct ownership. Scoped invalidation,
authoritative mutation responses and reconnect reconciliation keep caches current;
cancel stale reads before applying authoritative replacements. Partial snapshots
preserve omitted fields. Shared transport/cache machinery does not replace
resource-specific invalidation or actor-specific access checks.

## Making a change

Trace the relevant path, identify invariants and use an equivalent feature as the
reference. Put business behavior in the feature, external I/O in its adapter and
presentation/wiring in Server or Agent. Preserve transaction, authorization,
cancellation and recovery boundaries; run the focused checks in
[test/README.md](../test/README.md). Update this guide when a design rule changes,
not for every implementation adjustment.
