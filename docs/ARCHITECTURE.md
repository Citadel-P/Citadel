# Citadel Rust architecture

This describes the Rust architecture and ownership boundaries for Deployments,
Stacks, Swarm Services, Builds, Git, Backups, Automation, Alerts, Platforms,
Identity and shared resources.

## Container hot-path alignment (2026-09-27)
batch the state projection:

- `ContainerChange::state_delta` classifies start/die/pause/unpause. The Agent emits
  ID/action without enriching these events; Core also accepts older enriched events.
- `status/state_delta.rs` updates bounded arrays of scoped identities with one
  narrow SQL statement. It does not serialize inventory JSON, insert containers,
  resolve image/ownership joins, replace metadata or release operation claims.
- `ProjectionWrite` and the existing observation watermark remain authoritative;
  `RuntimeIdentityIndex` supplies verified hints with a bounded stale-hint fallback.
- Local, Direct and Edge gather at most 256 raw state events within a fixed 75 ms
  window. Newest observations win per identity; metadata, tombstones, recovery and
  scope changes flush the batch before proceeding. Edge retains its session fence.
- Each scope uses one projection guard and set-based state transaction. External
  parent IDs are deduplicated; Stack reconciliation shares that transaction and
  Deployment reconciliation retains its separate post-commit lock order.
- Unmanaged and operation-owned rows skip parent reconciliation. Owned lifecycle
  observations preserve claims and suppress Stack drift; command completion still
  owns final parent effects. Changed IDs publish as one realtime invalidation.
- Metadata and tombstones keep their separate paths. Uncertainty requests only
  Container reconciliation, and scoped realtime invalidations follow commit.
- Operation events confirm in-flight commands; durable finish/recovery finalizes
  parent status and activities once. Missing daemon confirmation uses bounded,
  read-only Inspect fallback fenced by the pre-inspection projection generation.
  Superseded inspections retry at most three times; durable recovery retains failures.
  Standalone completion skips parent tables; parent claims preserve their lock order.
- Lifecycle patches route to `Containers`, `DockerDaemon`, and matching Container
  detail subscriptions. Parent updates carry affected parent and Platform IDs;
  ordinary state patches do not invalidate Platform/workload summaries.
- Container telemetry uses a shared narrow identity query, with linear sample
  mapping. Platform telemetry uses the current sample plus a narrow context query,
  independent of the statistics flush cadence. Details consume partial patches.
  Initial snapshots and metadata changes retain authorized full reads.
- Known-ID authorization uses the bounded actor/resource cache where safe;
  mutation claims hold the cache read fence and recheck locked ownership. Catalog
  visibility remains SQL-filtered.
- Alert delivery wakes on transactional outbox notifications and retry deadlines;
  a 60-second fallback recovers missed signals. Platform health transitions are
  owned by the five-second hysteresis monitor; `/ready` checks dependencies only
  when requested. The configured monitoring interval belongs to stats sampling.
- Local, Agent and Edge stats use bounded queues and shared writers.
  `JobConfiguration__MonitoringInterval` sets sample cadence; the configured
  flush interval and row threshold set persistence cadence.

## Runtime review implementation (2026-09-27)

- Fast container sampling and slow metadata refresh have separate lifetimes. One
  sampler owns one metadata task; generation changes, cancellation, and cache age
  prevent an old result from becoming fresh telemetry. CPU coverage is complete
  with bounded concurrency. Wire capture timestamps preserve sample identity.
- Direct commands resolve Agent clients through the same registered Platform
  connection owner as workers. Agent lifecycle RPCs share a daemon-wide budget;
  health has separate capacity. Edge health bindings are read as one batch.
- Non-Swarm resource lanes allow two active Platform scopes under the existing
  shared inventory budget. A Platform/resource retains one active collection,
  generation fencing and bounded coalesced follow-up work.
- Statistics keep successful cross-source batching, but split failed work by
  Platform. Retry age/capacity limits and explicit discard counters keep failures
  finite. Durable operation claims and alert observations are never governed by
  this lossy telemetry policy.
- Job alert observations are durable snapshots, with per-rule evaluation receipts;
  PostgreSQL notifications are wakeups. Container detail patches are scoped to
  canonical identity, including across navigation and late callbacks.

## Architecture consolidation (2026-10-01)

The consolidation retains the enriched Deployment, Stack, SwarmService and Platform
resource strategy. Related data is hydrated by repositories; no entity/detail split
or new public resource family is introduced.

The shared routing and permission foundations are:

- `adapters/connectors/routing/platforms/registry.rs` owns the existing direct Agent
  channel cache and target refresh lifecycle. Server composition creates one registry
  and shares it with workers and command routing; Edge retains its separate session
  registry. Registration still validates previously unregistered addresses directly.
- `PlatformRuntimeRouter` selects the persisted Platform or exact Swarm node. Its
  persistence reader returns typed connector/status fields and owns routing SQL.
  HTTP capability resolution, container commands, volume helpers, workload Agent calls and
  node-agent lifecycle operations delegate to this resolver. Workload metadata reads
  are reused for execution resolution, avoiding an additional lookup per command.
- Resolution checks persisted identity before using a cached channel. Address changes
  refresh the existing registry; deletion prevents resolution even while a cached
  channel remains. Node freshness and live manager identity checks are preserved.
- Platform read models expose `PlatformKind` and `ConnectorKind`; the API maps these
  explicitly to its transport vocabulary. Registration inputs still represent
  unsupported boundary values so existing validation responses remain possible.
- Persistence decodes routing-relevant descriptor fields into `PlatformRoutingMetadata`.
  Stored casing aliases remain in the adapter; the complete original JSON is retained
  for API metadata and lossless round trips. Manager checks use typed identities.
- Realtime no longer imports HTTP routes or Platform HTTP state. Log and terminal
  streams resolve through existing feature capability ports. Snapshot DTO mappings
  and authorized Automation/Build reads are shared at the transport mapping boundary;
  no SQL or connector dispatch moved into those shared helpers. Existing permission
  leases, shared event reads, stream bounds and cancellation remain intact.
- Stack Terminal and Pull are configurable permissions. Build queue/cancel and
  Backup restore use named requirements. Catalog grant minimums remain distinct
  from operation requirements: independently assigned grants can combine, and
  existing Read + Apply/Restore assignments are neither promoted nor rewritten.
- Identity service-account usage tracking lives in the Identity persistence adapter,
  using runtime's generic bounded queue. It remains best effort and coalesced;
  rejected observations increment `ServiceAccountUsageDropped`. Generic runtime
  has no dependency on Identity.

Verified against branch `feat--migrate-to-rust`, revision
`b13f5e122545ccc3b8e9bdec6e6674daee159366`, before this uncommitted pass:

| Finding | Verification and disposition |
| --- | --- |
| Independent direct-Agent channel construction | Confirmed; registered consumers now share the existing registry. Registration still validates unregistered addresses. |
| Realtime importing HTTP routes/state | Confirmed; removed, including Automation, Builds and Activities helper imports. |
| String Platform/Connector classifications and JSON routing probes | Confirmed; read models and routing metadata now use domain types with persistence-owned parsing. |
| Missing Stack Terminal/Pull catalog entries | Confirmed; added without changing operation authorization levels. |
| Read + Apply/Restore catalog minima | Not a defect: grants combine across assignments. Existing catalog semantics retained. |
| Identity tracker in generic runtime | Confirmed; moved to Identity persistence and exposed best-effort drops. |
| Enriched resource models | Intentional; retained. |
| Platform HTTP SQL/orchestration and sensitive workspace I/O | Confirmed; moved behind feature ports and infrastructure implementations in the completion pass below. |

The next stage moves narrow HTTP persistence reads behind `PlatformReader` and
`PlatformReadService`: Platform classification, Swarm initialization detection,
Deployment/Stack container selection, image identity/node selection and image
registry association. Deployment selection stays bounded to two rows and rejects
ambiguity; Stack selection stays scoped to the requested Stack. Both recheck
ownership after loading the container. Swarm network deletion preconditions reuse
the existing scoped projection read and reject missing, stale or in-use networks.
HTTP maps feature NotFound/Conflict errors without changing the public contract.

Platform HTTP receives `StatisticsReader` and `SwarmServiceRepository` from
composition instead of constructing PostgreSQL repositories per request. HTTP and
realtime share the injected statistics reader. Runtime mutation orchestration and
transport dispatch are still separate follow-up work; the HTTP state still owns
infrastructure dependencies needed by those unmigrated paths.

Validation of this read-boundary stage: Server/test compilation, all 60 Platform
unit tests and eight isolated PostgreSQL/API regressions passed. The regressions
cover Deployment/Stack inspection, image node routing, concurrent inventory
initialization, Platform/Service/workload statistics authorization, managed-Service
realtime reads and network deletion preconditions.
Formatting and OpenAPI drift checks passed (404 full / 305 public operations);
the generated API and frontend schemas remain unchanged.

The direct-SQL stage removes the remaining SQL execution from Platform HTTP.
Registry browsing resolves an ID through `RegistryRepository`, authorizes that
resource, then loads the registry through the same repository. Managed Swarm
service ownership uses a Platform-scoped repository query. Confirmed daemon
deletions use the injected `InventoryProjectionStore` with a closed
`SwarmResourceKind`; unsuccessful batch siblings remain visible for reconciliation.
The HTTP architecture guard rejects direct SQL execution in these routes.
This stage passed Server/adapter test compilation, five architecture checks and
five isolated PostgreSQL/API regressions covering registry authorization and
status, native Swarm preflight, partial deletion, and Platform-scoped ownership
and projection removal. Both registry creation/credential checks also passed.
Formatting and OpenAPI checks passed; the 404 full / 305 public operations and
frontend schema remain unchanged.

Finite container/image inspection, image exposed-port reads, manager Task reads
(including statistics identity checks), and finite log reads now use the existing
feature capability ports exposed by `PlatformRuntimeRouter`. Transport dispatch
lives in the adapter; HTTP retains authorization and response mapping. Resolution
and execution share the request cancellation token, with a drop guard on every
migrated path. Existing deadlines, redaction, stale-projection validation and
exact-node routing are retained. Architecture tests prevent those handlers from
switching transports again; mutation operations remain for the next stage.
This stage passed Server/adapter test compilation, six isolated PostgreSQL/API
regressions, six architecture checks and the early-cancellation regression for all
read capability factories. Log fixtures now grant permissions through the user
mutation boundary (with real users), preserving cache invalidation. Formatting
and OpenAPI verification passed with no API or frontend schema changes.

Network and volume listing, lookup, inspection, creation and deletion now use
feature capabilities on the shared router. The Platform feature owns network
batch preflight and sequential network/volume deletion. HTTP still validates
requests, authorizes access and publishes each confirmed removal. Unsupported
Swarm node-local mutation targets remain rejected. Preflight rejects protected/in-use/Stack
networks and stale or missing Swarm observations before deleting any target.
NotFound on deletion counts as confirmed absence; other failures stop the batch,
preserve successful notifications and retain the existing partial-result message.
No retry or full inventory scan was introduced.

This stage passed Server/adapter test compilation, five isolated PostgreSQL/API
regressions, six architecture checks and the early-cancellation test extended to
network and volume capabilities. The regressions cover authorization, Edge and
exact-node routing, lookup behavior, full network preflight, and notifications
and partial results after a deletion failure. The temporary database and role
were removed after testing. Formatting and OpenAPI verification passed: 404 full
and 305 public operations, with no API or frontend schema changes.

Image pull/deletion and Platform prune now resolve feature capability ports through
the shared router without HTTP transport switches. Pull progress validation and
matching the requested image against fresh inventory belong to the Platform
feature. Cancellation interrupts a stalled pull stream or progress queue before
inventory observation. HTTP retains admission limits, the tracked task/body
lifetime, deadlines and notifications. Image deletion retains its independent
tracked task so reconciliation runs after caller disconnection; its existing
claims, partial-failure observation and no-retry behavior remain. The review
follow-up below adds missing generation and row-version fencing to deletion completion. The deletion adapter now requires only `ImageInventoryPort` for its
post-operation observation. Registry preparation and image persistence/claim
helpers remain infrastructure-owned and are still called by HTTP; moving that
orchestration behind a feature service remains a separate boundary step.

Focused image-stage validation passed the three Platform pull tests (reference
matching, error sanitization and stalled-progress cancellation), the early
cancellation router test, six architecture checks, and three isolated HTTP/SQL
regressions for partial image deletion and Local/Edge pull/prune. The local pull
fixture now supplies the container inventory required by image usage calculation.
The stalled-client response-body regression also passed (14 focused tests total).
Formatting and OpenAPI checks passed with unchanged 404 full / 305 public
operations and frontend schema. The temporary database and role were removed
after testing.

The completion pass removes concrete clients, PostgreSQL stores and the database
pool from Platform HTTP state. Composition owns their construction; worker and
diagnostics dependencies are wired independently. `PlatformRuntimeProvider`
returns existing narrow capability interfaces through the shared resolver, without
another channel cache or a universal Docker execution interface. Realtime uses the
same provider. Flat route files and enriched resource models are retained.

- Native Swarm preflight, manager checks, version/ownership validation, batch
  mutation and refresh completion live in the Platform feature. Confirmed removals
  are persisted and notified even when a later sibling or the refresh fails.
  Platform uses the Swarm Service repository only to identify managed service
  ownership before native mutations; managed service commands remain in their
  own feature. This is an intentional, acyclic read dependency.
  Persistence failures keep their HTTP error classification. Network topology
  forwards its lightweight capability instead of falling back to usage collection.
- Platform management validates proposed addresses before persistence through a
  candidate-validation port. Registration and updates intentionally bypass the
  persisted-address cache for uncommitted candidates. Platform deletion commits
  first, then disconnects sessions and publishes removals without another await.
- Node Agent setup, removal and coverage use injected lifecycle/runtime/read ports.
  Edge enrollment/status/revocation use feature-owned target and session interfaces;
  Builds and Platforms share HTTP presentation outside the route modules.
- Image pull preparation and claims use `ImageMutationStore`; SQL lives in Platform
  persistence. The feature coordinates observed-image persistence, completion
  notifications and deletion reconciliation. Tracked task ownership, deadlines,
  claims, generation fencing and bounded progress remain intact.
- Registry browsing, volume content, backup coverage and download audit records use
  injected feature ports. Download completion still controls audit recording.
- Git credentials and clone staging use `GitWorkspacePort`; Automation uses a
  private script workspace lease. Implementations live in
  `adapters/filesystem`; the process runner remains independent of these features.
  Explicit cleanup removes credential-bearing files; abandoned futures transfer
  recursive removal to tracked blocking work. Cleanup admission is reserved before
  acquisition and retained through shutdown drain. Atomic clone publication and
  private filesystem permissions are preserved. Automation refuses an existing run-directory symlink. Backup
  staging cleanup is delegated to its existing source planner adapter.

The 2026-10-01 consolidation review follow-up tightens these guarantees:

- Runtime errors retain their kind through HTTP. Internal routing and image SQL
  failures use the common logging/sanitization boundary; safe conflicts,
  availability failures and invalid image references retain their public classes.
- Node Agent reconciliation policy is feature-owned and injected into HTTP
  initialization/refresh, lifecycle inventory writes and background reconciliation.
- Image deletion captures a generation before observing inventory and checks both
  that generation and claimed SQL row versions before removing projections. A
  newer pull cannot be deleted by an older absence. Claims are released and counts
  repaired even when an observation is rejected.
- Image deletion remains independent of browser disconnection, but resolution and
  daemon calls observe process cancellation. Shutdown skips network repair and
  allows at most two seconds for claim completion; interrupted cleanup leaves an
  expiring durable claim rather than replaying destructive commands.
- `SwarmOperations` owns native preflight, whole-selection validation, runtime
  resolution, manager verification, dispatch deadlines and refresh/partial-removal
  completion. HTTP owns extraction, actor authorization and response presentation.
- Each coarse realtime network/volume event shares runtime resolution together
  with inventory collection. Authorization and capability projection remain per
  actor. Swarm image replacement payloads preserve image capabilities; the frontend
  requires explicit image capabilities instead of allowing absent values.
- Git SSH uses strict host verification and shell-safe client-key arguments.
  `Git__KnownHostsPath` identifies the operator-provisioned trust file (default
  `<CITADEL_DATA_ROOT>/git-known-hosts`); unknown/changed keys fail closed, including
  recursive submodule requests. No automatic trust enrollment is performed.

Review follow-up validation passed 1,080 workspace tests (314 environment-dependent
cases ignored by the default runner), strict workspace Clippy, formatting, 19
focused PostgreSQL/HTTP regressions using isolated databases, a live SSH
trusted/unknown/changed-key test, three frontend capability tests, TypeScript
checking and linting of the changed frontend files. The focused database runs
include Node Agent setup/lifecycle, nondefault reconciliation policy, stale image
deletions, direct Swarm operations and per-subscriber realtime capabilities.
Temporary databases, roles, SSH server and test keys were removed after testing.
OpenAPI verification passed with unchanged 404 full and 305 public operations;
no generated frontend contract changes were needed.

Architecture guards reject concrete transport/persistence dependencies in Platform
HTTP and sensitive filesystem operations in these feature workflows. Release/CI
changes and event-system redesign remain separate concerns.

Completion validation:

- Workspace/all-target compilation passed. The seven consolidation architecture
  checks pass, including capability ownership and sensitive filesystem boundaries.
- All 77 Platform HTTP cases and 13 Platform creation/deletion cases passed across
  the complete isolated run and corrective reruns. Fixtures that changed grants
  after a cached denial now use User/Team mutation repositories. Assertions and
  production authorization were not weakened. Malformed Platform PATCH fields
  now retain their field name in the structured validation envelope.
- Six Git execution and one Automation execution PostgreSQL tests passed, as did
  the direct-channel reuse/address-change/deletion regression. Every database and
  test role was isolated from the running application and removed afterwards.
- Four workspace lifecycle/security tests, two real Git CLI tests, and the reviewed
  Stack permission-catalog differential test pass. Git workspace integration tests
  now live with their filesystem adapter, leaving the generic process crate free
  of Git/Automation dependencies.
- A disposable image containing the rebuilt Agent passed authenticated finite logs,
  redaction, interactive terminal and cancellation acceptance. Its containers,
  network and image were removed. This exercised the debug Agent executable on
  the existing Agent runtime image.
- OpenAPI verification passed: 404 full and 305 public operations, with no changes
  to generated API/frontend schemas. Formatting and diff whitespace checks pass.

The validation follow-up corrects the failures discovered by the first full run:

- ACL and feature architecture scans exclude dedicated test files and test
  directories. Alert configuration fixtures live in their own test module; the
  reviewed production ACL owner list is unchanged.
- The container protocol fingerprint includes the existing additive fields
  `ListContainersRequest.metadata_only = 5` and
  `ContainersStatsResponse.captured_at = 2`. Removing precisely those additions
  reproduces the former fingerprint. Wire tests pin their numbers and defaults.
- Build and Backup PATCH serialization lives with the API resource inputs.
  Container batch and Edge worker tests use conventional module paths without
  `#[path]` overrides. Edge enrollment presentation returns a view; routes own
  response headers and rendering.
- Complex callback/cache types have names; related backup-operation and container
  observation arguments travel together. Large platform enum payloads are boxed.
  Redundant conversions, borrows, map lookups and nested conditions are removed
  without adding lint suppressions or changing public wire contracts.
- The Stack differential assertion explicitly accounts for the two reviewed
  Terminal/Pull catalog additions while preserving the documented wire contract.

Follow-up validation: the full workspace suite passes (1,075 passed, zero failed,
308 ignored external/integration cases). Twelve selected ignored database cases
also pass in isolated databases: alert configuration caching, backup operation
leases, container state deltas, Edge identity/replay, Platform and Build Pool
HTTP enrollment contracts, and authorization cache invalidation. The temporary
database and role were removed after the run. OpenAPI verification also passes
with unchanged 404 full / 305 public operations and frontend schemas. Final
workspace/all-target Clippy passes with `-D warnings`; formatting and diff
whitespace checks also pass.


## Workspace groups

The workspace separates executable hosts from feature and infrastructure crates:

```text
src/
  server/                         # API and executable composition root
  agent/                          # Agent executable and transport composition root
  features/
    deployments/, git/, stacks/, ...
    primitives/                   # shared actor, permission, audit, patch and scheduling vocabulary
    execution/                    # process requests/results and ProcessRunner port
  infrastructure/
    adapters/                     # existing concrete integration crate
    database/                     # schema, migrations and migration runner
    docker-api/                   # generated Docker protocol client
    contracts/                    # authoritative Agent protobufs and generated bindings
    processes/                    # bounded OS process execution
    runtime/                      # task supervision, queues, metrics and I/O budgets
```

Each existing feature remains a separate Cargo crate. Directory grouping does not
merge the features into one application crate. Existing package names remain stable;
`citadel-processes` and `citadel-runtime` explicitly own the extracted implementations.
Features may depend only on other Features workspace packages. Infrastructure may
depend on Features and other Infrastructure packages, never Server or Agent.
Server and Agent may depend on both to compose implementations, but not on each
other. The architecture test checks actual Cargo
metadata, including build, target-specific and development dependencies.

Git and Automation receive a `ProcessRunner` implementation through constructors.
Git retains its `GitProcessPort` name as an alias for that shared port. Production
composition injects Infrastructure's `SystemProcess`. Process cancellation, output
limits, child termination/reaping and redaction retain their existing behavior.
Real subprocess/Git integration tests live with Infrastructure; feature unit tests
use injected process doubles. Runtime utilities have moved out of application services,
and no Feature depends on the concrete process or hosting runtime crates.

This grouping establishes crate dependency direction. Activity and licensing
services live with their feature owners; their presentation types live in Server.
Remaining source-level cleanup includes existing SQL in HTTP handlers and feature
filesystem operations; the folder move does not claim those have been eliminated.
Generators, build inputs, development instructions and architecture checks use
the current source paths.

## Agent protocol ownership

`infrastructure/contracts/proto` owns the ten Core/Agent protobuf sources.
`citadel-contracts` generates clients and servers from those files only; feature
crates do not consume protocol DTOs. The accepted source baseline is covered by
`contracts/tests/protocol_sources.rs`, and Edge protocol version remains 2.
Docker API generation similarly consumes the Rust-owned
`infrastructure/docker-api/codegen/v1.49.yaml` schema.

CI checks Rust without submodules and runs `test/scripts/check-source-ownership.sh`
before generation, workspace checks and tests.

The [Agent protocol inventory](agent-protocol.md) documents operations and
Swarm-node restrictions. `contracts/tests/agent_protocol.rs` checks it against
compiled protobuf descriptors and validates the four Agent environment templates.
Runtime behavior is covered by the Agent integration and compatibility suites.

`citadel-agent` owns configuration, TLS validation, unsigned process health,
bounded signal-driven shutdown and all 68 Direct RPCs across eight services.
It composes the shared `DockerClient` without opening a database connection.
Direct mode binds all IPv4 interfaces. Edge profiles expose health on loopback
and establish an outbound HTTP/2 connection to Core.
The shared Docker endpoint supports Unix sockets and explicit TCP/HTTP addresses
for generated requests, raw streams and Docker CLI execution. The Unix socket
default is unchanged. Unsupported schemes fail during configuration.

Local execution is reusable without a database: `DockerClient::apply_container_config`
creates, starts and observes deployments; `external/stacks::LocalStackApply` stages
and applies Compose/Swarm sources; `external/builds::runtime::DockerBuildSession`
owns temporary credentials, build/push execution and redacted progress. Core
routers retain persistence, Git materialization and target selection. Agent
handlers map wire requests onto these same runtime operations. Transported
build archives go directly to Docker stdin and are never unpacked by Citadel.

Direct authentication verifies the first raw protobuf message against the existing
Ed25519 method/body signature before dispatch. This preserves map-field bytes and
the established signing format. Replay protection is atomic and bounded, with
expiry at request timestamp + 60 seconds. Health is the only unsigned endpoint.
The transport enforces message limits and request deadlines; streaming responses
own their cancellation guards and bounded queues. Dropping build/stack streams
kills their child process and removes private temporary files. Successful Compose
secret bind mounts retain their private files until a later successful replacement.

`agent/src/edge` owns durable enrollment identity, Ed25519 challenge signing,
additive Core TLS trust, Docker identity observation, heartbeats and reconnect
backoff. Key and identity writes are private and atomic; corrupt state is preserved
and rejected. Agent hosting owns the Edge future and drops it on shutdown, so no
independent connection/heartbeat tasks survive the listener. Core authentication
storage failures return `Unavailable`, preserving enrolled identities on retry.
Edge dispatch exhaustively maps all 68 commands to the same protobuf operations
used by Direct RPCs, including shared interactive exec. A session owns at most
16 command futures; dropping it cancels streams and subprocesses. Each command has
its own cancellation token/deadline, and interactive input is bounded to 256
messages. Both queued input and output have shared 32 MiB byte budgets in addition
to the 16 MiB envelope limit and 512-envelope output capacity. Duplicate active
command IDs close the session; invalid targets/schemas never reach Docker. Each
command emits one terminal outcome. Swarm-node execution requires the configured
node identity and an explicit command allowlist. Helper creation validates mounts,
ownership, privileges and command arguments; binary exec rechecks the actual
container configuration and uses its immutable ID. Restore-volume mutations require
platform ownership and bounded creation settings, and deletion rejects volumes in
use. These checks run inside the command's cancellation/deadline scope.

Core and Agent share `src/tools/build/version.rs` for compile-time product version metadata.
`Dockerfile.agent` owns the Agent image, including its runtime tools and Rust
volume helper; `Dockerfile` owns Core. The Agent health check calls its loopback
probe, including Direct TLS. Agent uses Alpine runtime tools with isolated glibc
libraries from Core's pinned runtime source to run the GNU Rust binaries.
The independent `agent.yml` workflow validates the
Agent, contracts and helper, builds native amd64/arm64 release images from an
isolated Rust-only context, and runs smoke and compatibility tests before optional
publication. Stable Agent aliases remain gated until compatibility cutover.

## Runtime configuration

Server parses environment settings once in `config.rs` and `config/execution.rs`,
validates them before service construction, and injects typed options through
composition. Features and adapters do not look up these hosting settings.
`deploy/.env.example` documents settings for installing and operating Citadel;
`DEVELOPMENT.md` explains production and development env-file selection.
Keep env templates focused on operator decisions. Queue capacities, polling and
lease intervals, buffer limits, tool paths and implementation-specific tuning
belong in developer documentation or code defaults.

Comments and current documentation explain Citadel's behavior and constraints,
without referring to the language or framework of an earlier implementation.

Service-account limits are shared by token issuance and the API's limits response.
Node-agent setup receives its configured timing, architecture and resource policy.
Direct Agent clients retain TLS trust settings when retargeted to a Platform.
Backup execution receives path restrictions, log bounds and timeouts; durable
leases cover the operation plus configured padding, without retaining a database
connection during external work. Unset identity keys are generated atomically and
persisted in the data volume. Offline recovery must use the original keys.

## Ownership and dependency direction

Organize by feature-oriented bounded contexts, not workspace-wide domain/application
layers. A feature owns its business models, invariants, use cases, typed errors and the
ports it consumes. Adapters implement those ports. Server is the inbound adapter and
composition root. The intended dependency graph is:

```text
server ───────────> feature crates ───> tiny primitives
  └──> adapters ──> feature ports
          └───────> generated external protocol clients
```

Features must not depend on server, adapters, Axum, HTTP presentation, or generated
Docker models. A cross-feature edge requires its consumed contract, rationale and
owner in the dependency inventory; it must not create a cycle. Prefer a
consumer-owned port or an existing coordinator for workflows spanning features.
Do not create common/shared junk drawers, a generic service locator, or an authorization
framework. `citadel-domain` has been removed: actor/permission/redaction primitives,
audit models, licensing models and feature vocabulary now have explicit owners.
`citadel-application` has been dissolved: Activities and Licensing own their services
and ports, Server owns their HTTP presentation, and `citadel-runtime` owns the
process-lifecycle utilities. Neither umbrella crate has a compatibility replacement.

Server builds explicit typed components, handing narrow state to routes and separate
handles to jobs. Configuration is read and validated at startup, then passed as typed
values. Constructors should not discover hidden environment configuration. Preserve
existing environment names/defaults, secret redaction and external contracts.

## Resource vocabulary

| Slot | Responsibility |
|---|---|
| `model` | Entities, value objects, invariants and business state |
| `repository` | Durable persistence ports and cohesive transactional operations |
| `service` | Use-case orchestration using ports |
| `commands` | Optional transport-neutral mutation inputs |
| `read_models` | Optional transport-neutral read results and supporting filters |
| `runtime` | Optional Docker/agent/external execution ports |
| `validation` | Reusable business validation too large for model/service |
| `jobs/`, `adoption`, `webhooks` | Optional cohesive background/adoption/webhook use cases |

Start with role files such as `model.rs`, `repository.rs`, `service.rs`. Expand a large
slot into `service/mod.rs`, `service/apply.rs`, etc., preserving the conceptual entry
point. Use ordinary modules rather than avoidable `#[path]` indirection. Resource
namespaces are plural; role filenames describe responsibilities (`model`, `read_models`);
Rust modules use snake_case and
Cargo package directories use kebab-case (`swarm-services`). Avoid redundant names
such as `postgres/deployments/deployment_repository.rs`.

`lib.rs` and resource `mod.rs` files are façades: crate docs/attributes, module
declarations and deliberate selective re-exports. Implementations live below them.
Small crate-wide primitives require a documented natural ownership reason; there
is no file-length target and no permission to turn a root into a service collection.
Do not add empty per-resource services or split an atomic port just to fill slots.

## Models, projections and presentation

`*View` exclusively means HTTP/realtime presentation owned by server. Persisted
resources use business names (`Deployment`, `BuildRun`). Query results may use
`Summary`, `Details`, `Config`; preparation objects use `Draft`/`Preview`; operation
outputs use `Result`/`Outcome`; immutable workflow claims use `Claim`; runtime
observations use `Snapshot`/`State`/`Observation`. Classify semantics instead of
mechanically deleting suffixes. Efficient read projections are welcome.

Server owns HTTP request DTOs, response wrappers, schema metadata, capabilities,
status/error mapping, and realtime wire serialization. Convert HTTP requests to
feature commands. Map returned models/projections to server Views using `From` for
context-free mappings and named functions when permissions or other context is
required. Persistence never returns an API View or computes presentation capabilities.
Keep wire field names, nullability, redaction and OpenAPI component names stable even
when internal names change.

## Repository, Store and Reader

Use `Repository` for durable business-resource persistence. Keep authorization scope,
row versions, claims, idempotency and atomic multi-resource writes within the operation
that requires them. Do not move transaction guarantees to several unrelated calls.

Use `Store` for actual samples, caches, projections, transient security state or blobs:
`ContainerStatsStore`, `InventoryProjectionStore`, `MfaStore` and `OidcStateStore` are
legitimate. Use `Reader`/`Query` for lookup, history or read projections; do not hydrate
an aggregate merely to return a summary.

`GitRepository` and `BackupRepository` are business entities. Their persistence ports
are `GitRepositoryPersistence` and the shared `BackupPersistence`, respectively;
never `GitRepositoryRepository` or a second conflicting `BackupRepository` trait.
Shared Build/Backup/Automation ports may span resources where transactions require it.
Shared ports must make their transaction boundaries explicit.

`read_models.rs` owns transport-neutral read results: enriched resource details,
configuration snapshots, duplicate/adoption drafts, and supporting filters. These
are application data types consumed by services and mapped to HTTP views by the
server. Read operation implementations live in `service/read.rs`; PostgreSQL SQL
queries live in `adapters/postgres/<resource>/queries.rs`. Use `read_models` for
feature data types, when adding or refactoring resources.

Adoption resolves runtime ownership labels against the current database. An
unassigned standalone container may be adopted when its labeled Deployment no
longer exists; reserved labels are omitted from the draft and reported as a
warning. Compose and Swarm Stack imports require either all unmanaged workloads
or one consistent missing Stack owner. Existing owners, malformed or mixed
ownership, system workloads, and active operations remain protected. Ownership
and membership are rechecked when committing the database links; adoption does
not relabel, restart, or recreate Docker workloads.

## Implemented reference resource: Deployment

```text
src/features/deployments/src/
  lib.rs                         # selective exports
  model/{mod,resource,spec,operations}.rs
  repository.rs                  # DeploymentRepository, atomic durable operations
  service/{mod,read,mutations,apply,delete,updates,adoption,bindings}.rs
  commands.rs                    # transport-neutral mutation inputs
  read_models.rs                 # Config, Draft, filters
  permissions.rs                 # named operation requirements
  runtime.rs                     # DeploymentRuntime and consumed runtime ports
  tasks.rs                       # DeploymentTaskSpawner, consumer-owned port
  adoption.rs                    # semantic adoption previews and port
src/infrastructure/adapters/src/
  persistence/postgres/deployments/
    mod.rs                       # selective exports
    repository.rs                # PostgresDeploymentRepository, trait delegation
    queries.rs                   # ACL-filtered, batch-enriched reads
    rows.rs                      # SQL decoding into business/read models
    authorization.rs             # typed grants, transactional policy checks
    mutations.rs                 # create/config/metadata/rename transactions
    claims.rs                    # Apply/delete claim and completion transactions
    activity.rs                  # persisted audit snapshots
    adoption.rs                  # atomic adoption and preview adapter
    bindings.rs                  # environment/secret resolution
    updates.rs                   # update-check claims
  deployment_runtime.rs          # established Docker/Agent runtime router
src/server/src/
  api/routes/deployments.rs       # HTTP extraction, policies, use-case invocation
  api/resources/deployments/
    mod.rs                       # module declarations
    requests.rs                  # request DTOs and command conversions
    views.rs                     # DeploymentView, response/progress mappings
    spec.rs                      # wire value objects and OpenAPI schema ownership
    capabilities.rs              # typed effective grants to public capabilities
    adoption_views.rs            # adoption presentation
  tasks/deployments.rs            # adapter to the process DynamicTasks owner
  realtime/notifiers.rs          # resource notifier port implementations
```

Shared domain contracts live with their natural owner:

- Identity shares `ResourceAccessInput` and `ResourceAccessDetails` across users and
  teams, including assignment validation. Server exposes one matching request/view
  pair, and Postgres shares access projection and permission-mask helpers. Mutation
  transactions and authorization-cache invalidation stay with each resource owner.
- `citadel-primitives::AuditMetadata` groups creation time and typed `ActorId` for
  managed resources. `ResourceControlState` represents Idle/Queued/Processing; SQL
  adapters validate stored strings before constructing a domain model.
- `citadel-primitives::AutoUpdateState`, `AutoUpdateStatus`, and `UpdateBehavior` are shared by
  Deployments and Swarm Services, including their HTTP schemas. Stored status
  strings are validated at the SQL boundary. Digest comparison ignores repository
  prefixes and digest casing, and is also used by Stack image checks. Stack
  per-service image state and Git commit state remain Stack-owned.
- Git repository/reference, Build run/pool validation, Automation run, and Backup
  repository/run/item/restore/coverage statuses are feature-owned enums reused by
  the HTTP API. `status_enum!` shares exact text parsing and formatting, not a
  universal status enum. SQL readers reject unknown states; executor results and
  lifecycle decisions use variants. Audit snapshots retain their historical text
  payloads to avoid dependencies from Activities back into execution features.
- Deployments own `DeploymentStatus`, reused by container summaries through a
  narrow Platforms → Deployments vocabulary dependency. Stack release status
  uses the same fallible text parser; reconciliation status/action enums are reused
  directly by HTTP views. Swarm owns health, synchronization and persisted operation
  kind/state enums. Its apply-operation kind deliberately excludes deletion, which
  has a separate claim. Event reconciliation and batch completion derive typed
  Deployment/Stack states before persistence. Docker progress and runtime strings
  remain external observations, not managed-resource lifecycle enums.
- `PatchField<T>` handles missing/null/value metadata updates. `FieldUpdate<T>`
  makes nullability explicit in `T` for configuration patches. `merge_json` owns
  recursive JSON merge semantics. Nested webhook patches preserve omitted values.
- `citadel-primitives::schedule::CronSchedule` validates and evaluates the same
  five-field syntax for backups and automation, including time zones.
- `citadel-primitives::WebhookConfig` and `WebhookPatch` define common webhook
  fields, provider/authentication enums, validation and audit redaction. Git,
  Builds, Backups, Automation and Swarm Services use them directly; Stacks flatten
  the shared configuration beside `forceDeploy`. JSON is decoded at persistence
  and merge-patch boundaries, not repeatedly during webhook dispatch. Authentication
  and event filtering remain in the shared Git webhook engine.
- Backups carry `BackupRepositorySpec` and `BackupSourceSpec` through configuration,
  domain records, immutable run snapshots, authorization, source planning and Restic
  execution. JSON conversion stays at persistence and merge-patch boundaries;
  source permission targets and lease/audit keys come from the same typed spec.
- `citadel-activities::ActivitySummary` owns the typed latest-activity envelope.
  Event payload projection still belongs to the server.
- Deployment, Stack and Swarm specifications are reused by API contracts where
  their fields and visibility match. API-specific projections remain separate.
- `citadel-primitives::normalization` shares optional-text trimming and stable ID
  deduplication. Resource-specific length limits, allowed characters, sorting and
  nil-ID validation remain with each feature.
- `citadel-primitives::json_keys` shares first-letter property casing for Stack and
  Swarm storage and server activity presentation. Callers explicitly identify opaque
  dictionary objects; their keys and contents are never rewritten. Record arrays
  are still traversed. Deployment keeps its typed storage representation.
- Server `api/resources/capabilities::ResourceCapabilitiesView` is the common
  read/write/execute DTO for collections and profile capabilities. Collection grant
  projection is shared; resource-specific capabilities and permission policies
  retain their feature semantics. OpenAPI annotations remain server-owned.

These are composed values, not a generic entity base class or marker hierarchy.
Tags remain owned by `citadel-tags`; permissions remain owned by primitives.
Repository lease completion must check the operation token under the same resource
row lock used by acquisition before updating repository readiness or validation.

`Deployment` contains its state, specification, tags, latest activity and related
platform/image/container summaries. ACL-aware repository queries return
`AuthorizedResource<Deployment>`: a resource plus the requesting actor's effective
permission. Permission metadata stays outside the domain model and is mapped to
HTTP capabilities by the server. Authorization and enrichment remain in the same
SQL query.

Requests become business commands; repository and service methods return business
models or semantic projections. Feature crates own domain serialization but do not
depend on OpenAPI. Server-owned DTOs and schema descriptions in
`api/resources/schema_models` own `ToSchema` and schema attributes.
Feature crates remain independent of HTTP handlers and response envelopes. Explicit
conversions preserve wire defaults and build-image provenance while feature-owned
storage conversion preserves the existing PascalCase persisted specification. No
schema migration is required. HTTP and realtime use the same server View conversion.
The binary's `router.rs` assembles resource routers; the library's `api` namespace owns
Deployment's inbound adapter.

Realtime container claims and completions publish one invalidation per Platform,
with all affected Docker IDs. Committed Docker observations are coalesced by
Platform and action over a fixed 100 ms window, bounded to 1,024 pending IDs.
Each invalidation shares one persisted container projection across readers;
authorization is checked separately for every subscriber. This data is retained
only for delivery of that invalidation, with no TTL or long-lived resource cache.
Platform summaries update only the affected row. Deployment and Stack detail
notifications match resource IDs, and unrelated container changes do not reload
workload lists. Missing container rows still trigger authoritative workload reads
so deletions cannot hide an unhealthy workload. Logs and audit events are not
coalesced; reconnects continue to fetch complete authorized snapshots.

Container HTTP selections retain their batch through the mutation runtime. Targets
are grouped by Platform and owning node, and regular Agent/Edge state commands
send all IDs for that daemon in one RPC. Core verifies each target's observed state
before finishing the claim; partial remote failures retain the claim for read-only
recovery. Database ID resolution uses one query for UUID selections.

Apply uses `ApplyDeployment` (Read + Apply) in both the inbound check and the atomic
claim transaction. Write/delete/update operations likewise reuse named requirements.
Capabilities are presentation only; authoritative authorization remains in persistence.
Non-admin get/list retain two/three queries, respectively; admin get/list retain one.
Lists do not issue per-resource permission or enrichment queries.

Apply/delete/update-check operations submit work through `DeploymentTaskSpawner` to
the existing process-owned `DynamicTasks`. Dropping an HTTP request/progress receiver
does not abandon accepted work. Shutdown closes admission, signals cancellation and
drains the owner. Rejected admission releases unexecuted claims; uncertain runtime
outcomes retain the established stale-claim recovery semantics. Operation errors reach
the owner for logging even when the requesting client has disconnected. Feature code
does not construct a runtime or directly spawn these tasks.

Inventory commits container observations before reconciling Deployment status. Status
reconciliation locks one Deployment at a time, then reads the committed observations;
it never holds container locks. The existing health sweep retries skipped or failed
reconciliation, including interruption between the two commits. Inventory and Apply
use `FOR NO KEY UPDATE` where keys are retained so foreign-key checks remain compatible.
Only actual Deployment deletion excludes inventory with a platform lock before removing
referenced rows; normal Apply and configuration writes do not take that platform gate.

After Docker succeeds, Apply retries only completion persistence (three attempts),
retaining the original runtime result, image digest and binding snapshots. Repeating an
identical completion for the same version is idempotent and cannot duplicate its audit
event. Exhausted persistence attempts leave the claim for the existing recovery worker;
they do not record a Docker failure or recreate the container. An unavailable runtime
during recovery leaves the claim pending for another observation.

The established runtime router remains in place; moving unrelated Docker/Agent
adapters has separate ownership. Boundary tests enforce the Deployment
feature/persistence separation without a legacy exemption.

## Implemented multi-resource anatomy: Builds

Build projects default to `pushToRegistry: true`. Disabling it clears the registry
selection and builds a local `citadel/build-<project-id>:<tag>` image on the chosen
runner. Runs retain this choice in their immutable registry snapshot (`null` for
local-only output); local, direct Agent and Edge execution skip registry credentials
and push calls. Local-only results do not enter deployment completion queues or
published-image resolution. They remain on the builder and may be removed when an
ephemeral builder is cleaned up.

```text
src/features/builds/src/
  lib.rs
  projects/{mod,model,commands,patch}.rs
  runs/{mod,model,logs}.rs
  agent_pools/{mod,model,commands,patch,runtime}.rs
  repository.rs                  # BuildRepository: shared atomic persistence
  service/{mod,execution,pool_tests,pool_health,webhooks}.rs
  runtime.rs                     # execution, secrets and entitlement ports
  tasks.rs                       # process-owned pool-test admission port
  permissions.rs                 # named operation requirements
  jobs/completion.rs             # durable Build completion consumers
src/infrastructure/adapters/src/
  persistence/postgres/builds/
    mod.rs
    repository.rs                # trait delegation
    projects.rs, runs.rs, agent_pools.rs
    rows.rs, activity.rs, recovery.rs, completion.rs
  external/builds/executor.rs    # concrete build execution
  connectors/agent/build_pool_checker.rs # Agent capability checks
src/server/src/
  api/routes/builds.rs
  api/resources/builds/{mod,requests,views,spec,capabilities}.rs
  tasks/builds.rs
```

Each resource retains the same anatomy. Add a resource-local repository/service
only when there is an independent semantic responsibility. Here, project/run/pool
claim relationships justify a shared repository and service; three separate ports
would not improve the transaction boundary. A request becomes a neutral command,
the service validates and persists/claims work through `BuildRepository`, the job
executes through runtime ports, and server maps results and events to Views.
Backups, Git, Automation, Alerts and Identity follow this recursive resource convention.
Identity should not remain a separate miniature layered architecture without a
specific documented exception.

## Adapter and server boundaries

Adapters group by responsibility, then technology and resource:

```text
infrastructure/adapters/src/
  lib.rs                         # only group declarations
  persistence/postgres/          # repositories, state stores, readers and SQL
    identity/{users,teams,roles,service_accounts,actors,profile}/
    platforms/{containers,inventory,statistics,node_agents,edge,...}/
    deployments/, stacks/, builds/, backups/, activities/, licensing/, ...
  connectors/
    docker/, agent/, edge/       # protocol clients and semantic transport adapters
    registries/, oidc/          # external HTTP protocols
    containers/, swarm/         # transport mapping and stream helpers
    routing/                    # concrete local/Agent/Edge runtime selection
  external/
    builds/                     # concrete build tool execution
    backups/                    # Restic and pg_dump/pg_restore recovery bundles
    alerts/                     # Shoutrrr delivery
  filesystem/                   # workspace materialization and host disk access
  security/                     # cryptography, signing and credential verification
```

Within persistence, use `repository`, `reader`, `store`, `rows`, `queries` and other
cohesive operation files. Concrete identity implementations of aggregate repository
ports are named `PostgresUserRepository`, `PostgresTeamRepository`, etc. Keep `Store`
for authentication/MFA/OIDC state, samples, projections and other genuine stores.
Shared transactions retain their existing atomic boundaries. SQL files live beside
their consumer; the root `adapters/queries` directory has been removed. Query text and
SQLx offline metadata remain unchanged.

License signature verification and MFA cryptography are separate from PostgreSQL
storage. Build/Backup PostgreSQL credential resolvers and platform registration
persistence are separate from execution/transport implementations. Edge persistence
lives with Platforms, independently of the Edge protocol connector. Constructors and
consumers import the concrete owners directly; there are no old root compatibility
modules, and child modules use ordinary Rust paths instead of `#[path]` attributes.

System backup/recovery remains active: `external/backups/system_recovery.rs` owns
`PgDumpSystemBackupBuilder`, validated recovery bundles and pg_restore execution,
including the existing recovery and parity tests. Its former name/location was not
evidence of dead code. No public API, database or backup format change is intended.

Existing runtime routers and external executors still perform some PostgreSQL-backed
target selection; this move does not claim complete separation of all SQL from
transport orchestration or introduce new feature ports just to rearrange files.

Resource endpoints follow two parallel namespaces. `api/routes/<resource>.rs`
owns Axum handlers, authorization, resource error mapping, route registration and
narrow HTTP state. Keep each resource family in one named file, including adoption,
reads and mutations, so its complete endpoint surface is visible in one place. Do not
create resource subdirectories or a redundant `handlers` level under `routes/`. `api/resources/<resource>/` owns `requests.rs`,
`views.rs` and, when useful, `spec.rs`, `capabilities.rs` and presentation helpers.
Requests include query/path inputs and conversions to feature commands; views include
HTTP/realtime response types and semantic-to-wire mappings. Resource modules must not
import routes, Axum, persistence or task supervision. Keep shared wire vocabulary,
metadata patches and capability types directly under `api/resources/`; shared HTTP
error adaptation stays in `api/error.rs`. Identity and Git use named resource owners
such as `users`, `service_accounts`, `git_accounts` and `git_repositories` in both
namespaces. Do not combine unrelated identity DTOs in a shared `dto.rs` barrel.

Only create files for responsibilities that exist. Process task-port implementations
belong to `server/src/tasks/`; notifier-port implementations belong to
`server/src/realtime/notifiers.rs`. `api/endpoint_catalog.rs` owns the shared route-factory
inventory and endpoint metadata used by OpenAPI and Automation, leaving `api/routes/`
for the actual HTTP endpoints.

The `citadel-docker-api` crate is generated external protocol code, not a
Citadel domain model library. It must not depend on Citadel features or contain
business behavior. `adapters::connectors::docker` translates feature models/ports to generated
DTOs and back, and owns transport, daemon compatibility, streaming and semantic error
translation. Do not expose generated Docker DTOs through feature ports or API Views.
See `src/infrastructure/docker-api/README.md` for
generation pins, compatibility patches and streaming boundaries.

## Error and runtime ownership

Use feature-owned typed errors (`thiserror`) where callers make semantic decisions.
Adapters translate storage/runtime failures while preserving diagnostic source chains.
Server maps feature errors directly to sanitized ProblemDetails/statuses; do not
route unrelated errors through `IdentityError` or a workspace-wide error enum.
`anyhow` is for contextual outer orchestration/bootstrap/job failures, not feature
ports or HTTP contracts. Existing release panic policy is unchanged.

Runtime/task creation has a named owner, cancellation policy, shutdown drain and
recovery behavior. Preserve the difference between a disconnect-cancelled interactive
operation and a durable claimed operation.

## Refactoring discipline

Keep changes focused on the owning resource. Preserve API schemas and persisted formats through moves;
run workspace tests and OpenAPI verification. Authorization changes require route
and persistence regression tests.

Avoid mass renames, shared scaffolding and performance changes during authorization
work. Keep diagnostics and raw measurement output outside the source changes.

## Workload resources

Stacks and Swarm Services now follow the Deployment reference. Each feature owns
`model/`, `commands.rs`, `read_models.rs`, `repository.rs`, `runtime.rs`, `permissions.rs`,
`tasks.rs`, and operation-specific modules under `service/`. Crate façades export
specific contracts. Each resource has one domain model: `Deployment`, `Stack`,
`StackRelease`, `SwarmService`, `GitRepository`, `BuildProject` and `BackupPolicy`.
Tags, latest activity/run summaries and related-resource fields live on those models,
not separate per-resource `Details` wrappers. ACL-aware queries carry the caller's
permission separately in `AuthorizedResource<T>`; business helpers accept `&T`.

Full resource reads populate related fields with joined or batched queries. Execution
and scheduling reads may omit UI enrichment; their results must not be used as full
API responses or to replace tag links. Backup claims use `get_policy_for_execution`
and skip tag/run summary queries. Backup policy mutations return populated summaries,
including after metadata changes. HTTP schemas remain server-owned.

`adapters/src/persistence/postgres/{stacks,swarm_services}/` owns repositories, SQL projections,
row decoding, transactional authorization, bindings and durable claim transitions.
List/get queries retain SQL ACL filtering and batch tags/activity/operation metadata.
Permissions decode into `EffectivePermission`; administrator access is explicit.
No repository constructs HTTP capabilities or public activity envelopes.

`server/src/api/resources/{stacks,swarm_services}/` owns request DTOs, views, capabilities,
OpenAPI schemas and conversions, including progress, adoption and duplicate drafts.
HTTP and realtime use the same conversions. Public `ManagedSwarmServiceView` naming
remains at that boundary. Persisted spec serialization remains in the features;
server value objects preserve wire defaults, aliases and schema names explicitly.
`StackDrift` serialization also serves durable alert payloads and fingerprints.

`StackTaskSpawner` and `SwarmServiceTaskSpawner` admit durable work to the existing
process `DynamicTasks`. Stack Apply/rollback owns an already-acquired claim and
releases it if admission closes; Swarm mutations and update checks acquire claims
inside accepted work. Request drops do not drop accepted work. Shutdown drains the
same process tracker used by Deployments; persisted claims and reconciliation remain
the crash-recovery authority. Operation failures reach that tracker after redaction.

Stack runtime, source-materialization, build-image resolution and update scanner
adapters use consumer-owned ports. Automation, Alerts, Platforms and Identity
follow the resource organization described below.


## Multi-resource contexts

Builds uses one `BuildRepository` for project/pool/run claims and completion. Backups
uses one `BackupPersistence` for repositories, policies, backup items and restores.
Resource models and commands live in their plural namespaces; cross-resource
orchestration is split by operation under `service/`. Persisted specifications keep
Serde where JSON storage or merge-patch processing requires it. Business resource
models do not serialize HTTP responses.

Git owns Accounts and Repositories separately. `GitAccountRepository` stores protected
credentials; `GitRepositoryPersistence` owns catalogue CRUD and
`GitRepositoryExecutionPersistence` owns synchronization claims. `GitRepository` is
the full business resource; `GitRepositorySource`, snapshots, directory listings and
commit comparisons remain semantic projections. Execution is divided into synchronization,
materialization, browsing, discovery and credential preparation modules.

PostgreSQL implementations live under `adapters/postgres/{builds,git,backups}`. Runtime
executors, planners and external transport adapters retain their established locations.
Server owns all migrated request/response/schema types under `api/resources/{builds,git_accounts,git_repositories,backups}`;
HTTP and realtime use explicit conversions. Git activity presentation is mapped only
in server. Build list capabilities use one batched, typed permission lookup; migrated
SQL readers bind accepted ordinal permission levels instead of bit masks.

Pool-test tasks acquire claims only after admission to the process `DynamicTasks`.
Dropping the caller leaves accepted work owned by that tracker. Root shutdown cancels
the check, persists its failed validation result, releases the claim, and drains the
owner before database teardown. Abnormal process failure still uses durable recovery.

Git owns catalogue persistence and webhook evaluation directly. Tags owns shared tag
projections and transactional tag-link helpers; each catalogue adapter owns its audit
mapping. The former Resources compatibility delegates and schema facade have been
removed. APIs own their wire schemas in Server. Architecture guards
continue to apply; the ownership correction adds no exemptions.

The cross-feature contracts are:

| Importing feature → owner | Consumed contract | Reason |
|---|---|---|
| Builds → Alerts | `AlertEventSink`, `AlertObservation` | Publish execution outcomes through the observation contract owned by Alerts. |
| Builds → Tags | `TagSummary` | Tags owns the shared tag projection. |
| Builds → Git | webhook validation and evaluation | Git owns webhook semantics directly. |
| Git → Execution | bounded process requests, results and `ProcessRunner` | Git receives the concrete Infrastructure runner through its process port. |

These are explicit consumed contracts, not exceptions permitting feature-owned HTTP Views.

## Automation and Alerts contexts

Automation owns `actions/` and `runs/`; Alerts owns `channels/`, `rules/`, and
`events/`. Their roots only declare modules and selectively export contracts.
Configuration and patch types keep Serde where JSON interpretation is part of their
semantics. Resource entities, read results, and Automation progress carry semantic
data without HTTP serialization or schema derives. Quiet-hours, time-zone, rule
metadata, run-log redaction, and cron helpers retain named semantic modules.

`AutomationRepository` keeps enqueue/claim/finish and action ownership atomic.
`AlertRepository` keeps incident deduplication, rule state, delivery outbox claims,
and retries under the existing transaction boundaries. PostgreSQL implementations
live under `adapters/src/persistence/postgres/{automation,alerts}`. Shoutrrr delivery is a separate
runtime adapter in `alert_delivery.rs`.

Server owns requests, views, schemas and explicit conversions under
`api/resources/{automation,alerts}`. Realtime uses those same response conversions. Automation
capabilities use typed permissions and explicit administrator access. Both migrated
PostgreSQL readers bind accepted ordinal permission values; invalid bit-mask values
do not grant access.

Direct Automation runs enter `DynamicTasks` through `AutomationTaskSpawner` before
acquiring a durable claim. The task owns claim completion, uses a child of the root
shutdown token, and cancels when its progress receiver disconnects. Rejected admission
creates no claim. Shutdown drains completion while persistence is available. Scheduled
and worker runs remain independent of viewers and retain durable recovery.

| Importing feature → owner | Consumed contract | Reason |
|---|---|---|
| Automation → Alerts | `AlertEventSink`, `AlertObservation` | Report failed execution through the semantic observation contract. |
| Automation → Git | `RepoWebhookConfig` | Share validated webhook configuration without sharing API schemas. |
| Automation → Tags | `TagSummary` | Tags owns the shared tag summary projection; server maps it to the wire DTO. |
| Automation → Execution | `ProcessRunner` and bounded process contracts | Infrastructure executes Deno with the existing sandbox, cancellation and output limits. |

No new architecture exemptions are introduced for either feature. Their architecture
guards cover resource ownership, HTTP separation, persistence projection and detached tasks.

## Platforms, Identity and shared resource management

Platforms owns the closed `ConnectorKind` (Local, Agent, EdgeAgent) and
`PlatformKind` (Docker, DockerSwarm) classifications used by runtime collection.
PostgreSQL decodes persisted spellings before workers receive a target. Both
historical standalone descriptor spellings, `Docker` and `DockerStandalone`, denote
Docker; unknown classifications are errors. Open external Docker status strings
remain observations rather than being treated as closed Citadel classifications.
Generated Docker models remain confined to adapters. Server owns platform Views,
capability flags, Swarm overview messages and realtime JSON mapping.

Identity follows the same resource roles as Deployments, Stacks and Git. Its
`users`, `teams`, `roles`, `service_accounts`, `actors`, `profile`, `authentication`,
`mfa` and `oidc` modules live directly under `src/`, with `model`, `commands`,
`read_models`, `repository` and `service` slots as needed. There is no local
`domain/` or `application/` layer and no structural security exception. Shared
Identity authorization evaluation lives in `permissions.rs`; authentication
invariants and transaction boundaries stay with their existing services and ports.
Challenge consumption, session issuance, recovery-code consumption, account linking
and last-administrator checks remain atomic. Security behavior does not depend on
layer-shaped folder names.

Feature roles may expand from `model.rs` or `service.rs` into a directory with that
same name. Single-resource crates use those roles directly under `src/`; crates
with multiple resources repeat them under the resource namespace. A service slot's
`mod.rs` may define its service state and constructor shared by its operation modules;
crate/resource façades do not contain use-case implementations. Do not add empty
roles to projection-only or value-only resources.

HTTP DTOs and schema descriptors remain server-owned. Identity and lookup vocabulary
has semantic enum ownership in the feature, with server-side OpenAPI descriptors
preserving the existing schema names and enum values. Serde on patch/configuration
values preserves null/missing-field and persisted JSON behavior.

Platform connectivity uses the shared `primitives::PlatformStatus` in platform and
workload read models; Docker container, node and task observations retain their
external state vocabulary. Alerts owns `AlertRuleStatus`, `AlertEventStatus`, `AlertSeverity`, and the known
`AlertType` vocabulary. Internal observation names remain strings to support
custom observations and transient channel verification; HTTP input uses the known
alert kinds. Historical activity snapshots retain their original labels.
Adapters parse stored status text at the persistence boundary, while server-only
schema descriptors document these native enums without adding OpenAPI dependencies
to feature crates.

Read projections use Reader ports; durable resource mutations use Repository ports.
Sample/projection stores and transient MFA/OIDC security state retain Store where
that describes their actual responsibility. Registries, tags, bindings and secret
metadata have explicit owners described below; repository source configuration and
webhook configuration belong to Git. Encryption remains behind secret-protection ports;
HTTP mappings preserve credential redaction and never decrypt to construct a View.

Creation attribution in managed domain models uses `AuditMetadata`, including
alert rules, alert channels, Git accounts, and registries. It contains only creation
time and creator identity. Git account and registry updates preserve that attribution. Modification
times, execution actors, historical events and flattened read projections retain
their own semantics; HTTP views expose the existing flat creation fields.

### Explicit ownership of shared resource management

`citadel-resources` has been removed. It combined unrelated features behind one
repository and a misleading service name. `citadel-primitives` remains the small
actor/permission/redaction dependency; it does not own resource repositories or services.

| Crate | Responsibility | Mutation/read port |
|---|---|---|
| `citadel-tags` | Tags and resource tag assignments | `TagRepository` |
| `citadel-registries` | Registry configuration and image browsing | `RegistryRepository`, image browsing read models |
| `citadel-bindings` | Variables, secret bindings, definitions and providers | `BindingRepository`, `SecretService`, `SecretProtector` |
| `citadel-discovery` | Permission-filtered lookup and global search | `LookupReader`, `GlobalSearchReader` |

Bindings retains secret definitions/providers because reference validation, orphan
cleanup and binding mutations share transaction boundaries. Registries depends on
Tags for `TagSummary`; Tags, Bindings and Discovery do not depend on Git or each
other. Webhook consumers use Git's contracts directly. The small three-state patch
values are owner-local, with explicit conversions from the server's HTTP patch type.

PostgreSQL implementations live under `adapters/src/persistence/postgres/{tags,registries,bindings}`.
Tag-link SQL is reusable within an existing Registry or Git transaction, with typed
error conversion at that adapter boundary. Audit creation, tag replacement, row
locking and commit order remain in the original transaction. Shared access SQL
lives in the PostgreSQL authorization helper, not a feature-level umbrella port.
Platform description persistence is owned by `PlatformMetadataRepository`.

Server DTOs live under `api/resources/{tags,registries,bindings,search,lookup}`. Tag, Registry and
Binding handlers live beside their DTOs; lookup/search retain their existing HTTP
modules. Git catalog handlers retain their Git namespace and their own HTTP state. Route
composition supplies only each owner's repository/service plus identity and realtime.
HTTP filters, metadata patch parsing and access helpers are server-local. Feature
errors are typed by owner; HTTP ProblemDetails mapping preserves existing behavior.
The broader HTTP error-boundary cleanup remains part of the later refactor.

### Feature ownership of shared services

Shared services and vocabulary have explicit feature owners:

| Former domain content | Implemented owner/path |
|---|---|
| `ActorId` | `primitives/src/actor.rs` |
| Permission levels, resource vocabulary, specific masks and policy traits | `primitives/src/{permissions,authorization}.rs` |
| Environment-name redaction predicate | `primitives/src/redaction.rs` |
| Setup, actor/principal/role, MFA and preference enums | Identity resource `model.rs` modules |
| Lookup vocabulary | `discovery/src/lookup/model.rs` |
| Swarm ownership classification | `swarm-services/src/model/ownership.rs` |
| License identity/state/payload/capability vocabulary | `licensing/src/model/{resource,vocabulary}.rs` |
| Audit envelope, event information and invariants | `activities/src/model/{event,info,vocabulary}.rs` |
| Audit snapshots | Activities `model/` modules grouped by event family (Identity, Git, Builds, Stacks, etc.) |
| Change fields, source and webhook metadata | Activities `model/{changes,sources,webhooks}.rs` |

Activities owns historical audit snapshots rather than live feature aggregates. It
uses serialized strings for role/setup/preference audit values to avoid depending
on Identity, which emits events. Boundary mappings preserve the exact previous JSON
strings. Activities depends only on narrow primitives and Licensing models; Licensing
models depend only on primitives. Neither model crate depends on Identity services.
The complete consumed domain symbol map is in the resource-structure correction report.
There is no compatibility `citadel-domain` package or re-export layer. Platforms
consumes the Swarm Services `SwarmServiceOwnership` classification in its inventory
projection; this narrow dependency is acyclic and keeps that vocabulary with its
feature owner. It does not move Swarm mutation orchestration into Platforms.

The remaining `citadel-application` public items now have the following owners.
Paths below are relative to `crates`. This table accounts for the complete
remaining public surface before deletion; earlier domain/runtime moves are above.

| Former application public items | Final owner/path | Reason |
|---|---|---|
| `ActivityFilter`, `ValidatedActivityFilter`, `ActivityRecord`, `PagedActivityRecords`, `DEFAULT_ACTIVITY_PAGE_SIZE`, `MAXIMUM_ACTIVITY_PAGE_SIZE` | `features/activities/src/read_models.rs` | Activity read inputs, results and paging invariants |
| `ActivityQueryStore`, `WebhookActivitySink` | `features/activities/src/repository.rs` | Consumer-owned activity persistence ports |
| `ActivityService` | `features/activities/src/service.rs` | Activity use cases |
| `public_activity_info`, `public_latest_activity` | `server/src/api/resources/activities/presentation.rs` | HTTP/realtime dictionary-aware camelCase conversion |
| `LicenseSource`, `LicenseTransitionCheck` | `features/licensing/src/read_models.rs` | Neutral persisted source and transition result |
| `LicenseStore`, `LicenseValidationPersistence` | `features/licensing/src/repository.rs` | Atomic installation/removal and validation persistence |
| `LicenseVerifier`, `LicenseStateNotifier` | `features/licensing/src/runtime.rs` | Verification and notification ports |
| `LicenseService` | `features/licensing/src/service.rs` | License lifecycle and optimistic concurrency |
| `LicenseTransitionMonitor`, `license_transition_delay`, `next_license_boundary`, `LICENSE_TRANSITION_MAXIMUM_CHECK_INTERVAL`, `LICENSE_TRANSITION_BOUNDARY_MARGIN` | `features/licensing/src/jobs.rs` | Transition checking and deadline policy |
| `build_state` | `features/licensing/src/state.rs` | Effective license-state calculation |
| `InstallLicenseRequest` | `server/src/api/resources/licensing/requests.rs` | HTTP request validation/schema |
| `LicenseView`, `LicenseCapabilityView`, `LicenseEntitlementsView`, `LicenseRequestView` | `server/src/api/resources/licensing/views.rs` | HTTP response mapping and schemas |
| `LICENSE_REPLACEMENT_MISMATCH`, `LICENSE_REPLACEMENT_NOT_YET_EFFECTIVE` | Exact problem URIs retained in `server/src/api/routes/licensing.rs`; unused public constants removed | HTTP problem classification belongs to Server |
| Previously moved queues, supervisor, polling, last-used tracking, dynamic tasks, metrics, I/O budgets and signals | `infrastructure/runtime/src/` | Process ownership |

Activities accepts an `ActivityAccess` derived from the authenticated principal.
It contains the same ActorId and administrator decision used by the existing SQL,
and cannot be deserialized from HTTP input. Audit actor-kind results remain validated
historical labels; Server maps them to its existing response vocabulary. Activities
therefore does not acquire a reverse dependency on Identity.

Licensing owns `LicenseError`, `LicenseClock`, neutral `LicenseRequest` data, and
`LicenseChange` audit intents. It does not depend on Activities or Identity. The
PostgreSQL licensing adapter maps changes to the existing `ActivityEventInfo` and
`LicenseActivitySnapshot` payloads inside the original mutation transaction. CAS
fingerprints, validation transitions, retries, notifications and audit JSON remain
unchanged. Production injects UTC clock functions; tests retain fixed-clock injection.

Activity and Licensing service errors map directly to the existing ProblemDetails
wire shape in Server. Authentication and authorization errors remain Identity-owned.
The private PostgreSQL audit insert helper still serves existing adapters with its
error contract. Request and View types belong to Server.

`TaskSupervisor` retains returned error sources through a boxed `Error + Send + Sync`
and task name, instead of stringifying them. Cancellation and drain ordering are
unchanged; returned errors remain supervised and `panic=abort` remains process-fatal.

`citadel-application` and `citadel-domain` are absent from workspace members and
consumer dependencies. No compatibility crates or new architecture exemptions are
introduced.

The transport guard checks dependency boundaries. The lifecycle rules below
track Container work and the two bounded Identity password blocking boundaries.

Stacks now consumes Platforms' `PlatformKind` in its durable-operation claims and
read projections. This narrow cross-feature dependency prevents Stack
workers from reinterpreting persisted strings; it is acyclic because Platforms
has no dependency on Stacks. PostgreSQL owns the legacy spelling conversion.


### Server organization and startup configuration

The executable constructs `composition::ServerComponents` once and consumes it in
`router.rs`. It is not an Axum state or a clonable service locator. The former
`state.rs` is removed. Explicit construction functions under `composition/` cover
Identity, Licensing, catalogs, connectors, Alerts, Automation, Backups, Builds,
workloads and Platforms. Functions return typed bundles when there are multiple
outputs, or the service directly when there is only one. A private `RuntimeContext`
shares the database pool, Docker/Agent clients, Edge registry, task tracker,
cancellation token and realtime hub during construction only.

`Jobs` owns the runtime target registry, image scanner, alert delivery service and
single-owner worker inputs. HTTP states retain only their route-family handles.
The shared image digest cache is still constructed once for workload services;
this change makes no memory-performance claim. `app.rs` retains supervisor and
dynamic-task draining before PostgreSQL pool closure, with the advisory job lease
held throughout cleanup.

`Config::execution` owns typed path, external-tool, Automation and Edge settings,
plus the Restic/volume-helper image settings. It preserves the existing environment
names, aliases, defaults and Automation log-size clamping. CA bundles are loaded
with the same 1 MiB bound during configuration, before migrations or service
construction. Invalid execution configuration therefore fails earlier. The existing
redacted `EffectiveConfig` JSON shape is unchanged. Constructors no longer read
these settings: Docker receives its host root explicitly and the backup executor
receives its Restic executable explicitly. CLI-specific settings remain at the CLI
boundary. Unattended administrator bootstrap intentionally checks persisted setup
state before reading password-file settings, so removing that file after setup
continues to work.

Remaining endpoint families now follow these paths:

| Former top-level module | Server route owner | Presentation owner |
|---|---|---|
| `activities_http` | `api/routes/activities.rs` | `api/resources/activities/` |
| `license_http` | `api/routes/licensing.rs` | `api/resources/licensing/` |
| `platforms_http` | `api/routes/platforms.rs` | `api/resources/platforms/` |
| `webhooks_http` | `api/routes/webhooks.rs` | `api/resources/webhooks/` |
| `identity_http`, `mfa_http`, `oidc_http` | `api/routes/{authentication,mfa,oidc}.rs` | Matching owner under `api/resources/` |
| `users_http`, `teams_http`, `roles_http`, `service_accounts_http`, `profile_http`, `actors_http` | `api/routes/{users,teams,roles,service_accounts,profile,actors}.rs` | Matching owner under `api/resources/` |
| `search_http`, `lookup_http` | `api/routes/{search,lookup}.rs` | Matching owner under `api/resources/` |

Identity wire types belong to the corresponding resource's requests/views; the few
shared identity envelopes live in `api/resources/common.rs`. Old root module aliases
and `handlers` wrappers are removed.
`application_info_http` and `diagnostics_http` remain cohesive process-level HTTP
modules. Public routes, schema names, request/response types and authorization
behavior are unchanged by these moves.

All workspace packages inherit `unsafe_code = "forbid"`, including generated
protocol packages; no generated-code exemption is needed. The HTTP panic layer is
compiled only for unwind-capable builds. Production `panic=abort` remains fatal;
returned errors, task supervision and bounded shutdown remain the recovery path.

The shared HTTP boundary is `api/error.rs`: resource mappings produce the
Server-local `ApiError`, `HttpError` adds request context, and one ProblemDetails
renderer preserves status codes, field errors, request/trace IDs, authentication
headers and cache headers. Identity errors are one input to this boundary, rather
than a presentation dependency for unrelated features. Internal typed errors retain
their source for logging and remain sanitized in public responses.

OpenAPI dependencies and attributes belong only to Server. Feature-owned value
objects retain their Serde contracts and business behavior. Server schema descriptions
in `api/resources/schema_models` use native field types with schema overrides and
exhaustive typed conversions; enum descriptions derive wire values from native
serialization and exhaustively match variants. Existing descriptors in
`api/resources/vocabulary.rs` cover additional vocabulary. This keeps documentation
concerns out of features without adding serialization conversions to hot paths.
The resource structure tests enforce the dependency boundary across every feature.
The execution crate's process contracts now live in `execution/src/process.rs`;
its `lib.rs` is an explicit façade. Server worker/realtime child modules use normal
module paths, including their unit tests.

`api/endpoint_catalog.rs` shares the route-factory inventory with OpenAPI export. Startup's
Automation catalog and setup policy retain only sorted endpoint metadata. Each
route family still constructs temporary Utoipa metadata, but ordinary catalog
construction no longer merges, renders, parses or retains the full OpenAPI JSON
or compatibility schemas. A regression test compares catalog bytes and setup
policy with the complete document. This makes no measured RSS/CPU claim.

Container mutations use an injected task port backed by the process tracker.
Accepted work continues after HTTP disconnection; shutdown rejects new admission
and cancels runtime work while preserving ambiguous claims for read-only recovery.
Image deletion, image pulling and Node-agent progress operations are also tracked;
streamed operations retain cancellation when the response body is dropped.
`DynamicTaskReservation` registers cleanup before an external resource is acquired.
Volume helper cleanup keeps that registration and its capacity permit until cleanup
finishes, including when the client disconnects during explicit close or cleanup
begins after admission closes. The existing bounded cleanup and orphan reaper
remain authoritative. These tasks drain within the existing shutdown budget before
pool closure; timeout remains a reported shutdown failure, not successful cleanup.

The two Identity password `spawn_blocking` calls are accepted finite CPU boundaries,
not durable process jobs. Their semaphore permits stay inside the blocking closures
and cannot be released early by HTTP cancellation. The closures hold no database,
session or claim handles; Tokio joins started blocking work when its runtime exits.
Existing awaited host-disk sampling and key rotation remain bounded-purpose blocking
boundaries. Realtime writers and worker child tasks retain their connection/worker
owners; CLI signal handling now aborts and joins its local task.

Dependency audits use `cargo tree --locked -e features`, duplicate-version inspection
and the unused-crate-dependency lint. OpenAPI schema derives and HTTP presentation
contracts belong to Server; feature models remain independent of Utoipa. Server's SHA-2,
Bindings' Serde JSON and Runtime's futures utility dependencies are now test-only.
Generated Docker dependencies remain aligned with workspace pins. Axum/Tower HTTP
transport features, Reqwest TLS/JSON/stream/query, Tokio runtime/process/I/O,
SQLx PostgreSQL/macros and Tonic client/server features have concrete consumers;
no speculative default-feature reduction was made. Remaining duplicate major
versions come from upstream protocol/crypto dependencies and host/build feature
separation.

`unused_crate_dependencies` is not globally enabled: checking individual library,
binary and integration-test targets reports dependencies consumed by sibling
targets, including generated/test-only consumers. Suppressing that noise would need
broad exemptions. The workspace retains `unsafe_code = "forbid"` and strict Clippy.
### API routes and resource presentation layout (2026-09-20)

The user-approved layout correction separates endpoint implementation from its HTTP
contracts throughout Server, using the convention above. All consumers, composition,
realtime mapping, endpoint metadata and architecture guards use the new owners.
Request/response type names, Serde attributes and explicit OpenAPI component names
are preserved without compatibility module aliases or architecture exemptions.

Validation for this layout: formatting and strict locked workspace/all-targets
Clippy passed; workspace tests passed (682 passed, 239 opt-in tests ignored).
An isolated PostgreSQL instance separately passed 147 acceptance tests, including
all seven bootstrap/process cases and all 62 Platform cases. OpenAPI verification
retained 404 full and 305 public operations with unchanged schemas and frontend
types. No public contract baseline was regenerated to accept a difference.


### Flat route files (2026-09-20)

The user's navigation preference supersedes the earlier allowance for route
subdirectories: every route family has one `api/routes/<resource>.rs` file. Route
registration, handlers and their local HTTP helpers are colocated. Requests, views,
schemas and pure presentation mappings remain under `api/resources/<resource>/`.
Large files, including Platforms, follow the same rule; file length alone does not
justify splitting a route family. The architecture guard rejects route directories.

Validation of the flat route files passed: formatting, strict locked workspace
Clippy, 682 workspace tests (239 opt-in tests skipped), and 147 PostgreSQL
acceptance tests. OpenAPI and frontend type verification retained the unchanged
404 full and 305 public operations. No baseline was updated to accept drift.

## Appearance and design tokens

Frontend appearance is owned by `src/lib/appearance/AppearanceProvider` (module
`appearance-provider.tsx`), separate from navigation layout state. The provider
applies mode plus `data-theme-color`, `data-font`, `data-radius`,
`data-content-layout`, and `data-density` to the document. Shared CSS variables
flow into Tailwind semantic tokens and existing components; features do not branch
on appearance presets. The shared sheet is available from the header and Profile.

Preferences persist through the existing identity profile service and merge-patch
endpoint. Enum values are stable PascalCase API/database values; DOM identifiers
are mapped centrally. Changes are serialized per browser session, optimistic,
and scoped to the authenticated user. In-flight responses from a previous session
cannot update the next user's document. The selected local font loads before
React mounts, and system mode listens to the operating-system media query.

Citadel is unreleased, so appearance columns and their defaults belong in the
canonical schema and generated `0001_initial.sql` baseline. Repository updates
merge under the existing user lock; unchanged values do not update rows or emit
activity. The migration catalog retains a single baseline until the first release.
