# Citadel Rust architecture

This describes the target architecture of the incremental Rust refactor. Existing
resources move to these conventions in their owning migration phases. The examples
below describe planned roles, not scaffolding to create before a resource is migrated.

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
framework. `citadel-domain` and `citadel-application` are transitional umbrellas to
remove in Phase 11; the final tiny primitives bundle moves once in that phase.

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
| `queries` | Optional read ports, filters and semantic projections |
| `runtime` | Optional Docker/agent/external execution ports |
| `validation` | Reusable business validation too large for model/service |
| `jobs/`, `adoption`, `webhooks` | Optional cohesive background/adoption/webhook use cases |

Start with role files such as `model.rs`, `repository.rs`, `service.rs`. Expand a large
slot into `service/mod.rs`, `service/apply.rs`, etc., preserving the conceptual entry
point. Use ordinary modules rather than avoidable `#[path]` indirection. Resource
namespaces are plural; role filenames are singular; Rust modules use snake_case and
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

## Complete single-resource target: Deployment

```text
crates/deployments/src/
  lib.rs                         # selective exports
  model.rs                       # Deployment, specification, state, claims
  repository.rs                  # DeploymentRepository, scoped durable operations
  service.rs                     # DeploymentService
  commands.rs                    # transport-neutral mutation inputs
  queries.rs                     # filters, DeploymentSummary/Details when useful
  runtime.rs                     # DeploymentRuntime port
  adoption.rs                    # adoption use cases
  jobs/                          # background work, only where needed
crates/adapters/src/
  postgres/deployments/
    mod.rs                       # exports
    repository.rs                # PostgresDeploymentRepository
    rows.rs                      # SQL row decoding when useful
  docker/deployments/
    mod.rs
    runtime.rs                   # DockerDeploymentRuntime implements feature port
crates/server/src/api/deployments/
  mod.rs                         # router assembly
  handlers.rs                    # extraction, authorization, use-case invocation
  requests.rs                    # HTTP request DTOs
  views.rs                       # DeploymentView and response mappings
  capabilities.rs                # presentation mapping if large enough
```

These are target roles, not scaffolding to create in Phase 1. The root selectively
exports `Deployment`, `DeploymentRepository`, `DeploymentService` and
`DeploymentRuntime`. A get flows from the handler through scoped persistence to a
model/read projection, then through server mapping to the existing JSON contract.
Apply invokes the feature service, atomically claims the version in persistence,
executes through the runtime port, persists the result and emits a transport-neutral
event. Progress streaming observes this operation; disconnect must preserve the
established durable-operation contract.

Typed policies will be introduced in Phase 2 and consumed during resource migration.
Collections remain SQL ACL-filtered. A transactional recheck is authoritative; a
capability is only presentation. Do not add authorization round trips to satisfy an
API shape. The known `Read + Apply` versus `Execute + Apply` discrepancy remains
recorded for Phases 2/6; this document does not change current behavior.

## Complete multi-resource target: Builds

```text
crates/builds/src/
  lib.rs
  projects/
    mod.rs
    model.rs                     # BuildProject, argument/secret specifications
  runs/
    mod.rs
    model.rs                     # BuildRun, BuildLog, claim/result types
  agent_pools/
    mod.rs
    model.rs                     # BuildAgentPool, provider configuration/state
  repository.rs                  # BuildRepository: shared atomic persistence
  service.rs                     # BuildService: cross-resource orchestration
  commands.rs                    # neutral build/project/pool inputs when useful
  queries.rs                     # query/filter projections when useful
  runtime.rs                     # build executor and provider ports
  jobs/                          # claimed background execution
crates/adapters/src/
  postgres/builds/
    mod.rs
    repository.rs                # PostgresBuildRepository, atomic implementation
    projects.rs                  # optional internal query implementation
    runs.rs
    agent_pools.rs
  docker/builds/
    mod.rs
    runtime.rs                   # Docker execution implementation
  external/builds/
    mod.rs
    providers.rs                 # external build-agent providers
crates/server/src/api/builds/
  mod.rs
  handlers.rs
  requests.rs
  views.rs                       # BuildProjectView/BuildRunView/BuildAgentPoolView
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
Phase 4 owns production adapter migration. See `crates/docker-api/README.md` for
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
to Phase 5 and adoption to the resource phases; this phase introduces no runtime code.

## Incremental migration

Keep changes focused on the resource being migrated. Existing legacy layouts do not
justify adding new ones. Preserve API schemas and persisted formats through moves;
run workspace tests and OpenAPI verification. Authorization conventions and parity
are documented in [AUTHORIZATION.md](AUTHORIZATION.md).

Feature Views and legacy umbrella crates remain until their owning migration phases.
Avoid mass renames, shared scaffolding and performance changes during authorization
work. Keep diagnostics and raw measurement output outside the source changes.
