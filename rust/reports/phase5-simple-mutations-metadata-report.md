# Phase 5 simple mutations and metadata report

Date: 2026-09-03

## Outcome

Phase 5 is implemented and accepted. Rust now owns the existing Citadel HTTP
contracts for Tags, Registry and Git repository metadata, resource bindings,
internal and external Secret definitions, Vault-compatible Secret providers,
Platform descriptions, and the selected Network/Volume create/delete
operations. The unchanged frontend receives authoritative invalidations over
the versioned WebSocket.

The context and worker boundaries were also cleaned up before adding the
mutations. `citadel-resources` owns metadata models, validation, use cases, and
ports. `citadel-platforms::jobs` owns inventory/event/statistics behavior.
Concrete SQLx, Docker, and Agent implementations remain in `citadel-adapters`,
while `citadel-server::workers` owns only scheduling and supervision. No new
process or binary was introduced.

## Implemented behavior

### Tags and shared metadata

- Tag names/colors are normalized and validated before storage; normalized
  names remain unique under concurrent requests.
- Tag assignment validates every target Tag and replaces the complete set in
  the same transaction.
- Platform, Registry, and Git repository tag routes use the resource's own ACL.
  Other aggregate tag routes move with those aggregates in Phases 6 and 7.
- Platform description changes return 400/404/409/500 Problem Details rather
  than collapsing expected failures into 500.

### Registries and Git repositories

- Authorized list/detail/config/create/update/metadata/rename/delete contracts
  retain the existing method, path, and response shapes.
- List queries filter unauthorized resources in PostgreSQL. Resource mutations
  evaluate the concrete resource ID, including every ID in batch deletes.
- Registry and Git configuration updates use recursive JSON Merge Patch
  semantics. Metadata-only and rename routes cannot accidentally mutate
  credentials or operational configuration.
- Tag changes, resource state, and typed safe Activity evidence commit in one
  transaction. Registry credentials are redacted from Activity snapshots.
- Git detail projections include the latest relevant Activity without adding an
  N+1 query to collection reads.

Actual Registry connectivity validation, Git clone/pull, Git webhooks, and Git
commands remain Phase 7 external-execution behavior. Phase 5 persists their
validated definitions but does not start external work implicitly.

### Bindings and Secret definitions

- Global and resource-scoped variables/Secrets preserve the current scope and
  permission rules. Plaintext Secret values are never returned.
- Internal Secret values and provider tokens use authenticated, versioned
  encryption at rest.
- Mounted-file targets reject unsafe paths; global bindings accept only
  environment-variable Secret delivery.
- Deleting the last binding to a Secret removes its orphaned definition and any
  internal encrypted value in the same transaction. Shared Secret definitions
  remain intact.
- External Secret patches distinguish omitted fields from explicit `null`, so
  an omitted version is preserved and `null` clears it.
- Secret-provider updates lock and merge the current row, preserving omitted
  fields and credentials.

Contacting Vault to validate a provider or resolve a Secret remains Phase 7.
No external call is hidden inside these metadata transactions.

### Network and Volume mutations

- Citadel-generated Docker API models and operations implement Network and
  Volume create/delete for Local and Agent transports; Bollard is not used.
- Agent requests retain the signed protocol and transport-equivalent payloads.
  Ambiguous mutation failures are not retried.
- Inputs and batches are bounded and validated before transport. Network delete
  preflight rejects system, in-use, Stack-owned, or incorrectly scoped targets
  before the first irreversible delete.
- Batch deletion reports partial completion if Docker changes after preflight.
- Swarm node-local Volume deletion remains unavailable until explicit node
  routing is owned by the later Swarm/orchestration slice; Citadel does not
  silently target the manager's local Volume.

### Realtime and background ownership

- Mutations publish metadata-free invalidations only after successful state
  changes. Read requests never publish changes.
- Global realtime clients learn only the resource type/event kind, then repeat
  their normal authorized query; resource IDs are not leaked across ACLs.
- The React query cache invalidates the affected Registry, Git repository, Tag,
  Binding, and Activity keys.
- Platform inventory reconciliation, event policy, and container-stat sampling
  are context-owned modules. The server owns bounded channels, timers,
  cancellation, and supervised task lifetime.

## Test mapping from the .NET reference

| .NET reference area | Rust proof |
| --- | --- |
| `TagTests`, `TagViewTests`, and `ResourceTagIntegrationTests` | `citadel-resources` unit tests plus `resource_metadata_persistence.rs` cover normalization, uniqueness, atomic assignment, authorization, and concurrent duplicate creation. |
| `RegistryCreateTests`, `RegistryPatchTests`, `RegistryDeleteTests`, and `RegistryViewTests` | `resources_http.rs`, `resource_metadata_persistence.rs`, and the recursive merge-patch unit test cover compatible routes, ACLs, protected projections, activities, rollback, tags, rename/metadata separation, and deletion. Remote connectivity is Phase 7. |
| Git repository create/patch/delete/view and webhook-validation tests | Resource unit and HTTP integration tests cover bounds, URL normalization, webhook/command metadata, account references, latest Activity, ACLs, transactions, and deletion. Clone/pull/webhook execution is Phase 7. |
| `ResourceBindingsTests` | Binding unit tests and both PostgreSQL-backed suites cover global/scoped permission rules, variables, encrypted internal Secrets, external Secret version merge semantics, provider field preservation, orphan cleanup, and non-disclosure. External provider I/O is Phase 7. |
| `NetworkEndpointTests`, `CreateNetworkTests`, and Swarm Network lifecycle tests | `platforms_http.rs`, Docker transport tests, and `agent_mutations.rs` cover validation, preflight, generated Docker requests, Local/Agent parity, signing, and no retry after ambiguous mutation failure. |
| `VolumeEndpointTests` | The same Platform/transport suites cover create/delete payloads, custom drivers, bounds, authorization, and transport parity. Volume browsing/backup belongs to later phases. |
| SignalR cache synchronization | `realtime_subscription.rs` and `signalr-provider.test.tsx` cover bounded delivery, ACL recheck, metadata-free global invalidation, and frontend query refresh behavior. |

## Verification

Passed locally on 2026-09-03:

- `cargo fmt --all`
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --no-fail-fast` (all non-environment tests)
- `cargo test -p citadel-adapters --test agent_mutations`
- `scripts/Test-Phase5Metadata.ps1`, including the disposable PostgreSQL-backed
  adapter and authorized Axum lifecycle suites, the Linux Unix-socket Platform
  mutation fixture, Agent transport equivalence, realtime bounds, and generated
  contract checks
- `cargo run -p xtask -- openapi --check` (159 full, 107 public routes)
- full frontend unit suite (100 files, 340 tests), including the 15 focused
  realtime tests
- frontend ESLint with zero warnings

The disposable acceptance script completed successfully and removed its test
database container and network. Phase 5 therefore satisfies its authorization,
concurrency, error, transaction, transport-equivalence, and recovery exit gate.

## Remaining migration gates

Phase 5 implementation does not close the existing Phase 4 Edge Agent and
cgroup-soak gates. It also deliberately excludes orchestration aggregates
(Phase 6) and external execution/recovery (Phase 7). These are explicit phase
owners, not placeholder behavior inside the metadata routes.
