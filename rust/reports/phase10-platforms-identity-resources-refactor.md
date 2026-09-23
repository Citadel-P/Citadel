# Phase 10 — Platforms, Identity and shared Resources

Status: the original Phase 10 validation completed on 2026-09-20. The user then
requested the structural correction documented in
[resource-structure-correction.md](resource-structure-correction.md), which supersedes
the Identity layout exception and brings forward the domain portion of Phase 11.
Validation below describes the original Phase 10 run; see that report for the correction.

## Files and ownership

- Platforms now separates `read_models.rs`, `repository.rs`, `service/read.rs`,
  `runtime.rs` and the closed classifications in `model.rs`. Its former `read.rs`
  and implementation-bearing crate root are retired.
- Identity originally retained local `domain/` and `application/` layers. The
  structural correction removes both: models, commands, read models, repositories
  and services now live under their resource namespaces directly beneath `src/`.
  Invariants and atomic security workflows remain in Identity.
- Resources has explicit registry, tag and binding model/command/validation
  modules, a shared durable repository, a service, and lookup/search Reader ports.
  Provider connection testing lives in `secret_providers.rs`, replacing the
  misleading `secret_provider_tests.rs` production-module name.
- Server owns the moved Views, request DTOs and schema metadata in
  `identity_http/dto.rs`, `resources_http/dto.rs`, and `platforms_http/{dto,views,swarm_views}.rs`.
  HTTP and realtime callers map the semantic results explicitly. Platforms no
  longer stores presentation capability fields in database read projections.
- PostgreSQL Platform reads and shared resource persistence are grouped under
  `adapters/postgres/{platforms,resources}/`, with reader/repository, row, query
  and audit responsibilities separated. The Swarm summary SQL moved with its reader.
- The duplicated Resources webhook configuration file was removed. Git catalogue
  persistence now has its own HTTP/realtime handles; Resources no longer inherits
  or implements Git's persistence port. Shared semantic webhook validation still
  delegates to Git for Builds/Stacks compatibility.

All three feature crates have deliberate crate-root exports and no Utoipa dependency,
HTTP `*View` declarations or generated Docker protocol dependency. Editing helpers,
logs and audit backups are outside Git under `/tmp/citadel-phase10`.

## Runtime and compatibility

`ConnectorKind` distinguishes Local, Agent and EdgeAgent. `PlatformKind` distinguishes
Docker and DockerSwarm. Database adapters decode exact persisted strings, accepting
both historical standalone spellings, `Docker` and `DockerStandalone`. Runtime target
registries, inventory jobs, Stack claims, and Deployment/Stack/Swarm routing consume
typed classifications. Unknown persisted classifications produce errors rather than
being interpreted as a supported runtime. Stack read projections expose the canonical
`Docker` spelling for both standalone aliases.

No database migration is required. Existing request defaults, patch missing/null
semantics, cookie/session behavior, credential redaction, password hashing, MFA/OIDC
policies, administrator checks and resource transaction boundaries are retained.
Serde remains on semantic configuration/patch values and existing durable projections;
OpenAPI and HTTP contract ownership is exclusively server-side.

## Retained work and exemptions

The original run retained both umbrella crates. The subsequent user-directed
correction removes `citadel-domain`; `citadel-application` remains for Phase 11.
`ARCHITECTURE.md` records implemented owners and outstanding moves.

105 external architecture exemptions were retired (82 Phase 0, 23 Phase 1).
Three existing task exceptions are explicitly carried to Phase 12: the two bounded,
awaited password-blocking operations, and the Container mutation task whose committed
claim must survive HTTP disconnect. Their implementation behavior is unchanged;
Phase 12 must normalize process ownership while preserving permit and claim lifetimes.
No transport/schema exemption was added.

## Validation

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed.
- `cargo test --locked --workspace`: 658 passed, 0 failed, 239 external tests ignored
  by the default run.
- `cargo run --locked -p xtask -- openapi --check`: passed; verified 404 full and
  305 public operations.
- Explicit PostgreSQL-backed runs: 152 passed, 0 failed across 14 suites:
  `users_http` (28), `teams_http` (10), `roles_http` (7),
  `service_accounts_http` (6), `mfa_http` (1), `oidc_http` (1),
  `platform_creation_http` (13), `platforms_http` (62), `resources_http` (1),
  `phase7_resources_http` (1), `identity_access` (1),
  `identity_authorization_differential` (1), `resource_metadata_persistence` (2),
  and `platform_inventory_persistence` (18). Each suite used a separate disposable
  database. The remaining 87 external tests were not run in this phase.
- `git diff HEAD --check`: passed.

The targeted suites cover authentication/session behavior, MFA/OIDC, administrator
and ACL rules, service-account tokens, Platform registration/read/runtime endpoints,
inventory persistence, registries, tags, bindings and secret metadata. Docker protocol
tests use local fixtures; this is not a live Docker workload or external secret-provider
integration certification. The two new architecture checks and persisted-classification
round-trip/rejection test passed in the workspace run.

Validation disabled debug information and incremental compilation through Cargo profile
environment overrides to limit disk growth. Logs and helper scripts remain outside the
repository under `/tmp/citadel-phase10`.
