# Rust development and verification commands

Packaged Agent checks (Bash, Docker and OpenSSL):

```bash
docker build -t citadel-agent:local -f Dockerfile.agent .
bash test/scripts/test-agent-image.sh citadel-agent:local
bash test/scripts/test-agent-compatibility.sh citadel-agent:local
docker build -t citadel-core:acceptance -f Dockerfile .
bash test/scripts/test-agent-acceptance.sh citadel-core:acceptance citadel-agent:local
```

The smoke test checks the packaged Deno runtime and its file-access restrictions,
alongside Agent startup and runtime tools. It removes its disposable containers
and build image. The compatibility, live Docker and Edge intake suites require
the pinned Rust toolchain and share a private registry, database
and three privileged Docker-in-Docker daemons; it removes only its own fixtures.
The compatibility suite includes six real builds across three Agents (Direct and
Edge), two preparation permits, and 12 MiB contexts. An HTTP barrier proves all
six Docker RUN steps overlap. It checks canonical daemon occupancy through
platform aliases, bounded logs, and refusal of data and remote scripts work
while the shared heavy-work budget is full. This requires an Agent image built
from the current checkout; mock-executor tests do not substitute for this run.
The acceptance test uses the complete Core and Agent images and calls Core's HTTP
API for setup, Direct/Edge registration, stacks, inventory, reconnect, Core restart
and revocation. It also exercises Core's three-node Swarm installer and worker
routing through Direct and Edge managers, including outage recovery, repair,
upgrade and removal. It needs Rust, creates its own database and Docker daemons,
and removes only its own containers, volumes and network.
The current-pair run also exercises network explorer metadata over Direct and Edge:
a two-network application, backend-only container, aliases, host/none/shared
namespace modes, sanitized networking responses and detach. Swarm-node routing
checks include the narrow networking inspection. The current Core/Agent pair also
checks real health transitions and unchecked members over Direct/Edge, retained
history, intentional stops, and three-node Swarm health coverage during worker
loss and recovery. Observing Standalone health must not restart the container.
Long scenarios renew the fixture administrator's session through normal login;
production token expiration and authorization remain intact. Citadel targets one
current Core/Agent/frontend build, with no historical image matrix.
The Swarm image-transfer regression also checks digest-pinned inputs in a clean
Docker daemon, verifying that save/load preserves the selected image identity.
Explicit image pulls retry up to three times for registry failures; test operations
and assertions are never retried.
Run it independently with:

```bash
cargo test --locked -p citadel-agent --test acceptance stages_digest_pinned_image_without_registry_digest_metadata -- --ignored --nocapture
```

See [Agent development](../docs/DEVELOPMENT.md#agent-development) for runtime
configuration and local image setup.

To reuse compatibility-test compilation locally, set
`CITADEL_AGENT_COMPATIBILITY_CACHE_DIR` to a dedicated directory outside the
checkout before running `test-agent-compatibility.sh`. Use a separate directory
for each architecture and pinned builder, and do not share it between concurrent
runs. The script retains this explicitly supplied directory while removing its
temporary fixtures. Without this variable, compilation remains disposable.

Instance update contract tests:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-updates
SQLX_OFFLINE=true cargo test --locked -p citadel-database --lib
SQLX_OFFLINE=true cargo test --locked -p citadel-server --lib workers::updates::tests
SQLX_OFFLINE=true cargo test --locked -p citadel-server --test update_release_cli
SQLX_OFFLINE=true cargo test --locked -p citadel-server --lib diagnostics_http::tests
SQLX_OFFLINE=true cargo test --locked -p citadel-server --test configuration
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::updates::releases::tests
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::updates::artifacts::tests
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::updates::inspection::tests
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::updates::compose::tests
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::updates::bootstrap::tests
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::updates::recovery_keys::tests
# Requires Docker Compose CLI, but no daemon access; uses private disposable inputs.
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::updates::compose::tests::actual_compose -- --ignored
python3 -m unittest discover -s test/release -p 'test_update_fixtures.py'
python3 -m unittest discover -s test/release -p 'test_candidates.py'
python3 -m unittest discover -s test/release -p 'test_release.py'
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test instance_update_journal
# Fresh disposable database only; its name must start with citadel_update_test_.
CITADEL_UPDATE_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:PORT/citadel_update_test_state SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test instance_updates -- --ignored --test-threads=1
# Separate fresh disposable database for read-only preflight validation.
CITADEL_UPDATE_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:PORT/citadel_update_test_preflight SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test update_preflight -- --ignored
# Dedicated disposable PostgreSQL with CREATE DATABASE permission; no Docker socket.
CITADEL_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:PORT/postgres SQLX_OFFLINE=true cargo test --locked -p citadel-server --test bootstrap_process updates:: -- --ignored --test-threads=1
# Docker bootstrap acceptance: use a Core image built from this checkout.
docker build -t citadel-core:update-acceptance -f Dockerfile .
python3 test/scripts/test-update-registration.py citadel-core:update-acceptance
```

These cover release/operation invariants, bounded release HTTP requests, cancellation
before persistence, completion of notification after commit during settings changes
and shutdown, protected journal files and transactional state/Activities. Stopped
attempts cover committed outcomes, unchanged-source release, retry ownership and
retention of unresolved recovery. Frontend installation-settings tests cover saved
status during realtime disconnection and cancellation of stale reads. Artifact
tests use local HTTP fixtures and an injected process runner to verify publication
rejections, exact signature policy, credential isolation, output limits and failure
handling. They do not replace a live signature-verification run or qualify a packaged updater, an installation profile,
a published source-to-target transition or database recovery. Descriptor checks
cover embedded catalog identity, offline CLI output, per-architecture acceptance
and preservation of accepted metadata during publication recovery. Empty source
lists intentionally provide no installation eligibility.

Target-inspection service tests enforce signature-before-execution, exact release
identity, supported target metadata and awaited cancellation cleanup. Injected Docker
runner tests cover immutable image selection, architecture/digest mismatches,
explicit platform selection with both architectures cached and distinct manifest/local IDs,
image-declared volumes, container isolation, ambiguous creation, cancellation,
nonzero/running results, malformed/truncated output, foreign ownership and failed
cleanup. CLI tests reject invalid input or disabled network policy before external
work. These tests do not pull images or touch Docker/databases, and do not substitute
for signed-image inspection on a disposable daemon or packaged updater acceptance.

Update-check frequency regressions cover six/twelve/twenty-four-hour scheduling,
restart and manual-check timing, settings changes without restart, and cancellation
through persistence. `mfa_http::installation_choices_are_atomic_admin_only_persistent_and_enforce_mfa`
also verifies frequency validation and concurrent settings patches. The
`migrations::instance_updates_upgrade_preserves_existing_settings_and_migration_history`
test requires its own fresh disposable `CITADEL_TEST_DATABASE_URL`; it builds the
initial catalog before applying the consolidated instance-update migration. Updates UI tests cover
disabled controls, manual checks, saved frequency and realtime synchronization.

Installer Compose checks cover private snapshots, explicit input order, unsafe
files, unsupported settings, changed inputs, cancellation cleanup and redacted
failures. Dependency checks reject external files and remote includes before the
Compose process starts. The opt-in Compose tests resolve both the shipped layout
and compatible custom filenames, comments and resource names without starting
services. Installer templates are test inputs, not compiled runtime dependencies.
These configuration checks do not validate a live installation or qualify the detached update executor. Bootstrap tests use an
injected bounded Docker runner and private temporary files to cover stable runtime
binding, input and runtime drift, conflicting identity, exclusive volume ownership,
cancellation, secret redaction and idempotent registration. Live registration checks
reuse those observations and test missing/unsafe state, executor mismatch, changed
inputs and instance identity, runtime drift, concurrent evidence/registration changes,
lock contention and cancellation without rewriting registration. CLI tests assert
missing state is not created and local checks are independent of release-network policy.
Catalog tests reject missing, incomplete, failed, duplicate, altered and unknown
migration entries. The disposable PostgreSQL preflight test verifies initialized
identity, exact catalog matching, retained ownership (even corrupt operation JSON),
bounded lock waits, cancellation and preservation of migration history. Storage
tests cover capacity arithmetic and a private write/sync/remove probe without
modifying registration or existing recovery evidence. Recovery-key tests check
file/environment precedence, encoding and size limits, private permissions,
symlink/FIFO refusal, missing-key preservation and secret-free output. Bootstrap
tests require the key diagnostic to run as Core's packaged service user, bind
external key sources to installer settings, and reject failures/cancellation
without rewriting registration. CLI tests exercise file and external keys without
database configuration or normal Core initialization. These checks do not create
or restore-test a checkpoint, prove keys match in-memory state, or test decryption
of all stored ciphertext.
The opt-in Docker bootstrap script checks live revalidation for both the base and
Edge profiles, including drift and preservation of registration/recovery evidence.
Journal tests check exclusive atomic creation without replacing existing files or following links.
The disposable PostgreSQL test also verifies that the identity diagnostic refuses
missing identity or incomplete setup without creating state. These tests do not
replace packaged bootstrap acceptance against an isolated Docker daemon.

Startup checks cover a pristine database, retained and terminal-but-unreleased
ownership, mismatched registration identity, corrupt operation JSON and missing
migrated state. Filesystem tests cover private registration, lock contention,
missing state, foreign paths/ownership, unsafe files and preserved orphan/completed
journals. Process tests verify that maintenance precedes migrations/setup even
without a database claim, survives restart, excludes normal API/Edge routes,
retains the single-Core lease and stops listening on lease loss. Startup lease-loss
regressions also hold migrations blocked or resume them immediately after another
owner takes the lease, checking that the original Core exits without creating an
administrator or opening listeners. These checks do not qualify live draining,
target startup/migration verification or automatic
recovery/resume; those execution paths remain unimplemented.

The live admission barrier has targeted runtime, HTTP and worker regressions:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-runtime admission::tests
SQLX_OFFLINE=true cargo test --locked -p citadel-server --lib api::admission::tests
SQLX_OFFLINE=true cargo test --locked -p citadel-server --lib workers::git::tests
# Each command below needs a fresh disposable CITADEL_EXECUTION_DATABASE_URL.
SQLX_OFFLINE=true cargo test --locked -p citadel-server --lib workers::builds::tests::queued_builds_execute_concurrently_and_shutdown_drains_both -- --ignored --test-threads=1
SQLX_OFFLINE=true cargo test --locked -p citadel-server --lib workers::backups::tests::dispatcher_bounds_claims_and_drains_controllers_on_shutdown -- --ignored --test-threads=1
# Separate disposable CITADEL_PLATFORM_DATABASE_URL with CREATE DATABASE permission.
SQLX_OFFLINE=true cargo test --locked -p citadel-server --test platforms_http realtime_groups::terminal_websocket_requires_join_and_terminal_permission_and_owns_its_session -- --ignored --test-threads=1
```

These cover admission/close races, ownership fencing, timeout/cancellation without
automatic resume, paused queues, admitted execution completion, streaming response
lifetimes and maintenance HTTP responses. The terminal regression rejects new Edge
sessions during maintenance while keeping admitted input/output and authorization
revocation working. The database-backed cases preserve queued
work and distinguish maintenance from normal shutdown cancellation. They do not
qualify a complete drain of durable reservations, writer quiescence, checkpoint
creation or detached handoff. The production update coordinator does not yet
activate this barrier.

Owned drain regressions also exercise the feature coordinator and Server task
adapter:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-updates service::drain::tests
SQLX_OFFLINE=true cargo test --locked -p citadel-server --lib tasks::updates::tests
# Fresh disposable CITADEL_UPDATE_TEST_DATABASE_URL, named citadel_update_test_*.
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test instance_updates -- --ignored --test-threads=1
```

The PostgreSQL test checks exact claim ownership, corrupt/released/stale records,
bounded lock waits, queued versus running work, expired source leases and executor
reservations retained after terminal run status. The coordinator tests reject stale
ownership before closing admission, retain maintenance on timeout/cancellation or
read failure, and detect a local-to-remote handoff during a database snapshot.
The Server adapter test keeps detached cleanup visible after the request ends
without closing the task tracker needed for further cleanup. These checks do not
qualify final writer quiescence, journal handoff, checkpoint creation or source resume.

Checkpoint preparation and shared recovery bundle regressions:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::updates::checkpoint::tests
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::backups::system_recovery
# Two fresh, distinct disposable databases, both named citadel_update_test_*:
# CITADEL_UPDATE_TEST_DATABASE_URL and CITADEL_CHECKPOINT_TARGET_DATABASE_URL.
# Set CITADEL_PG_DUMP and CITADEL_PG_RESTORE to compatible PostgreSQL executables.
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::updates::checkpoint::tests::checkpoint_builds_real_bundle_and_preserves_failed_or_interrupted_attempts -- --ignored
```

These verify exact journal/registration/lock checks, ordered private installer
snapshots, refusal to reuse interrupted attempts, ENOSPC write handling, symlink
refusal, cancellation with awaited subprocess cleanup, failed/corrupt dumps,
configuration drift and mismatched database identity. The disposable PostgreSQL
case creates a real archive and restores its instance identity and marker into
the clean target. The existing `citadel_system_recovery` integration test separately
covers the full schema and encrypted-state round trip. No test authorizes a live
replacement: helper acknowledgement, Core shutdown/writer quiescence, crash-time
power-loss durability and packaged source-to-target recovery remain unqualified.

Checkpoint restore rehearsal has injected-runner tests and an opt-in real PostgreSQL
test. The real test creates its own source and verifier containers with tmpfs data,
no external network, no published ports and no installation volumes. It requires
a cached `postgres:18.6` image and removes its fixtures, including on failure.

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::updates::checkpoint::restore::tests
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::updates::checkpoint::restore::acceptance -- --ignored
```

Coverage includes restored instance/claim/catalog checks, tampered input/dump evidence,
create ambiguity, cancellation, restore failure, foreign-container protection,
cleanup failure, exclusive receipts and no journal advancement. It does not replace
packaged helper or source-to-target upgrade acceptance.

Handoff and shutdown guards:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-updates service::handoff::tests
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::updates::handoff::tests
```

These use private temporary state, real OS installation locks and an injected
Docker runner. They cover launch/binding exclusivity, lock release before helper
startup, verified create/bind/start ordering, missing start intent, wrong user/mounts,
automatic removal, create/start cancellation and ambiguous timeout retention.
Launch retries cannot create a second helper or replay start. They also cover
stale/dead/restarted helpers, retained evidence after helper loss, busy
drains, cancellation, grant-before-stop ordering, changed inputs, clean-exit
journal history and refusal to replay ambiguous stops. A failed/nonzero exit,
restarted Core, unavailable database or shutdown timeout never reaches `BackingUp`.
They do not launch the packaged helper or stop a live Core/Docker installation;
that process/daemon acceptance remains required before enabling installation.
The namespace regression gives Core nonexistent host paths and a different
executor UID, and routes only the bounded Docker exec request into the helper's
private state. CLI tests reject invalid/oversized input without starting Core.

Resource-copy continuation during maintenance has database-backed regressions:

```bash
# Fresh disposable CITADEL_EXECUTION_DATABASE_URL.
SQLX_OFFLINE=true cargo test --locked -p citadel-server --test execution_resources_http resource_copies:: -- --ignored --test-threads=1
```

These finish an admitted two-volume copy after admission closes, reject new HTTP
starts, and verify restricted restore claims skip an older unrelated queued run.
They use fake storage executors; no workload containers or volumes are touched.

`test-update-registration.py` runs the unmodified base installer and a compatible
Edge-overlay installation with custom filenames, comments and resource names,
completes setup through HTTP and invokes the Rust bootstrap
from a separate Core-image container. It checks real Docker metadata, initialized
database identity, private registration, identical retries, unsafe input permissions,
input drift, additional volume/network consumers and preservation of conflicting
registration evidence. It also checks that registration never restarts or replaces
Core or PostgreSQL. It needs Python 3 and local Docker socket access on the test
host; installed users do not need Python. All services and data run inside its
own privileged Docker-in-Docker fixture, with no host socket passed to Core and no
published host ports. A temporary private network allows the pinned PostgreSQL pull;
it is removed before Core starts. The test removes only its own daemon and storage,
including on failures. It does not qualify image replacement, recovery or source-to-target
database transitions.

Execution recovery regressions use a separate disposable PostgreSQL database:

```bash
# Set CITADEL_EXECUTION_DATABASE_URL to a new disposable database first.
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test build_execution --test backup_execution -- --ignored --test-threads=1
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib local_restart_reconciles_exact_helpers_and_fences_late_starts -- --ignored --test-threads=1
# Explicit opt-in to a disposable Docker endpoint with postgres:17-alpine loaded.
CITADEL_TEST_DOCKER_ENDPOINT=unix:///var/run/docker.sock cargo test --locked -p citadel-docker --test local_runtime -- --include-ignored
```

The Local helper test creates only uniquely named fixture helpers and removes
those helpers afterward. It checks interrupted ownership before admission and
after cleanup, same-daemon reconciliation, delayed dispatch fencing, unavailable
Docker, and unrelated helper preservation. Ambiguous creation, unsupported build
history, and interrupted Core staging/direct Restic work retain ownership rather
than authorizing replay or lease takeover.

Run these commands from the repository root in PowerShell.

Docker-based Cargo test suites use a unique `citadel-rust-test-<id>` build
volume per invocation and remove it in `finally`, including after test failures.
Dependencies/toolchains remain cached; compiled output is reused within a suite
but not between suites. A forcibly killed PowerShell process or unavailable
Docker daemon can prevent cleanup; the script warns when Docker refuses removal.
Cleanup never prunes unrelated volumes or forcibly removes an in-use volume.
Run `./test/scripts/Test-BuildCache.ps1` to test these safety rules without Docker.

```powershell
# Rebuild and verify the generated Docker subset (Rust plus Java 21).
docker build -t citadel-test-toolchain:local -f test/Dockerfile.toolchain .
docker run --rm --mount type=bind,source=${PWD},target=/repo -w /repo `
  citadel-test-toolchain:local cargo run --locked -p xtask -- docker-api --check

# Format, lint, and run unit/Unix-socket compatibility tests.
./test/scripts/Test-Workspace.ps1

# Run the PostgreSQL Actor-authorization fixture against an isolated database.
./test/scripts/Test-AuthorizedRead.ps1

# Build the pinned release container.
./test/performance/Build-MeasurementImage.ps1

# Short local measurement.
./test/performance/Measure-Server.ps1 -DurationHours 0.1

```

Measurement output is written to the ignored `artifacts/` directory. The
Compose file enforces `memory.max=100 MiB` and disables swap for the measurement container.

Windows-native Cargo commands additionally require the Visual Studio Build
Tools C++ workload because the installed Rust host is `x86_64-pc-windows-msvc`.
The scripts use the pinned Linux toolchain so the release path does not depend
on the host linker.

## Source ownership and contracts

Run the dependency guards locally (no database, Docker or network required after
dependencies are installed):

```bash
cargo test --locked --offline -p citadel-server --test resource_structure --test resource_architecture --test workload_architecture --test multi_resource_architecture
cargo test --locked --offline -p citadel-adapters --test architecture
npm --prefix src/frontend run test:run -- src/test/architecture.test.ts
```

These inspect all workspace dependency declarations and production source boundaries.
The Rust source checks parse grouped/renamed imports and skip test-only items without
skipping production code later in the file. Feature guards also catch synchronous
filesystem probes such as `Path::is_file()`. The frontend checks API/UI/routing import
boundaries. Keep fixture exceptions confined to test modules. No additional CI job
is needed to run these commands locally.

Run `bash test/scripts/check-source-ownership.sh` to verify source ownership and
`cargo test --locked -p citadel-contracts` from the repository root to check the protocol
inventory. Database generation uses the Rust declarative schema; see
[database development](../docs/DEVELOPMENT.md#database-schema-changes).

## API metadata, wire compatibility, and login cleanup

These focused regressions run in the existing Rust workspace test job:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-server --lib openapi::
SQLX_OFFLINE=true cargo test --locked -p citadel-contracts --test protocol_sources
SQLX_OFFLINE=true cargo test --locked -p citadel-docker --test local_runtime
```

The OpenAPI tests inspect the live handler definitions, so missing public-operation
summaries fail even when generated JSON has not been refreshed. Protocol tests pin
field numbers and verify old-peer decoding as well as the reviewed source baseline.
The Unix login fixtures use temporary shell scripts, not Docker or a Registry;
they verify redaction, credential-directory isolation, and cleanup after failure
or cancellation. No database or live resources are required.

Run `SQLX_OFFLINE=true cargo clippy --locked --workspace --all-targets -- -D warnings`
as well: ordinary tests do not run Clippy, and `--all-targets` includes test fixtures.

## Test fixture settings

Test names and fixture variables describe the subsystem they exercise:

| Fixture | Database variable |
| --- | --- |
| Authorized reads | `CITADEL_AUTHORIZED_READ_DATABASE_URL` |
| Identity and access | `CITADEL_IDENTITY_DATABASE_URL` |
| Platform inventory and reads | `CITADEL_PLATFORM_DATABASE_URL` |
| Resource metadata | `CITADEL_METADATA_DATABASE_URL` |
| Deployments, Stacks and Swarm workloads | `CITADEL_WORKLOAD_DATABASE_URL` |
| Git, Builds, Backups, Script and Alerts execution | `CITADEL_EXECUTION_DATABASE_URL` |
| Statistics persistence | `CITADEL_STATISTICS_DATABASE_URL` |
| Migrations and parity | `CITADEL_TEST_DATABASE_URL` (separate fresh database, CREATE DATABASE permission) |

These must point to disposable test databases. The PowerShell harnesses set them
for their fixture processes. Recovery tests use separate source and target databases.
Run ignored database tests serially (`--ignored --test-threads=1`).

The `platforms_http` suite requires `CREATE DATABASE` permission on
`CITADEL_PLATFORM_DATABASE_URL`. Each test creates and removes its own database
so administrator list and realtime reads cannot pick up another test's fixtures.

`Test-Workspace.ps1` runs formatting, Clippy and workspace tests. The server's
optional Docker and Agent probes are `docker-smoke` and
`agent-smoke --agent-id <uuid-from-agent-setup>`. The Agent probe uses the same
`CITADEL_AGENT_ID` as the target Agent; its address alone does not identify it.
`realtime-client.mjs` takes its token from `CITADEL_REALTIME_TEST_TOKEN` and connects
to the URL supplied through `--url`; the Core endpoint is `/api/v1/realtime`.

## Completion, import authorization and runtime CI regressions

The existing authorization/runtime database CI stage explicitly lists each critical
selector before executing it, so a renamed test cannot silently run zero tests:

- `stacks_http::stack_endpoints_enforce_auth_and_persist_apply_release_and_delete`
  uses `CITADEL_WORKLOAD_DATABASE_URL`. Its import matrix checks source-read and
  materialization counters for denied users/service accounts and explicit/inferred
  import kinds, alongside the existing transaction and preflight fixtures.
- `build_execution::build_runs_claim_once_persist_results_cancel_and_recover` and
  `backup_execution::backup_claims_are_repository_exclusive_and_late_results_do_not_overwrite_recovery`
  use `CITADEL_EXECUTION_DATABASE_URL`, each with its own fresh database because
  recovery inspects shared queues. They run real service orchestration with
  transaction wrappers injecting precommit failures and lost commit acknowledgments,
  recovery, supersession and cancellation. Execution and cleanup remain single-use.
  The build fixture also covers shared-daemon reservations, stale cleanup receipts,
  duplicate acknowledgments, terminal transaction rollback and restart after a
  durable cleanup receipt. Operator recovery covers current human-administrator
  access, active/stale attempt rejection, concurrent resolution, audit/log rollback,
  missing admission replies, and preservation of run outcomes without fabricated
  Agent receipts. The Edge fixture checks the original target's host identity;
  it does not prove cleanup across a live Agent restart.
  Agent ledger and signed Direct tests cover delayed
  requests after reconciliation and rejection of changed authority/daemon IDs.
- `alert_persistence::durable_observation_recovers_missed_wakes_and_does_not_evaluate_twice_after_ack_failure`
  keeps durable observation consumer/receipt coverage on a separate fresh execution
  database, so unrelated queued observations cannot affect its receipt counts.
- `alert_persistence::update_resolution` runs on another fresh execution database.
  It covers verified update recovery and supersession of pending targets, including
  cooldown, concurrent detection, stale observations, acknowledgment preservation,
  existing overlapping alerts, and transactional rollback on delivery-queue failure.
  Unchanged and cooldown-suppressed rechecks retain observation ordering across
  restart without extending cooldown; baseline coverage verifies the observation
  timestamp is available and restart preserves alert history and cooldown.
- Server **library** tests
  `workers::platforms::state_delta_tests::lifecycle_events_use_no_docker_reads_and_unknown_identity_stays_container_scoped`,
  `workers::platforms::state_delta_tests::local_and_direct_bursts_commit_once_and_publish_six_ids`,
  and `workers::platforms::inventory_tests::daemon_bursts_coalesce_and_events_during_refresh_schedule_one_follow_up`
  run serially with `CITADEL_PLATFORM_DATABASE_URL` pointing to a separate runtime
  database. Inventory temporarily changes other fixture connectors.

The already selected `platforms_http` suite retains realtime authorization/generation
coverage and checks HTTP request-drop cancellation of an active Edge log command,
including release of pending work. Migration/checksum tests keep their separate
database. Required CI check names and workspace feature resolution are unchanged.

## Focused release regressions

Run these locally when changing process cleanup, authentication dependencies, or
the persistence fixtures. CI also runs the ignored MFA, OIDC, users, teams and
roles HTTP suites, resource-copy HTTP regressions, cancellation-safe ACL commit
regression, and the complete `migrations` binary (including parity), serially
against disposable PostgreSQL. Migration checksum testing uses a separate fresh
database because it deliberately corrupts its migration journal:

```bash
cargo test --locked -p citadel-processes --test process_runner -- --include-ignored
cargo test --locked -p citadel-agent --test lifecycle
cargo test --locked -p citadel-adapters --test oidc_protocol
# Set the corresponding disposable database URLs from the table above first.
cargo test --locked -p citadel-adapters --test authorized_read --test resource_metadata_persistence --test resource_permissions -- --ignored --test-threads=1
cargo test --locked -p citadel-server --test execution_resources_http --test users_http --test oidc_http -- --ignored --test-threads=1
npm --prefix src/frontend run test:run -- --maxWorkers=1
npm --prefix src/frontend run build:prod
npm --prefix src/frontend audit
# Requires cargo-audit installed locally.
cargo audit
```

The new Linux process tests are opt-in to keep CI runtime unchanged. They cover
helpers inheriting pipes after their parent exits,
including timeout, cancellation, and a dropped runner future. Unix cleanup kills
the command's process group and reaps the direct child; the packaged init process
reaps orphaned descendants. Helpers that deliberately create another session or
process group are outside that group cleanup.

Interactive and binary Docker exec cleanup have a disposable-container suite
(local Docker and an existing `redis:latest` image):

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-docker --lib client::transport::terminal -- --include-ignored --test-threads=1
```

It checks shell/job exit after disconnect, idle expiry and shutdown, isolation
between sessions, stale process-identity rejection, binary bytes/options/exit status,
unpolled output, lossless output backpressure, cancellation during startup,
descendant cleanup after parent exit, and lost startup/approval responses. Stack hook descendant
cleanup and realtime heartbeat regressions run without Docker:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-docker --lib stacks::process_tests
SQLX_OFFLINE=true cargo test --locked -p citadel-server --lib realtime::connection
```

Dependency updates must preserve login/refresh and signed OIDC token verification.
The DOMPurify override keeps Monaco and API-generation tooling on the patched
sanitizer until their upstream dependency pins catch up. JWTs use the AWS-LC
backend to avoid the unpatched RSA dependency of the alternative backend.

These focused checks supplement exact-image Core/Agent acceptance and an overnight
workload soak; a short passing test run does not establish long-term memory stability.

## foundation

Run `./test/scripts/Test-Foundation.ps1` from the repository root to
exercise the embedded Rust migration runner against a disposable PostgreSQL
database and verify the generated database, Docker, OpenAPI, and frontend
artifacts. The script tests concurrent startup, restart idempotence, seed data,
and checksum refusal. It removes its uniquely named container and network when
the run finishes.

Release upgrade verification must also initialize a disposable database with the
previous **published Core image**, then start the candidate against that same
database and verify retained data and migration history. Tests constructed from
the current repository baseline do not establish published-version upgrade
compatibility. Preserve checksum enforcement and never repair a test failure by
rewriting an installed migration journal.

## identity and access

Run `./test/scripts/Test-Identity.ps1` from the repository root. It uses
disposable PostgreSQL databases and server containers to verify identity and access flows,
including local identity, authenticated application information, current
profile, lazy and persisted profile preferences, active browser-session listing
and revocation, password changes, atomic safe Activity evidence, authorized
Activity list/detail compatibility, Actor authorization, Service Account ACL/token,
license administration, concurrency, and restart behavior. The run executes
the versioned authorization matrix through the Rust PostgreSQL implementation.
It also runs the dedicated
database-backed Axum User, Team, Role, License, MFA, and OIDC endpoint suites for reads and administrator mutations,
including conflicts, assignments, password/session invalidation, safe Activity
evidence, rollback, and concurrent last-administrator protection. The HTTP flow
proves User and Team create/patch/rename persistence, Role permission
reduction/rename persistence, and stable License instance identity across restart and deletion
after restart. All resources are uniquely named and removed in the script's `finally`
block.

The OIDC portion also runs a local signed-JWT issuer fixture. It validates the
real discovery, token, and JWKS protocol adapter without relying on a public
identity provider or storing provider tokens.

Run `./test/scripts/Test-Frontend.ps1` to build the production React
bundle, serve it from the Rust process, and execute browser compatibility tests
against disposable PostgreSQL and Keycloak services. The default remains the
existing Keycloak OIDC suite. Pass one or more repository-relative Playwright
files through `-TestFiles` to exercise other screens, for example:

```powershell
./test/scripts/Test-Frontend.ps1 -TestFiles @(
  'tests/compatibility/oidc.spec.ts',
  'tests/compatibility/access.spec.ts',
  'tests/smoke/licensing.spec.ts'
)
```

The combined identity browser run covers first-run setup, OIDC redirect and
callback, browser-session restoration and logout, User/Team production forms,
Roles, Community-gated Service Accounts, License rendering, and OIDC provider
administration. For repeated browser runs, opt into a retained compiler cache
with `-BuildCacheVolume citadel-rust-e2e-local` on the first build, then pass that
same option with `-SkipBuild`. This explicitly retained cache is not removed by
the script; delete it when finished. Without this opt-in, the default temporary
cache is always cleaned, so `-SkipBuild` requires an explicit retained volume.

## Platform and Docker reads

Run `./test/scripts/Test-PlatformReads.ps1` from the repository root. It creates
a disposable PostgreSQL database and exercises Local Docker and signed Agent
mapping/transport, transactional inventory and statistics persistence, the
real authorized Axum Platform/Docker/Swarm read routes, reconciliation worker
behavior, and bounded authenticated realtime subscriptions. The script removes
its uniquely named PostgreSQL container and network in `finally`.

## simple mutations and metadata

Run `./test/scripts/Test-ResourceMetadata.ps1` from the repository root. It
creates one disposable PostgreSQL database and verifies Tag, Registry, Git
repository, resource-binding, internal/external Secret, and Secret-provider
lifecycles through the real store and Axum routes. It also covers Local and
Agent Network/Volume mutation parity, mutation retry safety, authorization,
activities, realtime invalidations, and deterministic OpenAPI/Docker contract
generation. The script removes its uniquely named PostgreSQL container and
network in `finally`.

## Deployment CRUD

Run `./test/scripts/Test-Deployments.ps1` from the repository root. It
creates a disposable PostgreSQL database and verifies the Deployment domain,
persisted JSON contracts, authorized SQLx CRUD/duplicate/delete
transactions, typed Activity evidence, the real Axum endpoints, deletion claim
rollback and timeout behavior, Local Unix-socket and signed Agent container
delete transport equivalence, and deterministic OpenAPI/Docker generation. It
also exercises CRUD/config regression scenarios:
merge-patch validation, duplicate conflicts/failures, Local-image projection,
direct and Team ACL filtering, Platform authorization, concurrency exclusion,
and recovery after request cancellation. Container deployment, progress and
reconciliation are covered by `Test-DeploymentApply.ps1`. The script removes its uniquely named PostgreSQL container
and network in `finally`.

## external execution and Git accounts

Run `./test/scripts/Test-ExternalExecution.ps1` from the repository
root. It verifies the bounded child-process primitive, Git argument and cache
safety, encrypted Git-account persistence, authorized HTTP lifecycle, and
generated OpenAPI parity against a disposable PostgreSQL database. The gate also
covers Git synchronization, Script/Build/Backup execution, Alert persistence,
Edge enrollment/transport, node projection isolation and authorized HTTP/realtime
responses. Agent command responders are test peers, not released Agents.

For the real Local Docker/Restic volume round trip, start the development
container and run `./test/scripts/Test-LocalBackup.ps1` from the host.
Use `-DevContainer <name>` if the workspace container has a different name.
It creates a separate disposable PostgreSQL database and uniquely named Docker
volumes, backs up a file, restores it at the volume root, verifies persisted
results and rejects an unapproved overwrite. It uses the existing development
compiler cache and does not build a Citadel image or touch the development DB.
The test removes its fixture volumes; the script removes its fixture database.
The round trip also verifies Deployment and Stack copies from existing backups
and current data, shared-volume mapping, numeric ownership and permissions of
the volume root and contents, source preservation,
cancellation, stale reviews and owned-volume cleanup. Current-data copies do not
enqueue backup-policy restore runs. The HTTP/database tests also cover durable
claims, idempotent start, interruption recovery and permission revocation. They
also verify committed copy-progress notifications and initiating-actor isolation.
The copy wizard tests cover empty-ID request prevention, realtime updates and
reconnects, active-state spinners, and returning from a completed copy.
The `resource_copies::recovery` regression injects lost claim acknowledgements
and completion-write failures against disposable PostgreSQL; it also verifies
finalization and cleanup during a paused copy, cancellation and worker shutdown.

`-AgentImage <candidate>` without `-UseRustFs` tests filesystem repositories
through a Direct Agent; adding `-UseEdgeAgent` tests the authenticated Edge session.
Both cover existing named volumes and host directories, initialization, validation,
backup, restore, retention, wrong-host rejection, missing storage, symlink rejection,
and repository/restore-volume overlap, with Core's Docker connection unavailable.
The directory fixtures are UUID-scoped under the Docker host's `/tmp` and removed
after the test. Tests run sequentially against the disposable database.

`-UseRustFs -AgentImage <candidate>` and `-UseEdgeAgent` run those same
copy checks through real authenticated Agents with Core's Docker connection
unavailable. `-MultiNode` additionally copies a pinned managed Swarm Service,
applies it on its original worker, and checks that no destination volume appears
on the manager or another worker. These are opt-in acceptance gates; normal
unit-test runs do not execute them.

For the browser workflow and actual Core process crash recovery, run the
[core-runtime Playwright suite](e2e/README.md) against its disposable environment.

Normal browser development (`dev.sh compose-up`) builds and mounts release Core
by default. Use `CITADEL_COMPOSE_PROFILE=dev` for faster unoptimized builds; use
the default release profile for comparable CPU/I/O measurements.
Native debugger tasks retain their development profile.

Cross-platform copy regression tests live in
`src/server/tests/execution_resources_http/resource_copies/destinations.rs`.
Run the `resource_copies::` filter with a disposable
`CITADEL_EXECUTION_DATABASE_URL`. They verify compatible destination catalogs,
SQL authorization filtering and revocation, source snapshot versus destination
restore routing, repository portability, stale destination fencing, and Swarm
reference remapping/node placement. These persistence tests simulate completed
restores; real data and owner-label checks belong to the Restic acceptance fixture.

Set `CITADEL_TEST_CROSS_PLATFORM_COPY=1` for the RustFS local-volume acceptance
fixture, with `CITADEL_TEST_AGENT_IMAGE` and `CITADEL_TEST_AGENT_NETWORK` configured,
to also exercise Deployment and Web Editor Stack copying to a second disposable
Docker daemon through a signed Direct Agent. Its S3 endpoint must be reachable
from nested Docker helpers. The fixture checks payload/ownership/symlinks,
source isolation, destination receipts and cleanup, and removes its nested daemon.

Cargo dependency advisories run independently in `.github/workflows/advisories.yml`
on dependency pull requests and default-branch pushes. The pinned `cargo-audit`
scans `Cargo.lock`; it does not scan OS packages, Docker/Compose, Restic, Deno or
frontend assets. Any narrowly scoped exceptions live in `.cargo/audit.toml`.
The Compose environment-isolation check runs explicitly in CI's **Rust unit and
database tests** job, after workspace tests. It verifies both exact test selectors
and runs the ignored wrapper with the pinned Docker CLI and Compose plugin from
`test/scripts/test-stack-runtime.sh`. It needs the Compose executable, not a daemon:

```bash
cargo test --locked -p citadel-docker --lib stacks::tests::compose_interpolation_excludes_core_secrets -- --ignored --exact
```

## Execution capacity gates

Use a fresh disposable database for each database command below. The worker
comparison deliberately simulates the old lifecycle-held preparation permit and
the new transfer-only permit with the same six projects and log workload; it is
not a benchmark of two released Core binaries.

```sh
SQLX_OFFLINE=true cargo test --locked -p citadel-server --lib workers::builds::tests::two_preparations_can_track_six_executions_and_drain -- --ignored --nocapture --test-threads=1
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test backup_execution -- --ignored --test-threads=1
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test build_execution -- --ignored --test-threads=1
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test edge_sessions
SQLX_OFFLINE=true cargo test --locked -p citadel-agent --lib --test direct --test configuration
```

The first three commands read `CITADEL_EXECUTION_DATABASE_URL`. Backup coverage
includes renewable ownership beyond the former timeout, stale owners, six data
reservations over three hosts, shared-heavy limits, aliases, expiry quarantine,
source deferral, immutable plans and completion failures. Agent Direct tests use
signed HTTP/2 with a Docker fixture, including a cancelled create whose absent
helper name must not free capacity. Edge session tests use 12 MiB contexts and
bounded noisy output; payload ownership survives cancellation until encoding ends.

`cargo test --locked -p citadel-backups --lib` also checks stalled renewal,
ownership-loss cleanup and capped completion backoff using paused time.
The backup execution fixture checks tracking ownership across dispatcher handoff
and completion retries. With a separate disposable `CITADEL_EXECUTION_DATABASE_URL`,
`cargo test --locked -p citadel-server --lib workers::backups::tests -- --ignored --test-threads=1`
checks concurrent dispatch, the tracking limit and shutdown drain without running
Docker or Restic.

`cargo test --locked -p citadel-agent --test edge_intake -- --ignored --test-threads=1`
uses `CITADEL_AGENT_TEST_DATABASE_URL` and the real authenticated Core/Agent Edge
transport, with a fixture Docker server. Data reservations survive reconnect and
late admissions are fenced after cleanup. The opt-in Agent `memory` test measures
an isolated Agent process across repeated Direct/Edge workloads. These protocol
fixtures do not substitute for the real Docker/Restic round trips above.

Capacity measurements from the disposable 2026-10-27 fixture (debug binaries):

| Fixture | Observed result |
| --- | --- |
| Lifecycle-held preparation, six projects | 2 executing; 2 preparing peak; 397 ms to overlap; 24 ms shutdown drain |
| Transfer-only preparation, same projects | 6 executing; 2 preparing peak; 855 ms to overlap; 56 ms shutdown drain |
| Persisted noisy output | 64 / 192 rows, 61,440 / 184,320 message bytes respectively |
| Combined worker comparison process | 38,264 KiB peak RSS; 1.47 s user + 0.09 s system CPU; 4.23 s elapsed |
| Isolated Agent, 36 Direct rounds | 103,256 KiB highest sampled RSS; 78,252 KiB final idle RSS; 12 final file descriptors |
| Isolated Agent, 36 Edge rounds | 49,504 KiB highest sampled RSS; 49,444 KiB final idle RSS; 12 final file descriptors |

These are fixture measurements, not production sizing guarantees or a comparison
of two complete Core versions. The Agent daemon/build process is simulated; the
transport, bounded 12 MiB input handling and cancellation are real. RSS samples
are not continuous peaks. The worker RSS/CPU figures cover both modes together.

Execution diagnostics use fixed `RuntimeWork` families: queue depth/oldest age,
durable build/data reservation counts, pending cleanup, preparation and cleanup
durations, and result-persistence attempt counts/durations. Edge transfer,
control and output buffered-byte gauges follow buffer ownership through drop.
Database gauges are sampled by maintenance; they are not admission authority.

### Script placement and credentials

Use a disposable `CITADEL_EXECUTION_DATABASE_URL` for the adapter test
`remote_attempts_fence_tokens_deferral_shared_capacity_and_recovery` and the Server
`script_http_execution` integration target. The latter also needs
`CITADEL_DENO_PATH`. Run each against its own empty fixture database because queue
claims intentionally cover all actions.

Agent unit tests cover shared heavy capacity, duplicate attempts and cleanup
fencing. The ignored Agent library test `actual_deno_is_bounded_redacted_sandboxed_and_reaped_after_stream_loss`
uses `CITADEL_DENO_PATH`, temporary workspaces and an unreachable fixture Docker
endpoint; it does not touch a real Docker daemon. It checks HTTPS input validation,
redaction, host-file/subprocess/network restrictions and process cleanup after a
lost stream. `token_safety` uses its own disposable `CITADEL_TOKEN_SAFETY_DATABASE_URL`
and verifies run tokens become invalid on cancellation, attempt replacement and
completion, in addition to its HTTP/realtime credential restrictions.
