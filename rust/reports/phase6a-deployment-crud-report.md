# Phase 6A Deployment CRUD report

Date: 2026-09-03

## Compatibility correction — 2026-09-05

The existing frontend can omit `spec.updateBehavior` when creating a Deployment.
Rust incorrectly required it during JSON deserialization, returning a generic
400 before reaching the create handler. The supplied nginx/External/bridge payload
reproduced `missing field updateBehavior` in a regression test before the fix.

Omission now defaults to `Disabled`, matching .NET's zero-valued enum behavior.
Explicit modes remain intact; invalid values, null, and a missing image are still
rejected. OpenAPI marks the field optional with its default. No frontend change
or database migration is needed.

Coverage extends the .NET `DeploymentCreateTests` external-image creation and
persistence scenario with the exact omitted-field shape: POST succeeds, the
database stores `Disabled`, and GET returns the saved image/network configuration.
Deployment model tests cover omission and explicit/invalid modes; an OpenAPI
generator test guards the optional-field contract.

## Outcome

Phase 6A is implemented. Rust now owns the existing Deployment
list/detail/config/create/update/metadata/rename/duplicate/delete HTTP
contracts without changing the current React client or PostgreSQL schema.
Deployment Apply, progress streaming, runtime reconciliation, update checks,
and auto-update execution are deliberately not exposed by this slice.

The new `citadel-deployments` crate owns the Deployment model, validation, use
cases, and persistence/runtime/notifier ports. SQLx, generated Docker transport,
signed Agent transport, and the HTTP host remain in their existing adapter and
server crates. No new process, generic mediator, or dynamic handler registry was
introduced.

## Implemented behavior

### Compatible contracts and storage

- The nine existing Deployment CRUD/config routes retain their methods, paths,
  operation IDs, success shapes, and Problem Details failures.
- Browser payloads remain camelCase with the existing `$type` image
  discriminator. An explicit storage DTO writes PascalCase properties so Rust
  can read and update the current .NET rows without rewriting label keys.
- Config updates implement JSON Merge Patch rather than requiring a complete
  replacement spec. Omitted nested values remain unchanged, explicit removals
  retain the existing optional-field semantics, and enum input accepts the
  case variants exercised by the .NET API tests.
- API responses omit null optional properties, matching the .NET host's JSON
  options. Internal optimistic-concurrency state is never serialized.
- New rows retain Npgsql's `-infinity` representation for the .NET
  `DateTime.MinValue` auto-update sentinel; reads normalize it to a finite
  year-1 API timestamp that SQLx can decode safely.
- Create strips resolved/applied image provenance. Update preserves build
  provenance only when the same Build Project remains selected.
- Validation retains name, description, image/update-mode, stop-timeout, and
  resource bounds. Explicit `null` Tag IDs are treated as an empty collection,
  matching the published contract.
- Enabling external-image auto-deploy or build-triggered redeploy requires the
  same Operational Guardrails or Automated Operations entitlement as .NET.

### Authorization and transactions

- Collection ACL filtering happens in PostgreSQL for the requesting Actor and
  enabled Teams. Per-row capabilities are computed in the same query; no
  per-row permission query is added.
- Create validates the target Standalone Docker Platform and every Tag before
  committing the Deployment, Tag links, copied resource bindings, and typed
  Created/Duplicated Activity as one transaction.
- A Swarm target retains the current API's not-found response while Swarm
  Deployments remain unavailable; other unsupported Platform kinds return a
  validation failure.
- Binding duplication requires the Resource Bindings capability whenever the
  source contains bindings, matching the .NET policy.
- Config, metadata, and rename mutations lock the target row, enforce its
  concrete ACL and target-Platform access, reject updates while another
  operation owns the resource, update its row version, and commit compatible
  Activity evidence atomically. Config writes also compare the row version
  loaded for the merge so concurrent requests cannot silently overwrite each
  other.
- Batch delete resolves every ID and Execute permission before claiming any
  row, preventing partially authorized requests from starting Docker work.

### Safe deletion and transport

- A delete claim marks the entire batch Processing in one transaction and
  captures every linked Docker container, not only the most recent projection.
- The post-claim operation is independent of request cancellation, bounded to
  30 seconds, and tied to application shutdown. Local Docker uses the generated
  API 1.49 delete operation; Agent uses the existing signed gRPC contract.
- HTTP/gRPC mutations are not retried after ambiguous transport failures.
  Docker/Agent Not Found is accepted so interrupted deletion remains
  idempotently retryable.
- Runtime failure, timeout, or completion failure restores the claimed
  Deployment status/control state. A claim-release failure is returned rather
  than hidden.
- Successful completion verifies the claim row versions, writes Deleted
  Activities, removes Tag/binding/ACL links and vanished container projections,
  and deletes the Deployments in one transaction before publishing realtime
  invalidations.

Process-crash recovery for an already persisted operation claim moves with
Phase 6B reconciliation. During coexistence, the existing .NET reconciliation
job remains the recovery owner; Rust does not introduce an ambiguous generic
stale-state reset that could interfere with Apply.

## Test mapping from the .NET reference

| .NET reference area | Rust proof |
| --- | --- |
| `DeploymentCreateTests` (4), CRUD-relevant workload Platform authorization, and Tag integration tests | `citadel-deployments` unit tests, `deployment_persistence.rs`, and `deployments_http.rs` cover External Notify success, unsupported image and pinned-digest failures, duplicate-name conflict, inaccessible/non-Standalone Platforms, Tag integrity, compatible JSON, authorization, and persisted creation. |
| `DeploymentViewTests` (4) and CRUD-relevant `DeploymentRepositoryTests` (2) | The PostgreSQL/HTTP suites cover forbidden creation, cross-team isolation, direct and enabled-Team ACLs, exact Platform/Tag filters, capabilities, and configured UUID versus unresolved non-UUID Local-image projections without N+1 permission queries. |
| `DeploymentPatchTests` (9) | The HTTP and adapter suites cover full and partial merge patches, lowercase enum input, default restart policy, immutable Platform ownership, rejection of non-config/non-object/unsupported/pinned input, metadata clearing/update, rename, inaccessible Platform no-write behavior, operation-state exclusion, stale row-version conflict, and Activity persistence. |
| `DeploymentDuplicateTests` (4) | The PostgreSQL/HTTP suites cover `-copy`/`-copy-2` naming, provenance clearing, host-bind warnings, Tag/binding copies, authoritative source identity, invalid source type, missing source, no partial insert, and atomic duplicate Activity. |
| `DeploymentDeleteTests` (2), `DeleteDeploymentsTests` (2), and CRUD-relevant cancellation behavior | Unit, PostgreSQL, and HTTP tests cover whole-batch claiming, deletion with every linked container projection, request-independent recovery after the caller disappears, bounded timeout, runtime failure rollback, concurrent claim exclusion, mutation exclusion during a claim, projection cleanup, and Deleted Activities. |
| `DeploymentLicenseConfigurationPolicyTests` (3) | Unit tests preserve the exact expansion policy: unrelated Build changes and disabling redeploy require no new entitlement, while changing an enabled Build target does. Create-time Operational Guardrails enforcement remains covered as well. |
| Docker/Agent mutation compatibility | `docker_transport.rs` verifies the versioned Unix-socket request; `agent_mutations.rs` verifies the signed `ContainerService/Delete` payload and no retry on ambiguous mutation failure. |
| CRUD-relevant SignalR Deployment serialization | Domain serialization tests verify canonical image and Activity discriminators, omitted nulls, and hidden concurrency metadata; generated OpenAPI verification preserves the frontend contract, and successful mutations publish metadata-free invalidations. |

There is no Deployment CRUD acceptance test in the current .NET acceptance
project to port. Its acceptance suite starts at broader candidate/runtime
workflows. Phase 6A therefore uses the real Axum + PostgreSQL CRUD lifecycle as
the highest existing applicable test level; Apply and runtime acceptance move
with Phase 6B rather than being falsely claimed here.

Apply/reconciliation, image update checks, container state actions, logs,
terminal, stats, and adoption tests are mapped to later Phase 6 slices because
their runtime behavior is not registered in Phase 6A.

## Verification

Passed locally on 2026-09-03:

- `cargo fmt --all -- --check`
- `cargo check --locked --workspace`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked -j 1 --workspace --no-fail-fast`
- `cargo test -p citadel-deployments`
- `cargo test -p citadel-adapters --test agent_mutations`
- `scripts/Test-Phase6ADeployments.ps1`, including real disposable PostgreSQL,
  authorized Axum lifecycle, Linux Unix-socket Docker transport, persisted
  claim rollback, Agent equivalence, and generated contract checks
- `cargo run -p xtask -- openapi --check` (168 full, 116 public routes)
- `cargo run -p xtask -- docker --check`
- frontend `npm run lint`
- frontend `npm run test:run` (100 files, 340 tests)

After completing the exhaustive CRUD/config mapping, the same day gate was
rerun with 18 Deployment unit tests, the disposable PostgreSQL adapter test,
the real Axum HTTP contract test, workspace-wide Clippy with warnings denied,
the complete Rust workspace suite, and all 340 frontend unit tests passing.

The disposable gate completed successfully and removed its PostgreSQL
container and Docker network.

## Next slice

Phase 6B owns Deployment Apply as a durable operation: variable/Secret
resolution and redaction, bounded progress streaming, Docker acceptance versus
runtime success, cancellation, restart recovery, event-driven reconciliation,
and authoritative success/failure Activity. CRUD does not imply that Apply is
ready.
