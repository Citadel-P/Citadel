# Phase 1 compatibility boundary

## Product behavior that remains compatible

The Rust rewrite preserves, unless the breaking-change ledger says otherwise:

- the accepted HTTP operation IDs, methods, paths, request/response fields,
  Problem Details mapping, and capability-driven action availability;
- the existing React resource registries, generated API consumption,
  authorization-aware pages, and realtime cache semantics;
- all active Agent and Edge Agent protobuf contracts and Local/Agent/Edge
  transport equivalence;
- Actor, Team, Role, Service Account, global permission, resource ACL, and
  licensing decisions at every request, stream, and job boundary;
- aggregate invariants, create-versus-apply, desired-versus-observed state,
  operation certainty, releases, activities, alerts, and reconciliation;
- durable data meanings, uniqueness, defaults, foreign keys, checks, JSON
  representations, UTC timestamps, and transactional ownership boundaries;
- bounded queues and streams, cancellation, timeouts, retries, leases,
  backpressure, process cleanup, and restart recovery; and
- Docker API semantics generated from Citadel's pinned upstream schema rather
  than a third-party Docker runtime client.

## Internal compatibility that is intentionally not preserved

Rust does not reproduce:

- .NET assemblies, namespaces, classes, interfaces, Mediator handler shapes,
  pipeline behavior types, DI registration APIs, or `IUnitOfWork` as a service
  locator;
- Dapper, Dapper AOT generated code, EF models, EF migration history, Refit,
  source-generated JSON contexts, or SignalR wire framing;
- private repository method signatures, internal SQL text, in-memory cache
  layout, object identity, log wording, stack traces, or allocation patterns;
- unreleased historical database migrations and ephemeral cookies, refresh
  tokens, OIDC state, setup state, MFA challenges, or transient operation
  buffers; or
- undocumented behavior that contradicts an accepted specification or a
  tested security/correctness invariant.

Framework replacement is not permission to change product ordering. Validation,
authentication, permission, resource ACL, licensing, transaction, remote I/O,
activity, and post-commit notification order remains explicit in each Rust use
case and its tests.
