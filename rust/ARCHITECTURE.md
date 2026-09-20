# Citadel Rust architecture

This describes the architecture of the incremental Rust refactor. Deployments is the
implemented reference resource as of Phase 6. Stacks and Swarm Services follow it
in Phase 7; Builds, Git, and Backups apply it recursively in Phase 8, followed by
Automation and Alerts in Phase 9, then Platforms, Identity and shared Resources in
Phase 10.

## Workspace groups

The workspace has exactly three directories under `rust/crates`:

```text
crates/
  server/                         # API and executable composition root
  features/
    deployments/, git/, stacks/, ...
    primitives/                   # shared actor/permission/redaction vocabulary
    execution/                    # process requests/results and ProcessRunner port
    application/                  # remaining activity/license service coordination
  infrastructure/
    adapters/                     # existing concrete integration crate
    database/                     # schema, migrations and migration runner
    docker-api/                   # generated Docker protocol client
    contracts/                    # generated Agent protocol definitions
    processes/                    # bounded OS process execution
    runtime/                      # task supervision, queues, metrics and I/O budgets
```

Each existing feature remains a separate Cargo crate. Directory grouping does not
merge the features into one application crate. Existing package names remain stable;
`citadel-processes` and `citadel-runtime` explicitly own the extracted implementations.
Features may depend only on other Features workspace packages. Infrastructure may
depend on Features and other Infrastructure packages, never Server. Server may
depend on both to compose implementations. The architecture test checks actual Cargo
metadata, including build, target-specific and development dependencies.

Git and Automation receive a `ProcessRunner` implementation through constructors.
Git retains its `GitProcessPort` name as an alias for that shared port. Production
composition injects Infrastructure's `SystemProcess`. Process cancellation, output
limits, child termination/reaping and redaction retain their existing behavior.
Real subprocess/Git integration tests live with Infrastructure; feature unit tests
use injected process doubles. Runtime utilities have moved out of application services,
and no Feature depends on the concrete process or hosting runtime crates.

This grouping establishes crate dependency direction. Remaining Phase 11 work includes
distributing the activity/license coordinator and removing its presentation types.
Remaining source-level cleanup includes existing SQL in HTTP handlers and feature
filesystem operations; the folder move does not claim those have been eliminated.
Historical reports retain the paths recorded at their audit dates. Generators,
build inputs, development instructions and active architecture checks use current paths.

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
owning phase in the dependency inventory; it must not create a cycle. Prefer a
consumer-owned port or an existing coordinator for workflows spanning features.
Do not create common/shared junk drawers, a generic service locator, or an authorization
framework. `citadel-domain` has been removed: actor/permission/redaction primitives,
audit models, licensing models and feature vocabulary now have explicit owners.
`citadel-application` remains a transitional activity/license service crate under
Features for the rest of Phase 11. Its process-lifecycle utilities now belong to
`citadel-runtime` under Infrastructure.

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
feature data types, including future resource migrations.

## Implemented reference resource: Deployment

```text
crates/features/deployments/src/
  lib.rs                         # selective exports
  model/{mod,resource,spec,operations}.rs
  repository.rs                  # DeploymentRepository, atomic durable operations
  service/{mod,read,mutations,apply,delete,updates,adoption,bindings}.rs
  commands.rs                    # transport-neutral mutation inputs
  read_models.rs                 # DeploymentDetails, Config, Draft, filters
  permissions.rs                 # named operation requirements
  runtime.rs                     # DeploymentRuntime and consumed runtime ports
  tasks.rs                       # DeploymentTaskSpawner, consumer-owned port
  adoption.rs                    # semantic adoption previews and port
crates/infrastructure/adapters/src/
  postgres/deployments/
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
crates/server/src/api/deployments/
  mod.rs                         # selective exports
  handlers.rs                    # HTTP extraction, policies, use-case invocation
  requests.rs                    # request DTOs and command conversions
  views.rs                       # DeploymentView, response/progress mappings
  spec.rs                        # wire value objects and OpenAPI schema ownership
  capabilities.rs                # typed effective grants to public capabilities
  adoption.rs, adoption_views.rs # adoption HTTP boundary
  tasks.rs                       # adapter to the process DynamicTasks owner
```

`Deployment` contains resource state, specification and row version. `DeploymentDetails`
wraps that entity with platform/image/container/tag/activity enrichment and a typed
`EffectivePermission`. These are query data, not presentation capabilities. The SQL
projection performs ACL filtering and enrichment in one query; the adapter decodes
persisted permission values and never constructs an HTTP View. Only server maps raw
activity snapshots, capabilities, null omission and wire field names.

Requests become business commands; repository and service methods return business
models or semantic projections. The feature has no Utoipa or HTTP dependency. Explicit
conversions preserve wire defaults and build-image provenance while feature-owned
storage conversion preserves the existing PascalCase persisted specification. No
schema migration is required. HTTP and realtime use the same server View conversion.
The binary's `router.rs` assembles resource routers; the library's `api` namespace owns
Deployment's inbound adapter.

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

The established runtime router remains in place; moving unrelated Docker/Agent
adapters is outside this resource migration. Boundary tests enforce the Deployment
feature/persistence separation without a legacy exemption.

## Implemented multi-resource anatomy: Builds

```text
crates/features/builds/src/
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
crates/infrastructure/adapters/src/
  postgres/builds/
    mod.rs
    repository.rs                # trait delegation
    projects.rs, runs.rs, agent_pools.rs
    rows.rs, activity.rs, recovery.rs, completion.rs
  build_executor.rs              # existing runtime implementation
  build_pool_checker.rs          # Agent capability checks
crates/server/src/api/builds/
  mod.rs
  handlers/{mod,projects,runs,agent_pools}.rs
  requests.rs, views.rs, spec.rs, capabilities.rs, tasks.rs
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

Adapters use technology first, resource second: `postgres/<context>/`,
`docker/<context>/`, `agent/`, `filesystem/`, `external/`. Files describe roles inside
those namespaces. Shared PostgreSQL transactions can use one context repository;
real state stores retain their semantic names. SQL rows and raw database permission
encodings stop at the adapter boundary.

Every substantial endpoint family lives in `server/src/api/<resource>/` with
`mod.rs`, `handlers.rs`, `requests.rs`, `views.rs`; optional capability mapping can
remain in views when small. Identity families live under `api/identity/<resource>/`.
Small families may combine request/view code without changing the vocabulary.
Shared capability/error presentation lives in `api/capabilities.rs` and `api/errors.rs`.

The `citadel-docker-api` crate is generated external protocol code, not a
Citadel domain model library. It must not depend on Citadel features or contain
business behavior. `adapters::docker` translates feature models/ports to generated
DTOs and back, and owns transport, daemon compatibility, streaming and semantic error
translation. Do not expose generated Docker DTOs through feature ports or API Views.
Phase 3 generates and tests the crate alongside the existing Docker implementation;
Phase 4 owns production adapter migration. See `crates/infrastructure/docker-api/README.md` for
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
operation and a durable claimed operation. The process lifecycle foundation belongs
to Phase 5; Deployments adopts it in Phase 6. Other resources adopt it in their phases.

## Incremental migration

Keep changes focused on the resource being migrated. Existing legacy layouts do not
justify adding new ones. Preserve API schemas and persisted formats through moves;
run workspace tests and OpenAPI verification. Authorization conventions and parity
are documented in [AUTHORIZATION.md](AUTHORIZATION.md).

Other feature Views and legacy umbrella crates remain until their owning migration phases.
Avoid mass renames, shared scaffolding and performance changes during authorization
work. Keep diagnostics and raw measurement output outside the source changes.

## Workload resources (v13 Phase 7)

Stacks and Swarm Services now follow the Deployment reference. Each feature owns
`model/`, `commands.rs`, `read_models.rs`, `repository.rs`, `runtime.rs`, `permissions.rs`,
`tasks.rs`, and operation-specific modules under `service/`. Crate façades export
specific contracts. `Stack` and `StackRelease` are durable business resources;
`StackDetails` and `StackReleaseDetails` add query enrichment. `SwarmServiceDetails`
wraps `SwarmService` and includes the semantic `SwarmServiceOperation` projection.
Immutable dereferencing supports read access; mutation names the owned resource.

`adapters/src/postgres/{stacks,swarm_services}/` owns repositories, SQL projections,
row decoding, transactional authorization, bindings and durable claim transitions.
List/get queries retain SQL ACL filtering and batch tags/activity/operation metadata.
Permissions decode into `EffectivePermission`; administrator access is explicit.
No repository constructs HTTP capabilities or public activity envelopes.

`server/src/api/{stacks,swarm_services}/` owns request DTOs, views, capabilities,
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
adapters retain their established locations and consumer-owned ports. This phase
moves workload persistence and inbound adapters, not the global runtime tree.
Automation/Alerts now follow the Phase 9 resource organization below. Shared
Platforms/Identity remain scheduled for Phase 10.


## Implemented multi-resource contexts (Phase 8)

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
Server owns all migrated request/response/schema types under `api/{builds,git,backups}`;
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
removed. Migrated APIs own their wire schemas in server. Phase 8's architecture guards
continue to apply; the ownership correction adds no exemptions.

The reviewed Phase 8 cross-feature contracts are:

| Importing feature → owner | Consumed contract | Reason |
|---|---|---|
| Builds → Alerts | `AlertEventSink`, `AlertObservation` | Publish execution outcomes through the observation contract owned by Alerts. |
| Builds → Tags | `TagSummary` | Tags owns the shared tag projection. |
| Builds → Git | webhook validation and evaluation | Git owns webhook semantics directly. |
| Git → Execution | bounded process requests, results and `ProcessRunner` | Git receives the concrete Infrastructure runner through its process port. |

These are explicit consumed contracts, not exceptions permitting feature-owned HTTP Views.

## Implemented Automation and Alerts contexts (Phase 9)

Automation owns `actions/` and `runs/`; Alerts owns `channels/`, `rules/`, and
`events/`. Their roots only declare modules and selectively export contracts.
Configuration and patch types keep Serde where JSON interpretation is part of their
semantics. Resource entities, read results, and Automation progress carry semantic
data without HTTP serialization or schema derives. Quiet-hours, time-zone, rule
metadata, run-log redaction, and cron helpers retain named semantic modules.

`AutomationRepository` keeps enqueue/claim/finish and action ownership atomic.
`AlertRepository` keeps incident deduplication, rule state, delivery outbox claims,
and retries under the existing transaction boundaries. PostgreSQL implementations
live under `adapters/src/postgres/{automation,alerts}`. Shoutrrr delivery is a separate
runtime adapter in `alert_delivery.rs`; global transport reorganization remains Phase 12.

Server owns requests, views, schemas and explicit conversions under
`api/{automation,alerts}`. Realtime uses those same response conversions. Automation
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

## Phase 10: Platforms, Identity and shared resource management

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

Read projections use Reader ports; durable resource mutations use Repository ports.
Sample/projection stores and transient MFA/OIDC security state retain Store where
that describes their actual responsibility. Registries, tags, bindings and secret
metadata have explicit owners described below; repository source configuration and
webhook configuration belong to Git. Encryption remains behind secret-protection ports;
HTTP mappings preserve credential redaction and never decrypt to construct a View.

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

PostgreSQL implementations live under `adapters/src/postgres/{tags,registries,bindings}`.
Tag-link SQL is reusable within an existing Registry or Git transaction, with typed
error conversion at that adapter boundary. Audit creation, tag replacement, row
locking and commit order remain in the original transaction. Shared access SQL
lives in the PostgreSQL authorization helper, not a feature-level umbrella port.
Platform description persistence is owned by `PlatformMetadataRepository`.

Server DTOs live under `api/{tags,registries,bindings,discovery}`. Tag, Registry and
Binding handlers live beside their DTOs; lookup/search retain their existing HTTP
modules. Git catalog handlers retain their Git namespace and their own HTTP state. Route
composition supplies only each owner's repository/service plus identity and realtime.
HTTP filters, metadata patch parsing and access helpers are server-local. Feature
errors are typed by owner; HTTP ProblemDetails mapping preserves existing behavior.
The broader HTTP error-boundary cleanup remains part of the later refactor.

### Domain removal and remaining Phase 11 work

The user requested domain ownership cleanup together with the Phase 10 structural
correction. The domain portion of Phase 11 has therefore been brought forward:

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

The separate `citadel-application` cleanup remains for Phase 11:

| Remaining application content | Destination |
|---|---|
| `activities` ports/service/filter/projections | Activities; public JSON sanitization/mapping to server |
| `licenses` services/ports/transition jobs | Licensing; request/View/schema types to server |
| `service_account_last_used` | Moved to `infrastructure/runtime`; implements Identity usage ports |
| bounded_queue, supervisor, polling, dynamic_tasks, io_budget, runtime_signal, runtime_metrics | Moved to `infrastructure/runtime` |

This correction does not claim completion of all Phase 11 work. Phase 12 still owns
final process lifecycle/shutdown auditing.

The Phase 10 transport guard intentionally does not certify final task ownership.
Two existing, bounded and awaited password `spawn_blocking` calls stay inside
Identity so that cancellation cannot release the hashing semaphore before the
blocking operation ends. The existing Container mutation task also retains its
claim-completion behavior when HTTP disconnects. Their three task-ownership
exceptions remain explicit in the external audit and must be resolved by the
Phase 12 lifecycle audit; they are not silently removed by moving their files.

Stacks now consumes Platforms' `PlatformKind` in its durable-operation claims and
read projections (Phase 10). This narrow cross-feature dependency prevents Stack
workers from reinterpreting persisted strings; it is acyclic because Platforms
has no dependency on Stacks. PostgreSQL owns the legacy spelling conversion.
