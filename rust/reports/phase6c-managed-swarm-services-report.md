# Phase 6C managed Swarm Services report

## Outcome

Phase 6C ports the core managed Docker Swarm Service lifecycle to Rust: typed
CRUD, Apply, scale, force update, delete, bounded progress, durable operation
claims, rollout observation, Activities, realtime invalidation, and
reconciliation. Docker accepting a mutation is not treated as rollout success.

Local and signed Agent connectors use the same application port. Agent
mutations are dispatched once because retrying an ambiguous mutation response
is unsafe. Edge Agent mutation remains unavailable until its inbound command
transport migrates; Rust does not silently route it through another manager.

## Correctness and safety boundaries

- PostgreSQL keeps the accepted .NET PascalCase `SwarmServiceSpec` shape while
  HTTP uses camelCase and the existing `$type` image discriminator.
- desired-state hashes ignore image provenance and webhooks, sort set-like
  collections, and preserve command/argument ordering;
- a transaction claims each operation before Docker I/O;
- at most four Service operations run concurrently and progress uses a bounded
  32-item channel;
- variables and internal encrypted Secrets are resolved only into the runtime
  request, Secret values are zeroized and redacted, and persisted specs keep
  their references;
- Docker 4xx responses are definite rejections, while transport loss and
  timeout remain `OutcomeUnknown` for later observation;
- accepted rollouts are inspected by a five-second worker in batches of 25;
- paused and rollback terminal states persist the rollout failure instead of a
  false success Activity;
- Secret and Config file targets use UID/GID `0` and mode `0444`;
- runtime health and Platform managed-Service counts are derived from the
  current bounded Swarm projection, including stopped, degraded, and failed
  Services.

## Test mapping

The .NET `ManagedSwarmServiceTests` validation and desired-hash cases map to
`citadel-swarm-services` tests. The applicable core lifecycle scenarios from
`ManagedSwarmServiceEndpointTests` map to the PostgreSQL adapter and Axum HTTP
tests. They cover authentication, typed creation, optimistic concurrency,
operation claim/accept/complete, progress, Activities, deletion, and persisted
queries. Runtime adapter tests cover Docker file-target defaults and exact
restart-policy serialization. Generated OpenAPI and Docker endpoint checks
remain part of the gate.

Adoption, duplicate drafts and copied bindings, webhooks, image update checks,
logs, terminal, statistics, build-backed images, and external Secret providers
remain owned by later mapped slices. They are not advertised here as complete.

Run the repeatable disposable gate from the repository root:

```powershell
.\rust\scripts\Test-Phase6CManagedSwarmServices.ps1
```

## Next slice

Phase 6D owns Docker Standalone Stack desired state, Apply progress,
transactional releases, and reconciliation. It must begin by mapping the
existing .NET Stack unit, integration, and acceptance scenarios.
