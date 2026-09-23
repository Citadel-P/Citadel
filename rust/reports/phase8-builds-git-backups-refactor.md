# Phase 8 — Builds, Git and Backups

Status: Phase 8 complete and validated. Phase 9 has not started.

## Files and ownership

- Builds: replaced the mixed root with `projects/`, `agent_pools/`, `runs/`, shared `repository.rs`, `runtime.rs`, `tasks.rs`, `permissions.rs`, and operation modules under `service/`. Moved project patches, pool patches/checker contracts, logs and health checks into their owners. Renamed durable completion persistence to `BuildCompletionRepository`.
- Git: replaced flat `accounts.rs` and `repositories.rs` with resource modules. Accounts separate models, commands, credential protection, persistence and service. Repositories separate the catalogue model/commands/persistence/service from execution, read models and webhook evaluation. Execution separates synchronization, materialization, browsing, discovery, credentials and validation.
- Backups: replaced the mixed root with `repositories/`, `policies/`, `runs/`, `restores/`, shared `BackupPersistence`, runtime ports, validation and operation-specific service modules. Metadata/patch/preview/progress/summary helpers moved to their resource owners.
- PostgreSQL: removed flat Build, Backup, Git-account, Git-execution and Build-completion store modules. Their implementations now live under `adapters/src/postgres/{builds,git,backups}/`, with transaction operations, row decoding, recovery and activity recording separated. Git catalogue CRUD moved out of `resource_metadata_store.rs`; that adapter delegates compatibility calls to `PostgresGitRepositoryPersistence`.
- Server: removed flat Build, Backup, Git-account and Git-execution HTTP modules. `api/{builds,git,backups}/` owns requests, views, schemas, capabilities, handlers and conversions. Git catalogue handlers moved out of the shared catalogue module. Realtime uses the same server-owned response conversions.
- Added `multi_resource_architecture.rs`, pool-task shutdown/admission coverage and capability hierarchy coverage. Updated consumers, fixtures, Cargo dependencies and `ARCHITECTURE.md`.

The earlier staged `queries.rs` → `read_models.rs` changes are preserved. No helper scripts, CSV inventories or generated audit JSON/text files were added to Git.

## Architectural decisions

`BuildRepository` and `BackupPersistence` retain atomic multi-resource claims, finish/fail operations, row-version checks and recovery. Splitting those transactions into independent resource ports would weaken ownership. Resource modules organize models and operations without duplicating transactional contracts.

`GitRepository` and `BackupRepository` are business resources. Git uses `GitRepositoryPersistence` for catalogue CRUD and `GitRepositoryExecutionPersistence` for durable synchronization. Its directory listings, comparisons, snapshots and source projections keep their semantic names.

Business resources no longer supply HTTP serialization or Utoipa schemas. Durable JSON configuration retains Serde where storage and merge patches require it. Explicit DTO conversions preserve request defaults and response fields. Git latest-activity presentation and Backup progress framing are server-owned.

Build pool tests use the process-owned `DynamicTasks` through `BuildTaskSpawner`. The accepted task acquires and completes the claim. Caller cancellation cannot drop accepted work; server shutdown cancels the checker, persists an invalid validation result, releases the claim and drains the owner. Admission rejection acquires no claim. Durable recovery still covers abnormal process failure.

## Compatibility and behavior differences

Public JSON field names, response wrappers, credential handling, webhook validation, audit payloads and OpenAPI component identities are preserved by explicit conversions and the compatibility checks below. Historical component names remain pinned even though source modules moved. The OpenAPI baseline is unchanged. No database migration is introduced.

Permission levels are ordinal. Migrated SQL list filters now bind the accepted database levels instead of bit masks. Build capability aggregation uses the maximum valid level, fails closed on invalid stored values, and represents administrator access explicitly instead of inventing level `7`. This intentionally denies malformed legacy levels rather than treating them as permission combinations.

Pool tests now have a shutdown owner. Shutdown rejection returns the existing conflict error form; an accepted check interrupted by shutdown persists an invalid result instead of leaving ownership detached from the process lifecycle.

## Deliberately retained legacy code

- `ResourceMetadataStore` and `PostgresResourceMetadataStore` retain compatibility delegation to Git persistence pending shared-resource normalization in Phase 10. Shared tag/audit helpers still belong to that migration.
- Resources re-exports Git webhook evaluation. Its existing `RepoWebhookConfig` wire/schema type remains for Automation until Phase 9. Git itself has no Utoipa dependency.
- Existing runtime executors, source planners and Docker/Agent/Edge transports retain their established locations pending adapter/server physical organization in Phase 12. Shared Platform/Identity APIs remain for Phase 10, and Automation/Alerts migration belongs to Phase 9.
- Transitional `domain` and `application` umbrella crates remain for Phase 11.

## Validation

Commands use disabled incremental compilation and debug information for validation to limit build-artifact growth. External tests use a disposable PostgreSQL container and dedicated fixture databases.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed |
| `cargo test --locked --workspace` | 657 passed, 0 failed, 238 ignored by the default workspace run |
| `cargo run --locked -p xtask -- openapi --check` | Passed: 404 full / 305 public operations |
| Phase 8 architecture guards | 2 passed, also included in the workspace suite |
| Adapter `build_execution` with PostgreSQL | 3 passed: claims/cancellation/recovery, authenticated registry resolution, protected secrets |
| Adapter `backup_execution` with PostgreSQL | 1 passed: repository exclusivity and stale completion rejection |
| Adapter `git_repository_execution` with PostgreSQL and real Git | 3 passed |
| Server `phase7_resources_http` with PostgreSQL | 1 lifecycle suite passed, including pool caller-drop, shutdown cleanup and rejected admission |
| Server `resources_http` with PostgreSQL | 1 authorization/catalogue/webhook lifecycle suite passed |
| Adapter `backup_restic_acceptance`, local volume round trip | 1 passed with Docker and Restic 0.18.1; restored data verified |
| Server `workers::builds::tests` with PostgreSQL | 3 passed: concurrency/shutdown and both retention contracts |

The selected external fixtures total 13 passing tests. Signed Agent/Edge, multi-node RustFS, and Forgejo/Vault acceptance suites were not run: their dedicated published Agent candidate, networks and service fixtures are not configured in this environment. Their deterministic transport/credential tests remain included in the workspace suite.

## Exemptions

Retired all 134 Phase 8 entries from the external Phase 0/1 architecture allowlists. Retained cross-feature contracts and their rationale are documented in `ARCHITECTURE.md`. No new architecture allowlist or exemption was added for Builds, Git or Backups. The new guards reject HTTP/schema dependencies, View definitions and detached Tokio tasks in these feature crates, and reject presentation mapping in their PostgreSQL modules. The shared Automation wire type remains outside the migrated features until Phase 9.
