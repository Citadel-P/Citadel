# Phase 7.5 test-port execution — 2026-09-08

Status: initial high-risk scenarios implemented and executed; **full .NET test
parity is still open**. The external Phase 7.5 inventory remains the baseline,
not a claim that its hundreds of unmapped scenarios are now complete.

Reconciliation follow-up: [current method-level closure ledger](../../../Citadel.Internals/specs/dotnet-to-rust-phase-7-5-test-closure.md).
Its assertion-reviewed statuses supersede any interpretation of this report's
individual passing assertions as whole-method parity. The latest follow-up
fixes quiet-hour Windows aliases and setup middleware behavior, and verifies
the remaining SetupEndpointTests assertions after the Alert contract fixes.
The ledger now separates 38 verified, 13 partial, 0 classified divergent, 1,638
unreviewed, 85 deferred and 7 inapplicable .NET method placements; these are not
Rust runner totals. New targeted execution is recorded in the latest section below.

No frontend source or database migration was changed. Existing staged changes
were preserved; this work was not staged.

## Mappings implemented in this pass

| .NET source scenario | Rust evidence | Scope/status |
| --- | --- | --- |
| `PreReleaseUpgradeTests.FailingMigration_ShouldRollbackSchemaAndRemainUnjournaled` | `crates/database/tests/parity/mod.rs::failed_baseline_rolls_back_schema_preserves_data_and_can_retry` | Verified against real PostgreSQL. A deliberate collision partway through the real embedded baseline rolls back earlier DDL, preserves existing data, leaves no **completed** journal entry and permits retry/restart. Rust intentionally retains an incomplete diagnostic row; DbUp's absence of any journal row is not copied. No synthetic production migration/catalog entry was added. |
| `PreReleaseUpgradeTests.CandidateMigration_ShouldEnforceSingleActiveActionRunPerAction` | `database_enforces_active_run_state_matrix_and_allows_terminal_history`; `concurrent_direct_inserts_allow_only_one_active_run_per_action` | Verified against the generated schema. All Queued/Running collision pairs assert SQLSTATE 23505 and the exact unique index. Terminal history and requeue are permitted; concurrent direct inserts allow only one winner; different Actions remain independent. |
| `ControlPlaneRecoveryTests.Candidate_ShouldRestoreControlPlaneAndOperateInCleanEnvironment` | `crates/adapters/tests/citadel_system_recovery.rs::system_bundle_restores_state_into_a_clean_database` | Expanded and verified with real `pg_dump`/`pg_restore`: Tag data, manifest/DB instance identity, persisted encrypted Secret, decryption with a freshly constructed protector using the retained external key, and wrong-key rejection. **Partial overall port**: packaged Core startup/login/workload operation is not exercised by this adapter test. |
| `AlertRulePatchTests.Patch_AlertRule_Should_Apply_MergePatch` | `crates/server/tests/phase7_resources_http/alert_rule_patch.rs::verify`, invoked by `phase7_resource_endpoints_authorize_validate_and_persist_lifecycles` | Verified HTTP + PostgreSQL partial configuration persistence, omitted field preservation, and successful/rejected realtime notifications. The .NET-specific cache implementation is not copied. |
| `AlertRulePatchTests.Patch_AlertRule_With_Invalid_CooldownSeconds_Should_Return_BadRequest` | Same HTTP helper | Verified 400 and unchanged persisted/read-back configuration and Channels; also tests invalid severity, missing Channel, non-object patch and no change notification. |
| `AlertRulePatchTests.Patch_AlertRule_Should_Update_Channels` | Same HTTP helper | Verified replacement, clearing and restoration of Channel links in both response and PostgreSQL. |
| `AlertRulePatchTests.Patch_NonExistent_AlertRule_Should_Return_NotFound` | Same HTTP helper | Verified valid partial/empty payload reaches resource lookup and returns 404 rather than failing full-create deserialization. |
| `AlertRulePatchTests.Patch_AlertChannel_Should_Apply_MergePatch` | Same HTTP helper | Verified URL/active-state update with preserved name/destination, response and persisted values. |
| `AlertRulePatchTests.Patch_NonExistent_AlertChannel_Should_Return_NotFound` | Same HTTP helper | Verified 404 for missing Channel. |
| Additional regression coverage | `crates/alerts/src/configuration_patch.rs` tests; HTTP helper concurrent PATCH requests | Verified missing vs explicit-null optional fields, protected Rule metadata, malformed values, duplicate/nil Channel IDs, anonymous/unauthorized callers and disjoint concurrent updates. Rule/channel reads and merges occur under the DB row lock. |
| Additional recovery regressions | `crates/adapters/src/citadel_system_backup/parity_tests.rs` | Verified exactly-once URL decoding, literal plus signs, decoded-NUL rejection, credentials absent from arguments and safe libpq database-name quoting. |
| Generated-contract regression | `xtask/src/openapi_gen.rs::alert_rule_patch_schema_does_not_require_a_full_create_payload` | Verified Rule PATCH schema is distinct from create requirements, while retaining the existing frontend operation/schema identities. |

## Failures reproduced and fixed

| Severity | Evidence before fix | Minimal fix / verification |
| --- | --- | --- |
| High — recovery identity consistency | The expanded live recovery test restored `None` for the instance ID while the manifest contained a UUID. The builder created the lazy identity **after** taking the database dump. | Commit/read instance identity before `pg_dump`. The same live assertion now passes on a fresh source/target pair. This does not yet certify the whole packaged license/recovery workflow. |
| Medium — backup credentials/target names | Unit regressions failed because URL-escaped user/password/database values reached libpq without decoding; decoded NUL was not rejected. | Decode components once, reject NUL/invalid UTF-8 without echoing credentials, and quote the database as a libpq conninfo value so embedded `=`/quotes cannot override other connection parameters. Unit and real restore checks pass. |
| High — Alert partial-update compatibility | The .NET-shaped partial PATCH failed at the JSON extractor (422 in the router fixture) before reaching expected authorization/persistence behavior. The handlers bound full create inputs. | Bind the patch document, authorize, lock/read current state, apply only allowed fields, validate and commit atomically. HTTP tests now pass, including concurrent disjoint updates and failure/no-notification checks. |
| Low — brittle migration tests | Existing unit test expected 83 tables; the generated baseline now contains 84. The old integration count would also have been stale. | Compare exact table-name sets against the declarative schema plus the Rust journal, instead of updating another magic count. Manifest/checksum and seed checks remain intact. |

These are reproduced test failures, not allocation-based guesses. No speculative
optimization or frontend workaround was introduced.

## Executed verification

Commands ran in the existing Linux development workspace, with disposable
PostgreSQL fixtures on its network. They did not use the development/product
database. Image: `postgres@sha256:4ef4dbc939d61acea57712655ddb4b4ab27419c913f94cca0cd57cb3ea3c2280`.

| Command / fixture | Result |
| --- | --- |
| `cargo test --locked -p citadel-database -- --include-ignored` with dedicated `CITADEL_TEST_DATABASE_URL` | 2 unit + 4 PostgreSQL integration tests passed; none ignored in this run. |
| `cargo test --locked --workspace --lib` | 415 passed, zero failed/ignored. Includes the new patch and recovery command regressions. |
| `cargo test --locked --workspace --jobs 2` | 509 passed, zero failed, 184 ignored across the workspace, integration targets and doc tests. Ignored environment-dependent tests are not counted as passing. |
| `cargo test --locked -p citadel-server --test phase7_resources_http -- --ignored --nocapture` with a fresh `CITADEL_PHASE7_DATABASE_URL` | One combined HTTP/PostgreSQL lifecycle test passed, including the added Alert helper scenarios. This is not counted as dozens of separately discovered Rust tests. |
| `citadel_system_recovery` compiled test executable, `--ignored --nocapture`, inside the matching PostgreSQL image | The expanded live recovery test passed twice against fresh source/target pairs, including database names containing spaces and `=`. Both runs used the image's unprivileged `postgres` user. This remains one test scenario, not two ports. |
| `cargo test --locked -p xtask` | 11 passed after preserving the existing Alert Channel schema identity. No frontend-contract test was disabled or relaxed. |
| Final `cargo test --locked -p citadel-alerts -p xtask` | 8 Alert + 11 generator/contract tests passed after the null-collection adjustment. |
| `xtask openapi` followed by `xtask openapi --check` | Regenerated and verified 404 full / 305 public HTTP contracts. |
| `cargo run --locked -p xtask -- parity` | All 401 .NET operation IDs, methods, paths and public exposure match. This is not a behavioral-parity result. |

The generator initially encountered existing generated-file permissions under
the unprivileged devcontainer user. Its compiled executable was rerun as the
container user owning those generated artifacts; no broad permission change was
made. An attempted new Channel request-schema name was rejected by the frontend
contract gate and was not retained. The existing Channel documentation still
shares its create input schema; its runtime partial-update behavior is verified
here, but improving that legacy schema requires an explicit contract change.

The disposable PostgreSQL fixture and its temporary test data were removed after
verification. The existing development workspace and its build cache were kept.

## Follow-up implementation

The subsequent pass extends the assertion mappings below. It keeps the original
inventory as a historical baseline rather than treating source-name matches as
completed ports.

| .NET source | Rust test / implementation | Preserved assertions and boundary |
| --- | --- | --- |
| `FirstRunSetupAcceptanceTests.PasswordFileBootstrap_ShouldInitializeOnceAndIgnoreOptionsAfterRestart` | `server/tests/bootstrap_process.rs::password_file_bootstrap_initializes_once_and_ignores_removed_file_after_restart` | Actual Core subprocess + PostgreSQL + HTTP login/profile; password-file setup, no automatically issued session, one Admin, two disabled default actions with tags, typed Unattended setup activity, restart after removing the file, successful subsequent login. Debug candidate, not release-image qualification. |
| `FirstRunSetupAcceptanceTests.PartialBootstrapConfiguration_ShouldFailAndLeaveSetupPending` | `bootstrap_process::partial_bootstrap_fails_before_serving_and_leaves_setup_pending_after_restart` | Actual failed Core startup, no user, persistent pending setup, restart without bootstrap and normal setup status. |
| `InitialAdministratorBootstrapServiceTests` password-file cases | `server/src/bootstrap.rs` unit tests | UTF-8/BOM, one LF/CRLF, preserved password spaces, 1 KiB limit, malformed UTF-8/multiple lines, directory/missing/relative-path rejection; errors do not echo secret content. |
| Setup endpoint concurrency and atomicity | `bootstrap_process::concurrent_interactive_setup_has_one_winner_and_one_set_of_disabled_defaults`; `default_action_persistence_failure_rolls_back_entire_setup` | Two concurrent real HTTP requests yield one success/one conflict, one set of defaults and Interactive activity. Missing required seed tag deliberately fails late in setup; user/actor/roles/actions and setup state roll back. |
| `ControlPlaneRecoveryTests.Restore_ShouldRejectDatabaseArchiveWithoutSecretEncryptionKey` | `bootstrap_process::offline_restore_rejects_missing_or_malformed_encryption_key_before_accessing_target`; expanded `adapters/tests/citadel_system_recovery.rs` | Rust external-key-custody equivalent: real CLI rejects absent/malformed key before bundle/DB access; adapter test with a valid bundle proves missing/short key leaves a sentinel row in the target unchanged. This checks presence/shape, **not** that a different valid-length key belongs to the backup. |
| `AlertRuleCreateTests` threshold/non-threshold/channel cases | `server/tests/phase7_resources_http/alert_rule_create.rs::verify` | Real HTTP + DB create/read-back, optional/null names default to type/destination, threshold values and channel counts persist; bad cooldown/missing threshold requirements/non-threshold threshold settings return 400 without Rule writes. Successful inputs explicitly send Enabled; omitted-status HTTP proof, exact created Channel identity and remaining original assertions stay open in the closure ledger. |
| `CreateAlertRule` / `PatchAlertRule` entitlement and activity requirements | Same HTTP helper; `alerts/src/configuration_patch.rs` | Unlicensed create/advanced edits/custom enable or channel additions return 403 and leave state/audit unchanged. Disabling/removing channels remains possible. Successful create/update writes typed activities atomically. Unit cases retain built-in enable/channel controls. |
| Additional activity synchronization regression | `server/src/realtime_groups/mod.rs::alert_rule_activity_follows_committed_alert_invalidations_only` | Alert Rule activity subscriptions recognize existing coarse Alert invalidations, still filter unrelated IDs/resources/statistics, and reload through the existing authorized reader. This routing unit test plus HTTP audit persistence is not a new full live-WebSocket acceptance claim. |
| `AlertService.ProcessAsync` license downgrade | `adapters/tests/alert_persistence.rs` | Reconstructed unlicensed store skips persisted custom rules without advancing their state, while built-in rules still evaluate. |
| Additional `AlertService.ProcessAsync` selection regression | `alert_persistence::matching_rules_choose_one_severity_winner_without_cooldown_fallback` | One observation persists only the highest-severity matching rule. Its cooldown suppresses a later incident without falling back to the lower rule; non-matches still reset/resolve their own state. The pre-fix test persisted both Critical and Warning events. |
| Six applicable `AlertRuleQuietHourTests` methods, including both DST theory rows | `alerts/src/quiet_hours.rs` | Summer/winter IANA offsets, full weekday names, previous-day ownership after midnight, overlap on following day but not early same day, invalid timezone rejection and unchanged public JSON. Added week-wrap/malformed schedule cases and HTTP invalid/overlap rejection. Windows timezone alias normalization is **not** ported: current Rust policy is IANA-only. |

Additional defects found by these ports:

- High: bootstrap settings were ignored by Rust startup. The real process test
  reproduced `requiresSetup: true` despite valid settings. Startup now reuses the
  locked setup transaction, and completed installations ignore removed files.
- Medium: a repeated process test exposed setup status reporting initialized
  while immediate login returned 503 after restart. The setup gate had waited
  for the first background probe. Core now loads committed setup readiness before
  listeners open; the test still performs one immediate login, without retries.
- High: offline restore could start database replacement without encryption-key
  material. The key-presence/shape preflight now precedes target operations.
- Medium: interactive setup omitted both .NET example automations and its initial
  administrator activity. Defaults and audit now share the setup transaction.
- Medium: Alert input defaults, threshold rules, create/update entitlement checks
  and typed activities differed from .NET. Existing custom rules also continued
  evaluating after license downgrade; runtime now applies the entitlement.
- Medium: weekly quiet hours compared `Sun` with the UI's `Sunday`, so overnight
  schedules could fail to suppress alerts. Evaluation now compares weekday values,
  and writes validate schedules/overlaps instead of silently storing invalid data.
- Medium: evaluation returned one result but persisted events for every matching
  rule. The new PostgreSQL regression reproduced two severity duplicates. Select
  the most severe match before cooldown, preserving non-match recovery.

## Follow-up verification

Linux workspace, Rust 1.97.1 and the cached PostgreSQL 18 fixture described above;
all databases are disposable and separate from the development installation.

| Run | Result |
| --- | --- |
| `cargo test --locked --workspace --jobs 2 --quiet` | **519 passed, 0 failed, 189 ignored**, including documentation checks. Environment-dependent ignores are not claimed as passing. An earlier run overlapped a queued adapter rebuild and lost an artifact during rustdoc; the final complete run was sequential and passed. |
| HTTP/PostgreSQL targets: `users_http`, `roles_http`, `teams_http`, `service_accounts_http`, `licenses_http`, `mfa_http`, `oidc_http`, `resources_http`, `platform_creation_http`, `platforms_http`, `realtime_subscription`, `deployments_http`, `stacks_http`, `swarm_services_http` | **147 passed**, with separate databases per target and `--include-ignored --test-threads=1`. This broad run preceded the last Alert evaluator change; focused Alert/bootstrap runs and the workspace suite verify that change. Existing coverage reruns do not count as newly ported methods. |
| `bootstrap_process` and `phase7_resources_http`, with dedicated PostgreSQL URLs and `--include-ignored --test-threads=1` | **5 + 1 passed** after the bootstrap/default-action/activity and Alert validation/entitlement changes. The combined lifecycle helper is one discovered test, not dozens. |
| Adapter PostgreSQL targets: `alert_persistence`, `identity_access`, `authorized_read`, `automation_execution`, `backup_execution`, `build_execution`, `deployment_persistence`, `stack_persistence`, `swarm_service_persistence`, `resource_metadata_persistence`, `git_repository_execution` | **14 passed** in separate databases. Additional final `alert_persistence` run: **2 passed**, including the new severity regression. Its initial failure reproduced two persisted events for one observation. Seeded global rules are isolated in rule-specific fixtures and restored afterward; production seeds are unchanged. |
| Live `citadel_system_recovery` executable in PostgreSQL 18 as its unprivileged user | **1 passed**, now also proving missing/short encryption keys leave target data untouched before the successful retained-key restore. |
| Unchanged frontend: `npm run test:run -- --maxWorkers=2` | **397 passed in 108 files**, zero failed. Existing Jotai deprecation, jsdom canvas and dialog accessibility warnings remain visible; no frontend source or test was changed. |
| `xtask openapi`, `xtask openapi --check`, `xtask parity` | Generated/verified **404 full / 305 public** HTTP contracts; all **401 .NET operation IDs** match methods, paths and exposure. This remains a catalog/contract gate, not full behavioral parity. |
| Final focused realtime routing regression | `cargo test --locked --workspace --lib alert_rule_activity_follows -- --nocapture`: **1 passed** after the last activity invalidation mapping. This additional test is not included in the earlier 519-test workspace count. |

The Alert fixture initially failed cleanup because its new audit rows correctly
retained the actor through a restrictive foreign key. Cleanup now removes only
the synthetic fixture's audit rows before its actor; the production constraint
and audit preservation are unchanged.

The bootstrap target was rerun after fixing the real startup-readiness race:
**5 passed**, followed by the combined Alert/Phase 7 HTTP target (**1 passed**).
No login retry or sleep was added to hide that failure.

Cleanup removed only the disposable `citadel-parity-followup-20260908` PostgreSQL
container and three failed-run `/tmp/citadel_bootstrap_<uuid>` directories created
by these tests. Their test-only data is not recoverable; the development database,
application containers, source changes and build cache were preserved.

## Still open

- The remaining method-level inventory and theory cases, not just name matches.
- Full packaged recovery/upgrade/bootstrap and Core/Edge process-restart tests.
  The new real debug-Core bootstrap tests and missing-key preflight do not close
  published-image or correct-original-key authentication requirements.
- The complete Alert feature parity audit, including resource scope/type rules,
  threshold/cooldown concurrency and expanded Channel read projections. The covered
  entitlement/activity cases do not certify every remaining Alert requirement.
- Full live HTTP Local/Agent/Edge/multi-node acceptance where only lower-level
  transport/worker evidence currently exists.
- Agent implementation tests remain with the later Agent rewrite; current Core
  interoperability is still a requirement, not deferred with them.

Do not mark Phase 7.5 or full application test parity complete from this pass.

## Alert HTTP assertion closure follow-up

Compared the actual .NET AlertRuleCreateTests / AlertRulePatchTests bodies and
their checked-in success/error snapshots. Expanded the existing HTTP helpers,
with shared assertions in `server/tests/phase7_resources_http/alert_assertions.rs`:

- Full success response values and independent PostgreSQL values, including
  description, severity, cooldown, threshold, required matches, status and exact
  Channel IDs. Creation timestamps/actors and additive Rust fields are checked.
- A valid create omitting status now explicitly proves the Enabled default.
- The same PostgresAlertStore instance used by the router evaluates observations
  immediately after HTTP changes. Tests assert current threshold/consecutive
  matches, severity, disabled state, cooldown and active Channel delivery links.
  They do not reconstruct the store and accidentally hide stale state.
- Channel operations and rejected mutations preserve the entire Rule/link set,
  not just a count. Empty URL also leaves the Channel count unchanged.
- Rename/description tests now independently assert each intermediate persisted
  value and runtime visibility. Configuration PATCH retains a non-null original
  description, and both concurrent disjoint PATCH fields actually change value.
- Requests exercise `application/merge-patch+json`, matching .NET fixtures.
- Validation/not-found cases assert the current full Rust error body, problem
  content type, no-store and request ID. This exposes but does **not** close the
  shared error-envelope mismatch with .NET's field errors/RFC type/traceId and
  resource-specific missing-Rule detail. No Alert-only error adapter was added.

Eleven further .NET methods are now Verified: five create/Channel methods, four
configuration/Channel PATCH methods and two metadata/rename methods. Six Alert
methods stay Partial for the concrete error-body mismatch. Per-method evidence
and totals are in the external closure ledger: **26 verified / 20 partial /
1 missing-divergent / 1,642 unreviewed / 85 deferred / 7 inapplicable**.

Verification (Linux, devcontainer vscode user, Rust 1.97.1, PostgreSQL 18 image
`sha256:4ef4dbc939d61acea57712655ddb4b4ab27419c913f94cca0cd57cb3ea3c2280`):

| Command | Result |
| --- | --- |
| `cargo test --locked -p citadel-server --test phase7_resources_http --jobs 2 -- --include-ignored --nocapture` | **1 passed**, repeated successfully against another fresh database. One discovered combined test with many assertions, not 17 separately discovered tests. |
| `cargo test --locked -p citadel-alerts -p citadel-server --lib --jobs 2` | **14 + 62 passed**, zero failures/ignores. |

The initial new assertion failed on PostgreSQL JSONB number 85 versus
serde_json float 85.0. Numeric threshold equality now compares numeric values;
other persisted fields remain strict. This was a test representation issue,
not a production bug. Both subsequent complete HTTP runs passed.

Only Rust test code and internal evidence/specs changed in this follow-up.
Application code, frontend, schemas, migrations and CI were untouched. No full
workspace, frontend or packaged/live external-service acceptance run is claimed.
The disposable `citadel-alert-parity-20260908` fixture owns all three databases
from this pass; no development data or actual notification destination was used.
After verification, that auto-remove fixture was stopped and removed, including
its disposable databases. Test data is not recoverable; development containers,
database volumes and the existing build cache were preserved.

## Alert error-contract closure follow-up

Closed the six previously Partial Alert error-response mappings against the
original .NET snapshots:

- Create: invalid cooldown, missing RequiredMatches and non-threshold rules
  containing threshold fields now return the original field-keyed errors.
- PATCH: invalid cooldown reports the original domain message under errors[0].
- Missing Rule PATCH and rename return the original resource-specific detail.

These use typed application errors and the existing centralized HTTP mapper,
not a separate Alert response renderer. The RFC type/title/status/traceId and
complete error bodies match the source snapshots. Existing generic responses,
authorization order, no-store behavior and storage-error redaction are retained.
The shared OpenAPI ProblemDetails schema now documents field errors and both
correlation names; neither correlation name is globally required. No frontend
source, database migration or CI configuration changed.

Tests preserve full Rule/link-state assertions for rejected requests and
no-change notifications for rejected PATCH. Additional unit cases cover multiple
simultaneous field errors and cooldown boundaries. The existing UI request shape
is used, including the legacy scope field in the missing-RequiredMatches case.

Verification (Linux, vscode user, Rust 1.97.1; dedicated PostgreSQL fixture at
the same pinned PostgreSQL digest recorded above):

| Command | Result |
| --- | --- |
| `cargo test --locked -p citadel-server -p citadel-alerts -p citadel-identity --lib --jobs 2` | **63 + 15 + 31 = 109 passed**, zero failures/ignores. |
| `cargo test --locked -p citadel-server --test phase7_resources_http --jobs 2 -- --include-ignored --nocapture` | Regression initially **failed** on missing traceId before the fix; **1 passed** after the fix, then **1 passed** again with the final source-equivalent requests on another fresh database. These are executions of one combined HTTP test, not six discovered tests. |
| `cargo test --locked -p citadel-adapters --test alert_persistence --jobs 2 -- --include-ignored --nocapture --test-threads=1` | **2 passed**, real PostgreSQL; atomic mutation, incident deduplication and severity/cooldown selection. |
| `cargo test --locked -p xtask --jobs 2` | **12 passed**, including ProblemDetails schema and both correlation-name contracts. |
| Compiled `xtask openapi`, then `xtask openapi --check` | Generated and verified **404 full / 305 public** contracts. |
| Compiled `xtask parity` | All **401 .NET operation IDs**, methods, paths and public exposure match. Not behavioral proof. |
| Targeted `rustfmt --check` and `git diff --check` | Passed. |

The workspace has no cargo xtask alias, so the generator was built with
`cargo run --locked -p xtask -- openapi`. Its first write was denied because the
three generated artifacts were already owned by root (mode 644). Only the
compiled generator was rerun as root to write those artifacts; generation
checks and all Cargo builds/tests ran as vscode. No permissions were widened.

Current ledger: **32 verified / 14 partial / 1 missing-divergent / 1,642
unreviewed / 85 deferred / 7 inapplicable**. Six additional method mappings are
closed; **1,657 Core/Contracts placements** still require assertion-level closure.
These are not a count of missing Rust test functions. No full-workspace,
frontend or packaged external-service acceptance rerun is claimed here.

All databases in this follow-up belong only to the disposable
`citadel-alert-errors-20260908` fixture. Development databases, workload
containers and the existing compiler cache were not modified or removed.
After verification the auto-remove fixture was stopped, deleting its disposable
databases permanently. No development data was included in that cleanup.

## Quiet-hour timezone and setup parity follow-up

The original AlertRuleQuietHourTests Windows/IANA overlap scenario first failed
in Rust because Romance Standard Time was rejected. Quiet hours now resolve
Windows IDs using the 139 global-default (territory 001) mappings from
[Unicode CLDR release-48](https://github.com/unicode-org/cldr/blob/release-48/common/supplemental/windowsZones.xml).
The small checked-in table uses the existing bundled chrono-tz rules, with no
new runtime dependency, OS lookup, network request or allocated lookup cache.
IANA IDs keep precedence; Windows names are matched case-insensitively. Stored
and public timezone strings are not rewritten. Profile timezone policy remains
IANA-only; this is compatibility for Alert quiet hours, not a global policy change.

Data attribution and refresh instructions are next to the mapping table, with
Unicode-3.0 terms in `crates/alerts/LICENSE.unicode`. Retain that notice when
packaging binary distributions. Tests check every mapped target, summer/winter
evaluation, overlapping and non-overlapping equivalent zones, invalid IDs, and
unchanged HTTP/PostgreSQL values after a rejected overlap PATCH.

New real-Core tests in `server/tests/bootstrap_process/setup.rs` compare the
remaining SetupEndpointTests assertions: pending status/cache headers, normal
API and former-default-login gating, initial identity/creator/Admin assignment,
completed setup actor, usable access/refresh session, typed audit event, exact
per-automation run-as actor/creator/enabled flags/cron/tags, repeat rejection,
and deletion of the initial User without reopening setup (including restart).
The original test removes the User through persistence; the port does likewise
only in its disposable database, without relaxing last-administrator API safety.

The first combined process run had 6 passing tests and one failure: the security
middleware returned 409 with type `about:blank`, while .NET requires
`setup_required`. The middleware now reuses the existing IdentityError mapper
for that branch. Database-unavailable 503 behavior is unchanged. A focused
middleware regression additionally checks no-store and request correlation.

Verification uses Linux Rust 1.97.1 as vscode and the disposable
`citadel-setup-zones-20260908` PostgreSQL fixture, pinned to the same PostgreSQL
image digest recorded above. No development instance, workload or database is
used. Final run results and closure counts follow below.

| Command | Final result |
| --- | --- |
| `cargo test --locked -p citadel-alerts -p citadel-identity --lib --jobs 2` | **17 + 31 passed**, zero failures/ignores. |
| `cargo test --locked -p citadel-server --lib --test bootstrap_process --test phase7_resources_http --jobs 2 -- --include-ignored --nocapture --test-threads=1` | **63 unit + 7 process + 1 HTTP/PostgreSQL tests passed**, zero failures/ignores. All seven setup/process tests reran after the middleware fix. |
| Targeted `rustfmt --check` and `git diff --check` | Passed. |

The new process coverage closes five .NET setup method mappings: one Partial
and four Unreviewed. The pre-existing concurrent-initialization mapping remains
Verified. Windows/IANA overlap closes the previously Missing unit mapping.
Updated totals: **38 Verified / 13 Partial / 0 Missing / 1,638 Unreviewed /
85 Deferred / 7 Inapplicable**, still 1,781 source-method placements. The remaining
**1,651 Core/Contracts placements** require assertion review/closure; they are not
necessarily missing tests. Required-MFA setup and broader identity coverage are
not certified by closing the six-method SetupEndpointTests class.

No frontend, migration, Cargo dependency, generated API or CI changes in this
follow-up. No packaged image build, full workspace suite or external-service
acceptance run is claimed. Existing staged changes remain untouched.

Cleanup stopped the auto-remove PostgreSQL fixture and permanently discarded
its test databases. Successful process tests removed their own temporary data;
the single directory left by the reproduced setup failure was also removed by
its exact fixture ID. No development data or compiler cache was removed.
