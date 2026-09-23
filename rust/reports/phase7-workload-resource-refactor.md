# v13 Phase 7 — Stack and Swarm Service resource refactor

Scope: Stack, StackRelease, SwarmService and SwarmServiceOperation. Phase 8 is not started.

## Files and ownership

- Replaced `stacks/src/model.rs` and `swarm-services/src/model.rs` with `model/`,
  `commands.rs` and `read_models.rs`. Added `permissions.rs` and `tasks.rs`; extracted
  `repository.rs` and `runtime.rs`. Public crate façades export named contracts.
- Replaced both feature `service.rs` files with operation-specific `service/`
  modules. Moved update-check/image-update execution under those services.
- Replaced flat `stack_store.rs`/`swarm_service_store.rs` and their child modules
  with `adapters/src/postgres/{stacks,swarm_services}/`: repository delegation,
  queries, rows, authorization, mutations, apply/delete, activity and recovery.
  Moved workload binding persistence and Stack release-resource capture there.
- Replaced `server/src/{stacks,swarm_services}_http.rs` with
  `server/src/api/{stacks,swarm_services}/`: request DTOs, specifications, views,
  capabilities, tracked-task adapters and handlers grouped by use case.
- Updated composition, realtime, shared runtime/metadata endpoints, consumers and
  fixtures to use the new contracts. Removed Utoipa dependencies from both features.
- Added workload architecture guards, permission/capability and decoder tests,
  request-drop/shutdown/rejection integration coverage, query-count assertions and
  Swarm Apply/Scale/ForceUpdate HTTP authorization coverage. Existing serialization
  tests moved to the server where the wire representations now live.
- Updated `ARCHITECTURE.md` and `AUTHORIZATION.md`. No generated scripts, CSV, JSON
  snapshots or text inventories are added to the Git changes.

## Implemented decisions

`Stack` owns durable identity/configuration state and its current release reference;
`StackRelease` owns the release specification and provenance. Enriched reads use
`StackDetails` and `StackReleaseDetails`. `SwarmServiceDetails` wraps the canonical
`SwarmService`, runtime enrichment and `SwarmServiceOperation`. Read projections use
immutable dereferencing; mutation explicitly addresses the owned business object.
Operation persistence stays in `SwarmServiceRepository` because its fences and state
are stored atomically on the parent Service row, not as an independently saved entity.

Repositories return semantic data and `EffectivePermission`, with an explicit
administrator variant. SQL ACL filtering, ordering, batched tags/activity/runtime
metadata, claims, row versions and transaction boundaries remain. HTTP and realtime
map these projections into the same server-owned views. Duplicate drafts and sources
are typed application data; their legacy JSON envelopes are constructed/parsed in
the server. `UpdateStack.spec` intentionally retains JSON merge-patch semantics.

Feature-owned task admission ports delegate to the existing process `DynamicTasks`.
Stack claims are released if admission rejects the already-claimed Apply/rollback;
Swarm operations and image update checks claim only inside accepted tasks. Permits
remain bounded and are released on rejection. Accepted work survives request drops.
Shutdown drains the process owner, while persisted unknown outcomes retain the
existing reconciliation path. Redacted operation/finalization failures reach the
owner; terminal progress is still sent if failure persistence itself fails.

Stored specifications keep their existing Serde formats. Server specification DTOs
have explicit field conversions and preserve wire defaults/aliases/schema names.
`StackDrift` also retains serialization for durable alerts and deduplication hashes.
These business serialization uses are not HTTP View ownership.

## Compatibility and differences

- Public routes, `ManagedSwarmServiceView` naming, realtime targets, envelopes,
  progress format and OpenAPI snapshots are preserved. No storage migration is added.
- Stack Apply/rollback preserves Execute + Apply, despite the generic identity matrix
  describing Read + Apply. This is the existing Stack endpoint/claim contract.
- Swarm `canViewLogs`/`canInspect` remain legacy read-based hints. Actual logs/inspect
  endpoints still require specific permissions and their existing Platform access.
  Tests explicitly distinguish hints from operation authorization.
- Stack state prechecks now require Execute through `ChangeStackState`, matching
  the transaction. Write-only actors are rejected earlier; effective access is unchanged.
- Unknown persisted hierarchy values now fail decoding. Unknown specific bits remain
  representable and do not grant known operations. Administrator access does not use
  fabricated integer grants.
- Shutdown now rejects new detached operations through the process admission gate;
  runtime cancellation and durable recovery semantics are preserved.

## Deliberately deferred

Builds/Git/Backups (8), Automation/Alerts (9), broad Platforms/Identity/shared resource
normalization (10), and umbrella ownership (11) remain in their existing architectures.
Their workload integration points only adopt consumed types, policies or constructors.
Runtime/source-materialization/build-image/update-scanner adapter locations remain
established: this is not a global physical adapter reorganization. The Swarm legacy
capability-hint discrepancy needs a separate compatibility decision.

No temporary architecture allowlist or exemption was added. Existing large-enum
representation choices remain; this phase does not optimize memory layouts.

## Validation

The external suites run against a disposable PostgreSQL instance with
`pg_stat_statements`, using a separate database per suite. Live runtime checks use
an isolated two-node Docker 29.7.2 Swarm. Temporary scripts, logs, CLI and runtime
files stay under `/tmp`, outside Git. The host Docker 23.0.1 CLI lacks the existing
`stack deploy --detach` option, so runtime tests use a temporary 29.7.2 CLI. An
existing root-owned runtime temporary directory is avoided with fixture `TMPDIR`.

The persistence fixture now applies the edited configuration before expecting the
previous version in release history; the existing SQL intentionally excludes the
current version. Swarm task completion is asserted through operation status rather
than inventory-derived health, which remains Unknown until inventory is available.

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed.
- `cargo test --locked --workspace`: 651 passed, 0 failed, 238 ignored external tests.
- `cargo run --locked -p xtask -- openapi --check`: passed; 404 full and 305 public
  operations, no snapshot changes.
- `git diff --check`: passed.

The following ignored integration suites were explicitly executed with
`cargo test --locked -p <package> --test <target> -- --ignored --test-threads=1 --nocapture`:

| Package | Target | Passed |
| --- | --- | ---: |
| citadel-server | stacks_http | 1 |
| citadel-server | swarm_services_http | 1 |
| citadel-adapters | stack_persistence | 1 |
| citadel-adapters | swarm_service_persistence | 3 |
| citadel-adapters | stack_webhooks | 1 |
| citadel-adapters | git_repository_execution | 3 |
| citadel-server | platforms_http | 62 |
| citadel-server | phase7_resources_http | 1 |
| citadel-adapters | stack_runtime_local | 3 |

Both workload HTTP suites require query-count instrumentation and verify each
resource list remains one SQL query. They also exercise permission matrices,
request-drop independence, closed admission, permit/claim cleanup and shutdown
recovery. Platform tests cover workload adoption, logs/inspection and realtime
projections. The remaining ignored workspace tests concern other external suites.

All 76 explicitly selected integration tests passed. Live runtime coverage includes
Compose ownership and DestroyBeforeDeploy container replacement; native Swarm
apply/delete and failed-release observation for rollback-completed/paused states;
and two-node release resource capture, fencing, interrupted-release recovery,
worker-loss degradation and recovery after rejoining. The Docker tests exercised
actual deployment and node recovery; rollback states were injected at the inventory
projection boundary to verify observation semantics.

The disposable PostgreSQL and two Swarm nodes/network are removed after validation.
No user containers, databases or system Docker installation are changed.
