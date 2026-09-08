# Phase 7.5 implementation evidence

Status: **in progress, not application-parity complete** (2026-09-08).
The authoritative phase plan is in
`Citadel.Internals/specs/dotnet-to-rust-phase-7-5-api-parity.md`.

## Contract inventory

The original audit identified 65 missing .NET operation IDs. The implementation
now includes all 65 of those IDs, including the previously uncataloged webhook.
The current catalog has 404 operations: all 401 shared with .NET and three
Rust-only health/readiness/metrics operations. **No .NET operation IDs remain
missing**. Public/internal exposure matches; 42
previously misclassified routes were corrected without changing authentication.

`cargo run -p xtask -- parity` checks all reference operation IDs, method/path
pairs and public exposure and now passes. The generated-subset compatibility
check is not a full parity gate, and neither check proves live behavior.

## Implemented operation IDs and scenario coverage

| Operations | Rust evidence | .NET scenario reference / additional coverage |
| --- | --- | --- |
| `getActor`, `patchActorEnabled` | `server/tests/users_http/actors.rs` | Actor endpoint read/enabled-state; human-admin boundary; Actor IDs versus User IDs; malformed requests; last-admin rollback; concurrent administrator disables |
| `listServiceAccountUsages` | `server/tests/service_accounts_http.rs` | Run-as Actor references in Automations/Backup Policies, enabled/archived state, unrelated Actor exclusion, authorization and missing Account |
| `getDeploymentTags`, `replaceDeploymentTags`, `getStackTags`, `replaceStackTags`, `getSwarmServiceTags`, `replaceSwarmServiceTags` | `server/tests/resources_http/workload_tags.rs` | ResourceTagIntegrationTests replacement/read/removal semantics; real persisted joins; resource-specific Read/Write; invalid Tag rollback |
| `getAlertRuleConfig`, `renameAlertRule`, `updateAlertRuleMetadata` | `server/tests/phase7_resources_http/alert_rule_metadata.rs` | Config projection, rename Activity, description missing/null/value, preservation of Conditions/Channels, denied writes and missing Rule |
| `globalSearch` | `server/tests/platforms_http/search.rs` | GlobalSearchTests ranking, literal LIKE characters, per-type limit, validation, direct/Team roles, resource grants, parent redaction, managed Swarm Services |
| `receiveWebhook` | Existing `resources_http` and `phase7_resources_http/*_webhooks.rs` tests | Existing execution is registered through typed contract metadata, not duplicated. Fixture authentication/dispatch tests remain in place |
| `getContainerInfo`, `getContainerData`, `getDeploymentContainerInfo`, `inspectDeployment` | `server/tests/platforms_http/container_inspection.rs` | .NET summary/inspection shapes; persisted and Docker IDs; Read versus Inspect; Deployment authorization without a Platform grant; environment redaction; exact-node routing and stale/missing identity |
| `startDeployments`, `stopDeployments`, `restartDeployments`, `pauseDeployments`, `resumeDeployments` | `server/tests/platforms_http/container_mutations.rs` | Deployment state commands; Deployment-specific grants; actual DB claims and observed state; invalid/missing batches; no Swarm task control through Deployment commands |
| `getSwarmOverview` | `platforms/src/swarm_overview.rs`, `server/tests/platforms_http/swarm_overview.rs` | Majority/leader/stale/offline quorum semantics; raw-Service status counts; system-Service exclusion; capabilities and Platform scope |
| `inspectSwarmTask`, `getSwarmTaskTerminalTarget` | `server/tests/platforms_http/task_runtime.rs` | SwarmEndpointTests runtime permissions; current Task identity; owning-node Edge inspection; Container-view redaction; stopped/replaced/offline targets rejected |
| `updateSwarmServiceMetadata` | `server/tests/swarm_services_http/metadata.rs` | Required description, null clearing, 600-character limit, forbidden/processing writes, specification preservation, row version and atomic Activity |
| `getBackupRunEvents`, `getBackupRestoreRunEvents` | `server/tests/phase7_resources_http.rs` | Existing .NET empty events response (`runId`, `events`); persisted Run/Restore lookup and parent Policy authorization. These routes do not replace execution logs or progress streams |
| `renameBackupPolicy`, `updateBackupPolicyMetadata` | `server/tests/phase7_resources_http/backup_policy_metadata.rs`, `backups/src/policy_metadata.rs` | BackupEndpointTests rename/metadata persistence; validation; concurrent name/description edits; duplicate-name rollback with no success Activity; missing/denied resources; typed old/new snapshots excluding webhook credentials |
| `updateBackupRepository` | `server/tests/phase7_resources_http/backup_repository_patch.rs`, `backups/src/repository_patch.rs` | BackupEndpointTests description-only patch; BackupEntitiesTests Ready-location immutability and S3 normalization; omitted/null fields; immutable name/password; failed patch rollback and authorization |
| `getPlatformBackupSummaries` | `server/tests/phase7_resources_http/backup_summaries.rs` | BackupRepositoryTests source-type counts and latest failed-run status; SQL authorization, archived-policy exclusion, duplicate/empty Platform selection, query bounds, and resource-scoped visibility |
| `rotateAgentHubKey`, `updatePlatform`, `renamePlatform`, `prunePlatform`, `pullImage` | `server/tests/platforms_http/platform_image_management.rs`, `adapters/src/agent_key_rotation_tests.rs`, `adapters/tests/agent_transport.rs`, `adapters/tests/agent_build_pool.rs` | Administrator and resource permissions; persisted edits and duplicate-address conflicts; bounded prune and pull streams; partial failure; durable signing-key rotation observed by existing client clones |
| `getExternalRepositories`, `getGhcrPackageVersions`, `getDockerHubRepositories`, `getDockerHubRepositoryTags` | `server/tests/platforms_http/registry_browsing.rs` | Registry visibility; provider request/response contracts; pagination bounds; credential redaction and remote errors |
| `updateSwarmNode`, `inspectSwarmNode`, `updateSwarmNodesAvailability`, `deleteSwarmInventoryServices`, `inspectSwarmService`, `restartSwarmService`, `createSwarmSecret`, `deleteSwarmSecrets`, `updateSwarmSecretLabels`, `createSwarmConfig`, `deleteSwarmConfigs`, `getSwarmConfigData`, `updateSwarmConfigLabels` | `server/tests/platforms_http/swarm_inventory.rs`, `adapters/tests/swarm_inventory_transport.rs` | Swarm endpoint permissions, exact cluster/node/daemon identity, version conflicts, system ownership, complete batch preflight, in-use protection, partial-delete reconciliation, immutable Config/Secret data, signed Agent/Edge equivalence and no ambiguous mutation retries |
| `getSwarmServiceAdoptionDraft`, `adoptSwarmService`, `getSwarmServiceDuplicateDraft`, `inspectManagedSwarmService` | `server/tests/platforms_http/service_adoption.rs`, `adapters/src/swarm_service_inspection.rs` | ManagedSwarmServiceEndpointTests adoption without Docker mutation, orphaned ownership reclamation, stack ownership rejection, stale/tampered previews, concurrent adoption, source-registry authorization, redaction, pending desired changes, duplicate bindings/tags and atomic Activities |
| `updateBackupPolicy`, `getDeploymentBackupSourcePreview`, `getStackBackupSourcePreview`, `getSwarmServiceBackupSourcePreview`, `runBackupPolicy`, `runBackupRestoreVolume` | `server/tests/phase7_resources_http/backup_completion.rs`, `backups/src/policy_update.rs`, `backups/src/progress.rs`, `adapters/src/backup_executor.rs` | Policy validation and optimistic updates; first-success source immutability; node-qualified previews and stale coverage; durable backup/restore progress, disconnect versus cancellation, committed terminal state, split-line credential redaction and bounded output |

The scenario mappings describe the tests actually added or reused, not a claim
that every test in the referenced .NET class has been ported. Existing lifecycle
test files exercise multiple operations within one test case.

## Safety and implementation details

- No React source changes, migration changes, new binaries, or command bus.
- Search retains the .NET SQL permission/ranking logic, limits each candidate
  set, and returns at most 40 rows. Swarm overview aggregates in SQL rather
  than loading inventories into application memory. These are structural
  bounds, **not measured performance improvements**.
- Deployment controls reuse existing bounded Container execution, transactional
  claims and recovery. Invalid/ambiguous Deployment-to-Container mappings fail
  before Docker commands; resolution reads at most selection size plus one.
- Task inspection uses existing Local/Agent/Edge Container inspection and
  redaction. One deadline bounds manager and node reads; dropping the request
  cancels the operation. Terminal target lookup does not create an exec session.
- Administrator enables/disables share the identity mutation lock, so competing
  operations cannot disable the last enabled administrator.
- Metadata changes preserve workload configuration; Service metadata rejects
  an active operation and commits its Activity with the description change.
- Backup Policy header edits use column-specific updates and transactional
  Activities. Repository edits lock the row before checking Ready-location
  immutability and reject location changes during an active operation. Validate/
  Initialize acquire the same row lock before taking their lease and read the
  location after acquisition, closing the stale-location/Ready-status race.
  Platform Backup summaries aggregate authorized policies in one
  SQL query and accept at most 256 requested Platform IDs.
- Test identities for cluster IDs, system Service/Secret IDs and Docker log
  targets are unique, preventing repeated-run collisions or ambiguous lookup.
- Service adoption locks the projected Docker identity before claiming it and
  commits ownership, desired configuration, Tags and Activity atomically. It
  does not apply the reviewed configuration or claim an applied image digest.
  Duplicate configuration copies only authorized bindings, not runtime identity
  or image provenance.
- Swarm mutations refresh persisted projections after successful changes,
  including successful siblings of a failed batch. Older snapshots cannot
  overwrite newer observations. Config edits preserve immutable Data; Secret
  reads and audit snapshots do not return its payload.
- Backup progress observes the existing durable execution worker rather than
  starting a second executor. Disconnecting the sheet leaves the job running;
  explicit cancellation reports committed cancellation. Output channels and
  line buffers are bounded, and credentials are redacted before publication.

## Verification and remaining gates

Executed against disposable PostgreSQL and Linux fixtures, not user workloads:

The final combined server/adapter run passed **241 tests** (none ignored in
that run), covering the library and integration targets listed below. The
disposable test containers/database were removed afterward; application volumes
and shared Cargo caches were retained.

- Platform HTTP suite: 61 tests passed, including Edge command/WS fixtures,
  all new native inventory operations, adoption/duplication and registry ACLs.
- Managed Service HTTP lifecycle, including metadata: 1 passed.
- Phase 7 resource HTTP lifecycle: 1 passed, including all six new Backup
  operations and cancellation/disconnect tests.
- Server library unit suite: 58 passed, including pull completion and Docker
  Hub alias regressions.
- Adapter library unit suite: 109 passed, including signing-key rotation,
  case-sensitive image references, native Service inspection and bounded
  backup output framing/redaction. Regular-Agent transport/build-pool suites:
  9 passed. Native Swarm signed-Agent/Edge transport-equivalence suite: 2 passed.
- Domain, Identity, Resources, Alerts, Platforms, Contracts, Backup, managed
  Service and xtask unit suites: passed. Backup alone has 10 unit tests.
- Earlier checkpoint evidence: User HTTP suite (28), Service Account HTTP
  suite (6), and resource HTTP lifecycle (1) passed in their scoped runs.
- `cargo clippy -p citadel-server -p citadel-adapters -p xtask --all-targets
  -- -D warnings`: passed.
- OpenAPI generation and drift check: 404 full / 305 public operations.
- Full reference catalog parity: all 401 .NET operation IDs, methods, paths and
  public exposure match. Generator tests also reject dangling schema references.
- Pinned Docker API generation check: passed.

All remaining 32 operations are implemented and have targeted scenario ports.
Still required for full phase closure: full composed-router and request/response
parity beyond catalog checks, live external-service and Local/Agent/Edge/multi-node
acceptance, and the later Phase 8 release/memory/soak evidence. Native Unix Docker
API fixtures and signed transport fixtures are not live multi-node acceptance.
The unchanged frontend regression suite passed: 108 files, 397 tests, run with
`node node_modules/vitest/vitest.mjs run --maxWorkers=2 --no-file-parallelism`.

An additional behavioral issue remains outside the original missing-ID count:
the existing Alert Rule full-view Channel projection and partial-update shape
need comparison with the .NET contract. Catalog presence alone cannot detect it.

Existing limitations are not lifted by these endpoint additions: regular-Agent
HTTPS and multiple simultaneous regular-Agent live subscriptions still require
separate work; rotating the Core signing key requires updating Agent trust.
Deployment-on-Swarm backup execution still has its existing placement restriction.
The new native Swarm Stack/Service previews do not remove that restriction.

To rerun from `rust/`, provide disposable database URLs for the test phases:

```sh
cargo test -p citadel-server --test platforms_http -- --ignored --test-threads=1
cargo test -p citadel-server --test users_http --test service_accounts_http \
  --test resources_http --test phase7_resources_http --test swarm_services_http \
  -- --ignored --test-threads=1
cargo test -p xtask -p citadel-contracts -p citadel-identity -p citadel-resources \
  -p citadel-alerts -p citadel-platforms -p citadel-backups --lib --bins
cargo test -p citadel-adapters --lib --test agent_transport \
  --test agent_build_pool --test swarm_inventory_transport
cargo run -p xtask -- openapi --check
cargo run -p xtask -- parity
```

Set `CITADEL_PHASE3_DATABASE_URL`, `CITADEL_PHASE4_DATABASE_URL`,
`CITADEL_PHASE5_DATABASE_URL`, `CITADEL_PHASE6_DATABASE_URL` and
`CITADEL_PHASE7_DATABASE_URL` before running the ignored integration tests.
Do not point these fixtures at an application database.
