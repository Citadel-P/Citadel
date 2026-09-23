# v13 Phase 6 — Deployment reference resource

Deployments is migrated end to end. This is the v13 architecture phase, distinct from the earlier API-porting Phase 6a/6b reports. Phase 7 is not started.

## Resource mapping

| Resource | Old feature type | New business/read type | Server View | Feature module | Postgres module | Status |
| --- | --- | --- | --- | --- | --- | --- |
| Deployment | `DeploymentView` | `Deployment` + `DeploymentDetails` | `api::deployments::views::DeploymentView` | `deployments::{model,read_models,service}` | `postgres::deployments::PostgresDeploymentRepository` | Migrated |

Config and duplication are semantic projections (`DeploymentConfig`, `DeploymentDuplicateDraft`, `DeploymentDraft`), not independent resources. Adoption owns semantic previews and a command. Apply progress is `DeploymentProgress`; only server serializes its HTTP stream representation.

## Files created, moved and removed

Paths below are relative to `rust/crates/`. Old paths are removed, without forwarding modules.

| Old file/module | Responsibility found | New file/module(s) | Reason |
| --- | --- | --- | --- |
| `deployments/src/model.rs` | Entity, spec/storage conversion, HTTP DTOs, claims/progress | `model/{mod,resource,spec,operations,tests}.rs`, `commands.rs`, `read_models.rs`, server `api/deployments/{requests,views,spec}.rs` | Separate business and persistence semantics from presentation |
| `deployments/src/service.rs` | Repository/runtime ports, use cases, detached work, tests | `repository.rs`, `runtime.rs`, `tasks.rs`, `service/{mod,read,mutations,apply,delete,bindings,adoption,tests}.rs` | Cohesive operations behind one service façade |
| `deployments/src/updates.rs` | Update checks and scheduling | `service/updates.rs` | Service-owned orchestration |
| `adapters/src/deployment_store.rs` | Persistence, ACL queries, capabilities, activity, claims | `postgres/deployments/{mod,repository,queries,rows,authorization,mutations,claims,activity,tests}.rs`; server `capabilities.rs` | Durable Repository; presentation removed from SQL adapter |
| `adapters/src/{container_adoption,deployment_bindings,deployment_updates_store}.rs` | Adoption, binding resolution, update claims | `postgres/deployments/{adoption,bindings,updates}.rs` | Resource namespace; atomic operations preserved |
| `adapters/src/postgres.rs` | Existing authorized Platform reader | `postgres/{mod,platform_reader}.rs` | Allow the Deployment namespace; reader behavior unchanged |
| `server/src/deployments_http.rs` and `deployments_http/adoption.rs` | Routes, extraction, authorization, notifier | `api/deployments/{mod,handlers,adoption,adoption_views}.rs` | Server owns the complete inbound boundary |
| `server/src/api/mod.rs` | Binary router composition | `server/src/router.rs`; new library `api/mod.rs` | Keep binary AppState composition separate from reusable API modules |
| No predecessor | Process task-owner adapter | `server/src/api/deployments/tasks.rs` | Feature consumes its own task-admission port |

`permissions.rs`, crate façades/manifests, composition, OpenAPI registration, realtime mapping and dependent test/import sites are updated. `ARCHITECTURE.md` now documents the actual reference implementation; `AUTHORIZATION.md` records the completed boundary. No generated scripts, raw measurements or new schema snapshots are added.

## Architecture and compatibility

- `Deployment` has durable state and row version. Enriched query data and typed effective permissions live in `DeploymentDetails`. Neither is an HTTP response contract. Feature commands/projections have no Utoipa dependency; explicit server conversions own schema names, null omission, defaults, capabilities and activity presentation.
- `DeploymentRepository` preserves atomic claims, row-version checks, ACL-aware SQL and batch enrichment. Named policies are reused for inbound and authoritative transactional checks. Apply remains Read + Apply, as accepted in Phase 2. Destination binding-copy access has an explicit Write + ResourceBindings policy. Foreign Platform/container checks remain with their existing owners.
- PostgreSQL retains the same schema and persisted JSON. Server value-object conversions preserve all Local/External/Build fields, including resolved/applied build provenance. HTTP and realtime use the same View mapper. The legacy `canPull` field remains a presentation capability; no Pull operation is invented.
- Apply, delete and update-check tasks use the Phase 5 process-owned `DynamicTasks` through the consumer-owned `DeploymentTaskSpawner`. Request/progress drop does not cancel accepted claimed work. The existing runtime cancellation, timeout and stale recovery behavior remains. Rejected Apply finalizes its unexecuted claim as failed; rejected delete releases its claim; rejected update checks never claim. All release their permits. Errors reach the owner even when a response receiver has disconnected.
- The existing `DeploymentRuntimeRouter` continues implementing the renamed `DeploymentRuntime` port. Docker/Direct Agent/Edge behavior is retained; unrelated runtime modules are not relocated.

The task admission gate and owner-visible errors are the intended lifecycle changes. Public payloads and persisted formats remain unchanged. Raw activity presentation now happens only in server.

## Validation

All commands run from `rust/` on Rust 1.97.1. Raw build/test logs remain outside Git.

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check`; `git diff --check` | Pass |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Pass; no lint exemptions |
| `cargo test --locked --workspace` | Pass: 643 passed, 0 failed, 238 environment-dependent tests ignored in this command; relevant external fixtures explicitly run below |
| `cargo run --locked -p xtask -- openapi --check` | Pass: 404 full / 305 public operations; schema snapshots and generated frontend types unchanged |
| Server `deployments_http` | 1 passed, no ignored; CRUD/actions, typed capabilities, revocation, update checks and measured query budgets |
| Adapters `deployment_persistence`, `deployment_apply_persistence`, `deployment_runtime_local` | 4 passed, no ignored; ACLs, duplicate bindings, tags/activity/runtime enrichment, row versions, atomic claims/recovery, secret masking and real Docker create/delete |
| Adapters `identity_authorization_differential` | 2 passed; shared .NET permission matrix and PostgreSQL resolution |
| Server `platforms_http` | 62 passed, no ignored; adoption, HTTP/realtime payload equality, inspection/statistics/log/terminal permissions and routing |
| Adapters `edge_transport` (Deployment Apply case) | 1 passed; Edge routing without local Docker fallback |
| Server `bootstrap_process` (startup/listener-shutdown case) | 1 passed; actual binary composition, route registration, workers and bounded shutdown |

External fixtures use `-- --include-ignored --test-threads=1` with their existing database environment variables. Deployment HTTP additionally requires `CITADEL_REQUIRE_QUERY_COUNTS=1`; its database enables `pg_stat_statements`. Separate databases isolate HTTP measurements from the other suites. Docker uses a pinned Alpine image. The owned disposable PostgreSQL container and volumes are removed after these checks.

| Measured HTTP operation | Accepted baseline | Phase 6 |
| --- | ---: | ---: |
| Non-admin get | 2 | 2 |
| Non-admin list, three authorized rows | 3 | 3 |
| Administrator get | 1 | 1 |
| Administrator list | 1 | 1 |

The workspace tests include retained request/progress-drop and timeout/stale-recovery cases, new admission-rejection cleanup and process-owner drain tests, specification wire/storage conversion, server capability parity and the architecture guard. During migration, compilation caught stale DTO-based test fixtures; the workspace run also caught an accidentally renamed schema-name lookup in a test. These were corrected without changing the public schema or relaxing checks.

## Deferred code and exemptions

Stacks/Swarm (7), Builds/Git/Backups (8), Automation/Alerts (9), Platforms/Identity/shared resources (10) and umbrella ownership (11) retain their existing architectures. Edits at their Deployment integration points only update consumed types/imports or fixture construction. The existing runtime router and shared activity presentation helper retain their established locations.

No temporary architecture allowlist or lint exemption is added. No tracked Deployment architecture allowlist existed to remove; the historical deferred-boundary note is replaced and the Rust convention guard now rejects Deployment Views/capabilities/Utoipa/server dependencies and direct task spawning in feature/persistence production sources.
