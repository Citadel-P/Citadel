# Authorization policies — Phases 2 and 6

`citadel-domain::authorization` temporarily owns `PermissionRequirement`,
`PermissionPolicy`, `SpecificPermissions` and `EffectivePermission`, alongside the
existing enums/ActorId. The complete primitive bundle moves once in Phase 11.
`IdentityService::require_resource<P>` and `require_scope<P>` reuse the existing
identity resolver; no second ACL engine, dependency, framework or request coupling
was introduced. Deployment markers live in `deployments/src/permissions.rs`.

`SpecificPermissions` is a small set with membership, union and bit conversion;
unknown persisted bits retain their bit pattern. A private `u32` needs no bitflags
operators, serialization or new dependency. PermissionLevel stays an ordered enum
with persisted values 0/1/2/4. SQL binds its tested `accepted_database_levels()` array
rather than treating hierarchy values as flags. Invalid levels are never valid enum
values. Effective administrator access is explicit, not level 7 / specifics 63.

## Deployment parity and enforcement

R = Read, W = Write, X = Execute. Scope checks use enabled-actor global roles;
resource checks use direct/team roles and resource ACLs. Human administrator resource
bypass is centralized in IdentityService; service accounts do not acquire it merely
by carrying an Admin role name. Scope authorization keeps the existing enabled-actor
snapshot behavior. Authentication remains responsible for producing a valid principal.

| Operation / ID source | Previous Rust | .NET / intended | Authoritative / additional checks |
|---|---|---|---|
| Get, config, duplicate draft / path | R | R | ACL-scoped query; duplication validates source bindings |
| List / actor scope, filter | SQL R | SQL R | SQL row filtering plus projected capabilities, no per-row authorization |
| Create / type scope | W | W | Platform read and image/reference checks; duplicate source and destination binding access remain explicit |
| Patch config/metadata, rename / path or body | W | W | Existing transactional access check; config retains Platform checks |
| Delete / body ID collection | X per selected ID | X | Existing atomic persistence checks; no added per-ID HTTP queries |
| Apply / body ID | **X + Apply in HTTP and claim** | **R + Apply** | Both now consume `ApplyDeployment`; transactional claim rechecks independently |
| Check updates / path | W | W | Existing transactional Write check and version claim |
| Container adoption draft/adopt / container path | Deployment scope W | W | Existing Platform read/inspect and binding/reference checks remain |
| Container info, statistics / Deployment path | R | R | Existing container selection and Platform/inventory checks remain |
| Inspect / Deployment path | R + Inspect | R + Inspect | Typed precheck before runtime inspection |
| Start logs / realtime argument | R + Logs | R + Logs | Typed precheck before log selection/start; container checks remain |
| Shell start/input/resize / realtime argument | R + Terminal | R + Terminal | Typed precheck before terminal operation; container/session checks remain |

Evidence: .NET declarations in `src/Citadel.Application/Features.Deployments/{Commands,Queries}`,
Rust `identity::permission_matrix()`, Deployment HTTP/transactional integration tests,
and `api::deployments::capabilities` parity tests.

Apply was the behavioral mismatch: Read+Apply now succeeds without Execute at both
boundaries. Missing/forbidden responses and pre-stream denial remain unchanged.
Capability mapping now enforces the same base level and specific flag. A second
inconsistency was found during parity testing: the legacy `canPull` wire field exists,
but Deployment Pull is absent from both the permission matrix and exposed operations.
Its current bit-based presentation is preserved; no artificial Pull policy was added.

## Query budget

Excluding authentication/connection setup and runtime work:

| Operation | Before | After | Role of each query |
|---|---:|---:|---|
| Non-admin get | 2 | 2 | Resource permission precheck + ACL-filtered projection |
| Non-admin list | 3 | 3 | Enabled actor + global permissions for collection capabilities + one ACL-filtered collection projection |
| Admin get/list | 1 | 1 | Existing bypass + scoped projection |
| Apply authorization | 2 | 2 | One inbound permission precheck + one in-transaction effective-permission query; resource locking/claim/result writes are additional unchanged operations |

Phase 6 retains the existing read precheck for error parity. List never calls the authorizer per resource. The HTTP
integration test checks get/list SQL counts when pg_stat_statements is installed and
verifies Apply grant, denial-before-runtime, and revocation after a successful precheck.
The measured collection test includes three authorized rows and still executes three
SQL statements total; the get executes two.

CI runs the HTTP authorization fixture, shared .NET ACL matrix, and Apply/binding
persistence tests against an isolated PostgreSQL container. It enables
pg_stat_statements and sets `CITADEL_REQUIRE_QUERY_COUNTS=1`, so missing query-count
instrumentation fails the fixture instead of silently skipping those assertions.
The container and its volumes are removed when the step exits.

## Deferred boundaries

Deployment View/capability mapping now resides in `server/src/api/deployments/`.
`PostgresDeploymentRepository` returns semantic projections with typed effective grants.
The feature has no HTTP schemas; the architectural guard has no Deployment legacy
exemption. The Platform projection retains one explicit typed-set-to-legacy-mask bridge,
removed in Phase 10.

Other feature authorization declarations remain unchanged: Stacks/Swarm (7),
Builds/Git/Backups (8), Automation/Alerts (9), Platforms/Identity/shared resources (10),
activities/umbrella ownership (11). Shared container/realtime fallback checks, binding
and tag permissions remain explicit; they are not replaced by Deployment-only policies.

Existing SQL mask sites to migrate in their owning phase: `build_store`, `backup_store`,
`backup_source_planner/preview`, `backup_platform_summaries.sql`, `git_account_store` (8);
`automation_store`, `alert_store` (9); `platform_read_store`, `resource_metadata_store`,
`lookup_store` (10); `activity_store` (11). Stacks/Swarm also retain their legacy
capability integer representations (7). No broad exemption was added to the guard.

The Rust `deployment_authorization_conventions` test prevents raw tuple declarations
returning to the migrated Deployment handler family, including scoped Deployment
inspection/statistics and realtime logs/terminal entry points in shared modules.
Those scopes must retain their named policy calls. The guard also prevents presentation
and direct task spawning from returning to Deployment feature/persistence modules.

## Historical Phase 2 validation

At Phase 2 completion, Phase 3 was unstarted. The initial `cargo test --locked --workspace`
passes: **613 passed, 232 ignored**. Seven targeted PostgreSQL integration tests also
pass: Deployment HTTP/authorization/query counts, the shared .NET ACL matrix, two
Apply/binding persistence tests, and Deployment inspection, statistics and logs.
OpenAPI verification passes (404 full / 305 public operations), and all seven
baseline contract hashes remain unchanged. The disposable database was removed.

Review follow-up: all four convention tests and the exact CI database command
(four integration tests, including the measured query counts) passed locally.

`cargo fmt --all -- --check` still reports preexisting formatting drift; none is in
a file changed here. Strict workspace Clippy still stops on existing `collapsible_if`
findings in Automation and Stacks. A supplemental pass allowing that lint found six
more existing findings in unchanged files: `agent.rs` (`needless_update`),
`container_stats_store.rs` and `resource_status_store.rs` (`items_after_test_module`),
`swarm_operation_reconciliation.rs` (`cloned_ref_to_slice_refs`), and two
`too_many_arguments` findings in `server/src/workers/platforms.rs`. No lint policy
was weakened in source or CI. `git diff --check` passes.

The earlier generated audit files and Python scripts were moved outside the repository;
the source change keeps only Rust implementation/tests and these human-readable guides.

Current Phase 6 validation and compatibility results are recorded in
[the Deployment refactor report](reports/phase6-deployment-resource-refactor.md).
