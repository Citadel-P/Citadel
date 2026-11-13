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
routing through Direct and Edge managers, including outage recovery, repair
rollouts that preserve the approved image, central update discovery and removal.
The same-version Core/Agent pair cannot approve an update; the test checks that
the central review endpoint rejects it, including when an unpublished candidate's
availability is unknown. It needs Rust, creates its own database and Docker daemons,
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
SQLX_OFFLINE=true cargo test --locked -p citadel-agent --lib updates
SQLX_OFFLINE=true cargo test --locked -p citadel-database --lib
SQLX_OFFLINE=true cargo test --locked -p citadel-server --lib workers::updates::checker::tests
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

Agent update tests use a fake Docker daemon to cover failed image pulls before
handoff, drain timeouts, idempotent settlement, rejected replay and changed identity.
Ambiguous helper creation and lost Agent-process evidence must retain recovery ownership.

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

Core replacement boundary tests use private disposable installer/checkpoint files
and an injected Docker runner:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib replacement
```

They check durable intent/journal ordering before mutation, exact Core-only Compose
arguments, private atomic `.env` publication, preservation of checkpoint inputs,
rejection of changed/missing checkpoint receipts, tampered bundles and installer
snapshots, cached image identity, runtime drift, cancellation, timeout, failed/truncated responses and no
replay. Successful command submission must leave maintenance active and must not
claim verified startup or completion. These do not exercise a real Core replacement
or qualify an end-to-end update; packaged driver/startup/recovery acceptance remains
required before enabling installation.

Replacement observation tests reuse the handoff/replacement fixtures and enforce a
read-only Docker command allowlist:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::updates::handoff::target_tests
```

They cover target binding while readiness is unhealthy, repeated observations,
interruption between binding and journal publication, target/helper restarts,
changed PostgreSQL configuration/start identity, recreated storage/network, leftover
source containers, installer edits during observation, malformed evidence and
cancelled/timed-out commands. The journal may advance only to `Verifying`; no
completion flags or maintenance release are inferred. These tests do not exercise
target migrations or a packaged end-to-end upgrade. Target exchange tests also
exercise the Compose/startup binding race: repeated pending reads preserve the
journal, a published binding becomes ready, and conflicting fences/grants,
terminal journals or a missing installation lock fail closed. Key-identity tests
reject replaced JWT/encryption/signing keys, stale loaded keys, changes to key
sources, and edited checkpoint key files/checksum metadata without generating
new keys or rewriting the journal. Bootstrap-process
tests continue to enforce maintenance before ordinary migrations, including
corrupt operation JSON and retained external evidence.

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
administrator or opening listeners. A valid claim targeting another build also
remains in maintenance without migrations or changes to the saved operation.
These process checks do not qualify a packaged target migration or automatic
recovery/resume. Target acceptance, host reconciliation and normal admission are connected.
Packaged source-to-target qualification remains separate from these process tests.

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
creation or detached handoff. The installation coordinator activates this barrier
after importing the acknowledged helper journal.

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
# Use two NEW empty databases for this separate test:
# CITADEL_CHECKPOINT_DATABASE_URL and CITADEL_CHECKPOINT_TARGET_DATABASE_URL.
# Both names must start with citadel_update_test_; set the same PG tool variables.
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::updates::checkpoint::recovery_tests::restores_checkpoint_and_releases_only_the_verified_fresh_database -- --ignored
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

Live updates verify checkpoint integrity without launching a PostgreSQL restore
sandbox. Replacement tests cover missing/foreign receipts, tampered dump/key/input
files, unsafe paths and cancellation before any installer mutation. The existing
checkpoint round trip above and `citadel_system_recovery` test exercise restoration
through the production recovery implementation. They are qualification checks, not
steps performed during every update.

Target acceptance retains decryption checks for stored credentials. The
`instance_updates` database test covers more than two pages of encrypted values,
a wrong key and malformed data on the final page; failure must retain maintenance
and leave the stored data unchanged. All six credential stores are exercised,
including optional OIDC secrets and missing required provider tokens. These tests do not qualify a published
source-to-target transition or a packaged end-to-end update.

Handoff and shutdown guards:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-updates service::installation::tests
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::updates::handoff::
```

These use private temporary state, real OS installation locks and an injected
Docker runner. They cover launch/binding exclusivity, lock release before helper
startup, verified create/bind/start ordering, missing start binding, wrong user/mounts,
automatic removal, create/start cancellation and ambiguous timeout retention.
Launch retries cannot create a second helper or replay start. The helper profile
requires the exact PostgreSQL network namespace, a read-only Core data mount,
and bounded scratch storage at `/tmp` outside the durable update-state directory;
host networking and writable data mounts fail verification. Stopped-source checks
reject restart, unclean exit, lost PostgreSQL and changed helper profiles. They also cover
stale/dead/restarted helpers, retained evidence after helper loss, busy
drains, cancellation, grant-before-stop ordering, changed inputs, clean-exit
journal history and refusal to replay ambiguous stops. A failed/nonzero exit,
restarted Core, unavailable database or shutdown timeout never reaches `BackingUp`.
They do not launch the packaged helper or stop a live Core/Docker installation;
that process/daemon acceptance remains required before enabling installation.
The namespace regression gives Core nonexistent host paths and a different
executor UID, and routes only the bounded Docker exec request into the helper's
private state. Source authorization and target acceptance exercise the same
`update-helper-handoff` command, including action/phase, operation and process
identity checks. Exit-wait tests reject changed helper images, labels, commands,
mounts, users and network/privilege settings, as well as restart, OOM, daemon error,
nonzero exit and unexpected lifecycle states. Clean exit and cancellation retain the
journal and maintenance claim. CLI tests reject invalid/oversized input without starting Core.
The replacement evidence checks are shared by observation, pending target reads
and completion; changing the saved checkpoint or configuration must fail all of
them without advancing the journal or granting/replaying execution.

Target-completion tests reject uncommitted or foreign receipts, changed target incarnations,
altered history, preparation changes during receipt verification and premature
maintenance release. Failure recording rejects a changed helper acknowledgement.
The mailbox never writes journal history concurrently with the driver.
Execution-state regressions require phase and
target identity to be published atomically, reject mismatched snapshots, prevent a
phase-only write from discarding execution evidence, and retain preparation on failure.
They also reject saved target or preparation changes during runtime inspection without
overwriting that evidence. Readiness tests cover waiting for preparation, preparation
finishing during helper verification, and cancellation without granting shutdown. Failure after replacement preserves the
checkpoint and records recovery without issuing another Docker mutation.
The `instance_updates` disposable-database test rejects inconsistent claim columns
on import, release and receipt verification, and proves rejected writes leave the
operation and Activities unchanged. An injected completion-Activity failure rolls
back the imported outcome. Released terminal imports and releases remain read-only,
including when a newer attempt owns the instance. It also exercises source-catalog
verification and target credential decryption over multiple pages, including a
wrong key and a malformed final page. This is current-schema verification, not
qualification of a published source-to-target transition.

Target preparation regressions reject shutdown without a fresh, matching preparation
receipt. Compose comparison tests exercise the real base installer and Edge overlay,
keep the input files byte-for-byte unchanged, and reject changes to credentials,
ports, volumes, PostgreSQL, ignored image selectors and concurrent input edits.
These are configuration and handoff checks, **not** a source-to-target update test.

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::updates::compose -- --include-ignored
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib persistence::postgres::updates::agents::tests
# Disposable database only, named citadel_update_test_*:
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib persistence::postgres::updates::agents::database_tests -- --ignored
```

The ignored `root_helper_reads_packaged_core_keys_without_trusting_other_owners`
test requires root inside a disposable container, a fresh tmpfs mounted at
`/app/data`, and `CITADEL_TEST_PACKAGED_KEYS=1`. Run the compiled adapters test
binary read-only with no Docker socket, host data mounts or network. It verifies
the root helper can read UID 65532 keys while refusing other owners, permissive
key modes and symlinks. Never point it at an installation's data volume.

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
registration evidence. It runs the packaged host utility with its read-only root,
private tmpfs, no network and restricted mounts. An explicit Core recreation then
checks that `--refresh` preserves installation/storage identity and all three
recovery keys, rejects unresolved recovery evidence, and leaves PostgreSQL running.
Registration and revalidation themselves never restart or replace either service.
The candidate release gate runs this check against each accepted Core architecture;
missing or failed registration evidence prevents promotion. It needs Python 3 and local Docker socket access on the test
host; installed users do not need Python. All services and data run inside its
own privileged Docker-in-Docker fixture, with no host socket passed to Core and no
published host ports. A temporary private network allows the pinned PostgreSQL pull;
it is removed before Core starts. The test removes only its own daemon and storage,
including on failures. It does not qualify image replacement, recovery or source-to-target
database transitions.

Run the updater lifecycle locally before publishing a release:

```bash
python3 test/scripts/test-instance-upgrade-local.py --runtime-image YOUR_LOCAL_CORE_IMAGE
# Verify source-helper / target acceptance across revisions using a separate checkout:
# git worktree add --detach /tmp/citadel-update-source <source-commit>
python3 test/scripts/test-instance-upgrade-local.py --runtime-image YOUR_LOCAL_CORE_IMAGE --source-tree /tmp/citadel-update-source --scenario upgrade
```

Use a Core runtime image built from this checkout. The runner compiles two
test-only Core executables with distinct versions in the Dockerfile's pinned
`rust-source` builder and packages them using that runtime. This avoids linking
against the host's potentially newer glibc. Compilation is cached separately in
each source checkout's `target/upgrade-fixture`; the packaged executable's version
and test-only identity are checked offline before any installation starts.
Only the release discovery/trust dependency is replaced by a private
fixture. With `--source-tree`, both the source executable and its initial Compose
file come from that checkout; the target comes from the current checkout. The
real HTTP setup/login, review/approval, Docker inspection, detached
helper, checkpoint, replacement, target verification and PostgreSQL persistence run
unchanged. The fixture registry is reachable only inside a disconnected disposable
Docker daemon. No fixture credentials or journals are stored in the checkout.

The command runs a normal update and target crashes after receipt commit, after
helper exit, and after host cleanup but before database ownership release. The
first two use barriers compiled only into the fixture executable and exercise
Docker's restart policy on the same accepted container. All must complete without replacing PostgreSQL or
changing installation identity, volumes or recovery keys. Each restores its actual
checkpoint into a fresh database and checks encrypted data and released recovery
ownership. Select one case with `--scenario upgrade`, `--scenario receipt-restart`,
`--scenario helper-exit-restart` or `--scenario cleanup-restart`. Additional cases
`--scenario concurrent-requests` and `--scenario preparation-failure` exercise the
real admission middleware/worker during blocked helper preparation and automatic
recovery of an unchanged source. All cases run by default.
Ordinary `cargo test` does not start this fixture server; its test executable exits
unless packaged with its private fixture file. Product binaries have no fixture
setting, and candidate publication rejects the test executable's identity marker.

This is a lifecycle regression test, not a signature test or proof that an arbitrary
older release can migrate to a future candidate. Keep production artifact verification
and source/target compatibility checks enabled. Real release transition qualification
must use the intended source catalog and target build.

For manual testing in a browser, use the same runner:

```bash
python3 test/scripts/test-instance-upgrade-local.py \
  --runtime-image YOUR_LOCAL_CORE_IMAGE --browser
```

This also builds the current frontend (`npm ci --prefix src/frontend` is required
first if dependencies are missing) and includes it in both test images. After setup,
the terminal prints `http://localhost:18080/updates` and a generated administrator
password for `upgrade@example.test`. Sign in, click **Check now**, then **Update
now → Update and restart** to move from `0.1.1-dev.9001` to `0.1.1-dev.9002`.
Observe the restart indicator, reconnection, completed operation and frontend reload.
You control approval; the runner does not install the update for you or restore a
checkpoint in this mode. It stays running after the update so you can inspect the UI.

Use `--port 18081` if the default port is occupied. Only the UI port is published,
bound to `127.0.0.1`; the inner Docker daemon and PostgreSQL are not exposed.
Browser mode retains the fixture's bridge network for port forwarding, so it has
outbound network access. Its test registry remains inside that daemon. Existing
Citadel instances, host volumes and development servers are not modified. Press
**Ctrl+C** in the runner terminal to remove its installation, data, network and test
images. Run it again for a fresh source version and new credentials.

`test-instance-upgrade.py` is a separate opt-in smoke test for an **already eligible
published transition**, not a pre-publication qualification bypass. It requires an
exact official source digest with the updater, and the target must be the release
currently offered to that source. Both signatures are verified by Core; the target's
embedded source list must allow the source catalog and architecture. Empty or
mismatched eligibility stops before creating an installation. The current descriptor
has no eligible transitions, so a complete run cannot pass yet.

```bash
python3 test/scripts/test-instance-upgrade.py \
  --source-image ghcr.io/citadel-p/citadel@sha256:SOURCE_DIGEST \
  --target-version TARGET_VERSION
python3 -m unittest discover -s test/release -p test_instance_upgrade.py
```

The runner reuses the registration fixture's private Docker daemon, retaining
outbound access for official release verification. It exposes no host ports and
passes no host socket to Core. Setup and an encrypted Secret use the real HTTP API;
review/approval are submitted once. It waits for the persisted success and released
maintenance, then checks PostgreSQL uptime, installation/storage identity, all three
recovery keys, retained ciphertext and sign-in. It stops only fixture Core and uses
the exact source image to restore the checkpoint into a separate empty PostgreSQL
container, checking restored identity, ciphertext and released maintenance while
preserving the original successful database and host evidence. After an approval
attempt fails, private state and bounded container logs are copied to a private
`/tmp/citadel-upgrade-failure-*` directory before fixture cleanup. These files may
contain fixture credentials; keep them outside Git. The daemon, its data and network
are removed on success or failure.

This smoke test does not qualify an unsigned candidate or test crash injection during
replacement/finalization. Its Python unit tests cover refusal and assertion logic;
they are not evidence that a published upgrade succeeded. The full candidate upgrade
and interrupted-recovery qualification remains a release prerequisite.

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

The Server `workers::notifications` tests also cover scoped copy/coverage/Stack
notifications on the shared connection: commit and rollback, reconnect, bounded
subscriber lag, and independent copy-lane wakeups. `workers::resource_copies`
unit tests check idle fallback, coalescing, active backoff and cancellation.
Adapter `backups::coverage::changes::tests` check distinct-platform batching,
duplicate suppression, bounded batches and resync without a database lookup.
The native `execution_resources_http` `resource_copies::` tests retain coverage
for lost commit acknowledgements, cancellation, finalization and admitted
multi-volume copies during maintenance. Use a fresh disposable
`CITADEL_EXECUTION_DATABASE_URL`; their storage executors are fixtures.

The Server `workers::notifications` tests cover all fixed wakeup topics, including
job-alert observations and Swarm pruning: commit/rollback delivery, one shared
LISTEN backend, reconnect resync and restored subscriptions. Their ignored tests
use `CITADEL_TEST_DATABASE_URL` on a disposable PostgreSQL database. The
`workers::alerts` unit tests also check startup draining, coalesced wakeups,
30-second recovery after hub closure and cancellation. The ignored
`workers::pruning` test uses `CITADEL_PLATFORM_DATABASE_URL` and a fake Docker
runtime to verify that the shared notification path still respects pruning
settings and only deletes terminal tasks without forcing deletion or removing volumes.

Statistics storage coverage includes the `migrations` test
`compact_statistics_preserve_samples_uniqueness_and_migration_history`: it upgrades
populated tables from migration 0003 through the consolidated 0004, verifies
redundant UUID indexes are removed, and normalizes measurements without changing
owner snapshots or the applied 0001–0003 checksums. Conflicting
copies must abort and roll back; corrected data can then upgrade. The
`stats_batch_persistence` storage tests compare raw and compacted gauges, resets,
nullable disk counters and ownership, exercise rollback and sealed-minute retries,
and verify sparse minutes, partial-range boundaries, partition expiry and bounded
default cleanup. A seven-day fixture checks executed compaction plans in both custom
and generic modes: sample selection and deletion must prune unrelated days while
preserving other sources and samples outside the compacted hour. They create
isolated child databases because maintenance affects the whole database.
Unchanged direct and buffered retries must preserve tuples and
acceptance counts. Platform history tests compare raw and compacted host/node
charts with resets, nullable counters, empty captures, removed containers and
node-only buckets. They exercise pending-alert revision barriers, a concurrent
capture that becomes pending while compaction waits for its lock, rollback,
late Direct/Edge retries, sparse/latest offline observations, daily summary expiry
and bounded default cleanup. The platform history migration test preserves raw
rows and pending `xmin` claims while registering historical sources without live
bindings. The `stats_batch_persistence::storage_review` module measures native
heap/TOAST/index bytes without counting partition indexes twice for small dense
10-second polling, 50-container dense 2-second polling, stable 10/15-second polling,
and high container churn. It compares history before/after compaction, checks
staggered replica/node timestamps over old and mixed windows, and exercises a
persisted same-cluster manager handover with both inventory projections present.
Its 1,024-source native probe reports elapsed throughput without a latency assertion.
Run with disposable migration/statistics URLs only.

The `stats_batch_persistence::query_efficiency` probes execute custom and generic
container-history plans over a multi-source fixture and bound scanned samples to
the requested source/day (one gauge pass and one counter pass). A native Platform
query checks a temporary-write budget at fixed `work_mem` over 432,050 raw
container observations with full-length Docker IDs, in custom and generic modes
and with an additional compacted node to exercise the mixed path. It compares
per-bucket values as well as exact totals and rates; it does not assert
machine-dependent latency. Identity-function wrappers
in a disposable child database count real capture lookups, checking that repeated
samples resolve each identity once, reuse warm Workload owners without helper
calls, preserve old owners on reassignment, and still reject sealed corrections.
A descriptor-heavy capture fixture checks executed materialization outputs and
zero temporary writes at fixed `work_mem`, with all samples retained. Swarm
handover coverage includes alternating host/node projections within one batch.

`cargo test --locked -p citadel-server --lib workers::maintenance::tests` uses paused
Tokio time to model 1,024 continuously sampled sources plus backfill, assert net
backlog reduction independent of retention sweeps, and bound compaction passes,
timeouts and cancellation. It checks five-minute startup, idle and error delays,
the full one-minute pause after backlog batches/timeouts, and the return to idle
after catch-up. This validates scheduling capacity, not a hardware SLA.

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
- `alert_persistence::threshold_job_policy_requires_fresh_matches_and_respects_suppression_and_severity`
  also checks that repeated healthy observations preserve the state tuple and
  cooldown timestamp, while still resetting broken match sequences and resolving
  incidents whose counters were already reset by triggering.
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

Realtime overhead regressions use the same disposable PostgreSQL fixture:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-server --test platforms_http realtime_ -- --ignored --test-threads=1
SQLX_OFFLINE=true cargo test --locked -p citadel-server --lib realtime
SQLX_OFFLINE=true cargo test --locked -p citadel-server --test realtime_subscription
npm --prefix src/frontend run test:run -- src/features/stacks/form/hooks/useStackInfoGroup.test.tsx
```

`realtime_overhead` holds inventory/configuration table locks to verify that safety
rechecks do not reload collections or container statistics/image/deployment joins,
and Stack statistics do not reload Compose/release configuration. A two-platform
workload fixture verifies scoped SQL reads, full reconnect snapshots, authorization
and preservation of unaffected rows. Telemetry unit tests count estimator calls and
check encoded-size alignment after invalid samples, while a locked-table shared-read test checks that subscribers reuse the same
event-owned telemetry context without bypassing authorization. Unit tests cover shared event/cache byte accounting, reservation
release, cancellation, sample selection and overflow recovery. Stack hook tests check
that compact samples preserve lifecycle metadata and cannot create unknown containers
or resurrect removed ones. These checks do not measure production RSS.

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

CI validation uses Ubuntu 24.04 explicitly, matching candidate builds. Rust
workspace tests have a 20-minute command deadline; each database test invocation
has a 10-minute deadline, including compilation. Tests are never retried by the
wrapper. Failures retain the original exit code; timeouts return 124 (or 137 if
forced termination is needed). The `rust-test-diagnostics-<attempt>` artifact keeps
command output, exit codes and timings for seven days, including on failed runs.
Database failures also capture connection wait/blocking state, without SQL text
or application records, before removing the owned disposable PostgreSQL container.
These diagnostics cannot survive an abrupt loss of the runner itself.

The database suite lives in `.github/scripts/test-database-regressions.sh`; its
selectors and fixture-specific databases are shared by CI and local reproduction:

```bash
RUNNER_TEMP=$(mktemp -d /tmp/citadel-ci-tests.XXXXXX) SQLX_OFFLINE=true bash .github/scripts/test-database-regressions.sh
python3 -m unittest discover -s test/release -p test_ci_check.py -v
```

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

### Execution retention and cleanup

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-processes --test process_runner -- --include-ignored
SQLX_OFFLINE=true cargo test --locked -p citadel-docker --lib builds::output
SQLX_OFFLINE=true cargo test --locked -p citadel-builds -p citadel-scripts --lib ownership_tests
CITADEL_DENO_PATH=/absolute/path/to/deno SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test script_sandbox -- --include-ignored --nocapture
```

The process regressions compare capture and stream-only modes, including concurrent
stdout/stderr, truncation, closed consumers, backpressure, timeout, cancellation,
inherited pipes and dropped futures. Stream-only results retain zero transcript
capacity; output limits count admitted bytes independently. Build output tests
check redaction across split secrets/UTF-8, bounded persistence batches, failure
cancellation, and blocked sinks during cancellation or producer timeout. Existing capture-mode script final logs are still checked.

The opt-in `execution_cycles::repeated_stack_build_and_deno_cycles_release_execution_resources`
scenario labels warmup and three repeated stack/build/Deno cycles. It uses a fake
Docker CLI, manual and Git-shaped source bundles (including context, env and config
files), the real staging/output adapters, and real Deno. It checks transient roots,
zero build transcript capacity, run-cache removal and tracked cleanup counts.
It does not run a Docker daemon, a Git checkout, or measure deployed Core RSS.
The adjacent real-Deno test also verifies abandoned-future cleanup and Linux reaping.
For durable claim/completion and HTTP-disconnect behavior, run `stacks_http` against
an empty `CITADEL_WORKLOAD_DATABASE_URL` and `build_execution` against a separate
empty `CITADEL_EXECUTION_DATABASE_URL`, with `--ignored --test-threads=1`.

These changes reduce bounded execution-time allocations. They do not establish
that a post-operation container working-set plateau is a Rust heap leak or predict
its size. A representative repeated workload still needs matched cgroup anonymous/
file-cache measurements, Core PSS and helper-process observations before and after
cleanup. Do not infer live allocations from RSS alone.

### Script placement and credentials

Filesystem-only cache and cleanup checks (no database or Docker daemon):

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test script_sandbox --test workspaces
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib external::backups::restic::process::tests
CITADEL_DENO_PATH=/absolute/path/to/deno SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test script_sandbox -- --ignored
```

These verify per-run Deno cache cleanup after repeated/concurrent execution,
failure, timeout, cancellation and abandoned futures; split-token/UTF-8 log
redaction; and tracked backup subprocess-cache cleanup through shutdown. Core
and Agents use the same script sandbox. The real Deno test uses temporary inputs
and does not contact an external API.

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


Updater installation integration checks now cover the supervised approval queue,
dropped browser responses, source drift, revoked administrators, idempotent retries,
acknowledgement failure, busy drains and failed grants without replay or release.
Finalization tests verify target registration, preserved checkpoint paths, archived
completion and refusal to remove changed/nonzero-exit helpers. Frontend tests cover
review-before-approval, a stable retry key, retained operation lookup, offline
progress and bounded planned-restart feedback. Completion tests distinguish a
committed success from released maintenance, wait for the matching running version
and reconnect, and verify one-time frontend reload with the URL retained and the
completed session hint/banner cleared after reload. Active operations hide release notices.
Failed outcomes, foreign instances and unavailable session storage do not cause an
automatic reload; a manual action remains available after completion.

These tests do not authorize adding a source to a release descriptor. A packaged
source-to-target run, including checkpoint restore and interrupted cleanup recovery,
is still required before publishing a qualified transition. Preserve private state
and checkpoints on failure; never delete journals or clear database ownership merely
to make startup pass.


### Central Agent updates

To test real Direct and Edge Agent updates in the browser without publishing a
release, run:

```bash
python3 test/scripts/test-agent-upgrade-local.py \
  --runtime-image YOUR_LOCAL_CORE_IMAGE \
  --agent-runtime-image YOUR_LOCAL_AGENT_IMAGE --browser
```

Use existing Core and Agent runtime images as bases. The runner builds the current
frontend and binaries from this checkout, starts Core at `0.1.1-dev.9002`, and
registers a Direct Agent and an Edge Agent at `0.1.1-dev.9001`. Each Agent has its
own disposable Docker engine and persistent identity volume. The private catalog
offers an Agent image matching Core; no Core update is needed.

The terminal prints `http://localhost:18080/updates` and a generated password for
`upgrade@example.test`. Under **Config → Available updates**, select both Agents,
click **Review and update**, then **Update now**. Check progress in the same dialog,
Activities, and both Agents returning as up to date. Core remains available.
Reloading or closing the page must not cancel the operation. Run the command again
to reset the fixture. Use `--port 18081` if needed; **Ctrl+C** removes its containers,
volumes, network and built images. Existing installations are untouched.

Omit `--browser` for automated approval and verification of authenticated
reconnection, target digest, changed container IDs, preserved identity mounts and
ports, and retained source containers. It updates Direct first and verifies the
unselected Edge container is unchanged, then updates Edge using the generated
`edge-agent` label and mixed-case Edge mode. Edge's isolated daemon listens only
at `/run/user/1001/docker.sock`; the Agent uses two custom networks with aliases.
The runner verifies the helper uses that socket and replacement retains both
network IDs and aliases. This exercises a rootless socket layout, not a genuinely
unprivileged daemon or its host kernel configuration. Both must settle their exited helper
before the operation finishes. This mode disconnects the outer test daemon
from its network after loading prerequisites. Browser mode publishes only the UI
on loopback and retains outbound connectivity. Neither mode exposes Docker or
PostgreSQL on the host.

Only the test Core replaces release verification with its private catalog.
Production binaries have no switch for it. These runs cover standalone Direct and
Edge updates; they do not qualify published signatures, Build Pool registration,
Team schedules, or multi-node Swarm rollouts.

The focused service tests cover independent Agent eligibility, installation alias
deduplication, stale reviews, request replay, sequential failure handling,
interrupted-attempt reconciliation, Team scheduling and authorization rechecks.
They also cover shutdown after dispatch, cancellation during verification and
lost Swarm resume acknowledgements without releasing the claim or redispatching.

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-updates service::agents
SQLX_OFFLINE=true cargo test --locked -p citadel-agent --lib
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib connectors::routing::agent_updates
npm --prefix src/frontend run test:run -- src/features/updates/index.test.tsx src/features/activities/index.test.ts
```

`citadel-adapters --test agent_updates` exercises real PostgreSQL claims, concurrent
approvals, unresolved ownership, revision fences, Activities and schedule occurrence
reservations across minute boundaries. It checks duplicate/busy occurrences and
rejects stale pre-claim configurations while preserving the frozen scope of an
already-owned occurrence after schedule edits. Service tests cover shared schedule
continuation after simulated Core restart and cleanup, compatibility blocking and
Team expiry. These simulated restarts do not replace packaged replacement tests. Create a
fresh disposable database named `citadel_update_test_*`, then run:

```bash
CITADEL_UPDATE_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:5432/citadel_update_test_agents \
  SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test agent_updates -- --ignored
```

The standalone replacement test uses the real Docker API against containers and a
volume created by the test. It requires an already-pulled official Agent digest;
set `CITADEL_AGENT_TEST_IMAGE` to `ghcr.io/citadel-p/citadel.agent@sha256:<digest>`:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-agent --test agent_replacement -- --ignored
```

This test verifies actual container replacement, persistent storage, environment,
host ports and retained-source restart policy. It uses a shell fixture inside the
Agent image; it does **not** establish an authenticated Direct/Edge rollout or a
multi-node Swarm rollout. Use the local runner above for authenticated standalone
rollouts. Keep published-release signature verification separate from these Docker
fixtures; do not weaken image validation for local testing.

Update availability alerts use the existing alert persistence test fixture. Run against
a **fresh disposable** database named `citadel_update_test_*`:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-updates --lib availability
CITADEL_EXECUTION_DATABASE_URL=postgres://postgres@127.0.0.1:PORT/citadel_update_test_alerts SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test alert_persistence instance_updates:: -- --ignored --test-threads=1
```

The persistence test covers seeded Community rules, channel assignment, delivery
deduplication, aggregate summary refresh, supersession, failed checks, offline
observations and stale queued availability after installation. It never sends external notifications.


### PostgreSQL engine maintenance

The Updates page reads the connected server version through the existing status
API and offers only the qualified target for a registered, supported installation.
Status reads do not launch helpers or acquire the installation lock; review and
approval perform the live preflight. Run its focused read/error and realtime checks with:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-updates --lib database_version
SQLX_OFFLINE=true cargo test --locked -p citadel-updates --lib database_review
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib database_status_reads_version
python3 -m unittest discover -s test/release -p test_instance_upgrade.py
npm --prefix src/frontend run test:run -- src/features/updates
```

The `database_engine_version_comes_from_the_connected_server` test in
`citadel-adapters --test instance_updates` exercises the query and closed-pool failure.
It uses the same disposable `CITADEL_UPDATE_TEST_DATABASE_URL` restriction as the
other instance-update database tests; run with `--ignored` against that fixture.

### Browser and full-lifecycle PostgreSQL upgrades

Build the current Core image, then run the existing isolated updater fixture:

```bash
python3 test/scripts/test-instance-upgrade-local.py --runtime-image citadel-core:local --database-source 18
# Run the same replacement on a real rootless daemon in disposable DinD:
python3 test/scripts/test-instance-upgrade-local.py --runtime-image citadel-core:local --database-source 18 --rootless
# Retain a disposable installation for clicking through the real UI:
python3 test/scripts/test-instance-upgrade-local.py --runtime-image citadel-core:local --database-source 18 --browser --port 18081
```

The `--rootless` mode uses the pinned rootless Docker image, verifies its daemon
mode, keeps installer/state ownership mapped to the daemon user, and checks the
bundled database status before registration. It uses no host Docker socket inside
the fixture. The same flag also supports Core update and recovery scenarios.

The first command exercises PostgreSQL 18.1 → 18.6 replacement via the real
review/approve APIs, duplicate approval, checkpoint, retained credentials/keys,
startup acceptance and registration refresh. It also verifies that an unsupported
replication setup fails preflight without stopping Core or PostgreSQL. PostgreSQL
17 and earlier are rejected; this runner does not qualify future major upgrades. Browser mode prints its own login;
choose **Review and update → PostgreSQL → Update and restart**. Core remains the
same version and existing Agent/production fixtures are untouched. Ctrl+C cleans up
only this fixture. Private failure evidence is retained outside the repository.

The `--database-source 18 --scenario database-preparation-failure` scenario
creates a replication slot and verifies that review is rejected before ownership.
It then reviews without the slot, recreates it, and verifies that approval also
rejects the changed eligibility without stopping Core or PostgreSQL.

### Container network throughput and disk I/O

The shared container statistics panels offer accumulated bytes and per-second
throughput. Focused frontend coverage is in `stats/network-usage.test.tsx`,
`lib/stats-buckets.test.ts` and `useServiceStats.test.ts`. The Docker mapping test
`maps_block_io_across_devices_without_counting_total_or_sync_entries` covers
optional counters and device totals. Agent mapping preserves optional block I/O
in the existing statistics transport tests.

`stats_batch_persistence` tests use a disposable `CITADEL_STATISTICS_DATABASE_URL`
and exercise history for containers, workload replacements and Swarm Tasks,
including resets, elapsed-time-weighted rates, irregular sample intervals, missing baselines, duplicate writes
and both persistence paths. Run them serially with `--ignored --test-threads=1`.
`platform_history_large_window_preserves_totals_and_restores_connection_settings`
checks 86,415 physical counter observations over a two-day window, all 577 output
buckets, and restoration of the pooled connection's JIT setting after both a
successful read and a SQL failure. It asserts values, not a machine-dependent
latency threshold. For performance investigation, use `EXPLAIN (ANALYZE, BUFFERS,
SETTINGS)` on disposable data and compare JIT compilation and temporary I/O as
well as total execution time; the Platform reader runs with transaction-local
`jit=off`.
The consolidated `0002_instance_updates.sql` migration adds nullable counters
without filling historical samples with zero. The block-I/O migration test upgrades
from `0001` and verifies that recorded statistics and its applied checksum survive.

Container sampling error propagation and recovery are covered by
`cargo test --locked -p citadel-docker --test docker_transport container_stat_sampling`.
This reuses the Unix-socket transport fixture to verify typed failures retain
Docker's error cause and a subsequent successful reading works normally.

### Telemetry allocation and query regression checks

Use disposable PostgreSQL 18 databases for every command below. Alert fixtures
require a database named `citadel_update_test_*`; set the three database URLs to
separate disposable databases. `SQLX_OFFLINE` does not isolate runtime SQL.

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-server --lib workers::statistics
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib batch_input
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib history_admission -- --ignored --test-threads=1
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test alert_persistence -- --ignored --test-threads=1
SQLX_OFFLINE=true cargo test --locked -p citadel-server --lib workers::disk_tests -- --ignored --test-threads=1
SQLX_OFFLINE=true cargo test --locked -p citadel-server --lib workers::runtime_persistence_tests -- --ignored --test-threads=1
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test stats_batch_persistence -- --ignored --test-threads=1
```

The 10,000-container ingress fixture checks complete-capture admission, conservative
encoded bounds and shared reservation lifetime. Writer tests cover byte boundaries,
retry ownership and shutdown. Native alert tests use a one-connection pool with cold
configuration and a database-backed entitlement provider; injected acknowledgement
failure must roll back events and suppress callbacks. Concurrent corrections must
remain pending. History admission tests observe `pg_stat_activity`, cancel a real
sleeping backend, and verify admission is held until server cleanup and local
settings are restored. Compaction budget fixtures check complete-minute prefixes,
oversized-minute fencing, another source's progress and later recovery.

Performance comparisons must keep accepted samples, polling, history and query
memory settings constant. Include full-length Docker IDs, raw and mixed history,
custom and generic plans, unrelated sources/days, resets, missing counters and
historical ownership. Compare every returned field and record temporary I/O,
backend memory, elapsed time and useful progress separately. Encoded SQL argument
bytes are not allocator peaks; container working set includes cache/shared memory
and is not a direct measure of PostgreSQL private allocations. Do not trade hidden
sample loss for a lower memory graph. Retain source/owner/time predicates, narrow
counter windows, shared history admission, atomic alert acknowledgement and
complete-minute compaction boundaries in future changes.

#### Review measurements (2026-11-12)

Baseline: `7255781fc16ef298edf0ec620d5bda2bba47e1b9`. Measurements used disposable
PostgreSQL 18.6, `work_mem=4MB`, JIT off and unchanged accepted input. Timing values
are illustrative measurements on the development machine, not production SLAs.
The workload/service fixture contained 50 physical sources, full-length Docker IDs,
8,641 observations per source, resets and missing block counters. Mixed history
compacted its first hour. Every output field matched the baseline in all eight
resource/raw-or-mixed/custom-or-generic combinations (floating tolerance `1e-10`
relative / `1e-8` absolute).

| Finding | Validated mechanism and implementation | Evidence and limitation |
| --- | --- | --- |
| F1 | `statistics/batch.rs`, `batch_input.rs`, `platform_metadata_batch.sql`: full capture JSON was sent to locking/metadata queries. Use UUID arrays and borrowed, narrow projections. | The 10,000-container unit fixture reduces modeled SQL argument bytes from 6,473,688 to 1,618,681, about 75%. These are encoded bytes, not allocator/RSS savings. Native tests preserve duplicate order, metadata, Edge fencing and closure while waiting for a lock. |
| F2 | `alerts/platform_metrics.rs`, `rule_evaluation.rs`, shared rule selection: alert processing reacquired the pool while holding the flush transaction. Prepare configuration/entitlements first; evaluate and acknowledge atomically. | The one-connection cold path timed out before the change and succeeds afterward, including a database-backed entitlement provider. Injected acknowledgement failure commits no events and emits no callback. Concurrent corrected revisions remain pending. Preparation may make sequential checkouts; evaluation never nests one. |
| F3 | `workers/statistics/budget.rs`, `ingress.rs`, `writer/`: item counts ignored nested payload size. Shared admission tracks independently reclaimable writer portions through completion/retry; batches honor encoded limits. | Six complete 10,000-container polling cycles fit through a minute flush when container writes make progress. The isolated debug test process peaked at 43,008 KiB RSS; this is not a baseline Core comparison. Oversize/capacity drops are explicit, whole-cycle and rate-reported; retry expiry and shutdown release reservations. Collection/transport and allocator-retained memory remain outside the ingress bound. |
| F4 | `alerts/platform_metrics.rs`, `workers/stats_alerts.rs`, runtime metrics: wide pending reads retained unused fields and stalled alerts could hide blocked compaction. | The 500-capture/1,000-container fixture's sample CTE width falls from 102 to 64 bytes; neither version spilled. This is a narrow-read improvement, not a claimed major memory win. Bounded diagnostics report pending work, oldest age, progress and blockage without clearing protected work. |
| F5 | `platforms/history_admission.rs`, `reader.rs`, Core composition: unrestricted expensive reads multiplied database working sets. Shared immediate admission and local timeouts retain ownership through cleanup. | Six one-hour fixture reads completed in 6.56 seconds with one concurrent execution and 4.66 seconds with two; individual latency rose with two. Two slots retain throughput while bounding concurrency, clamped to one for a one-connection pool. Native cancellation tests observe the backend and restored settings. A slow large range can now return a timeout; shorten/retry it. |
| F6 | `workload_history.sql`, `service_history.sql`: wide array-bearing windows amplified sort/temporary work. Separate weighted gauges from narrow physical counter endpoints and combine logical replicas after physical deltas. | Temporary writes fell 64–80% across the eight comparisons. Mixed generic workload: 323,133,440 → 72,695,808 bytes. Sampled backend anonymous RSS peak: 20,128 → 15,584 KiB. Sampling is a lower-bound observation, not an allocator high-water mark. Generic plans may still choose one date-index scan with owner filtering; the rewrite avoids repeating that scan per owner. |
| F7 | `platform_maintenance.rs`, `platform_compaction_bounds.sql`, `platform_compaction_size.sql`, maintenance store: an hour cap did not bound nested observations. Size complete-minute prefixes, recheck under the scope lock and rotate candidates. | A 1,000-container source-hour has 360,000 observations; a 16-minute prefix has 96,000. Compaction statement time was 801 → 145 ms and temporary writes 6,316,032 → 0 bytes. Four bounded commits sealed the hour; sizing/discovery round trips are additional cost. Discovery across 1,024 sources measured 15.8 ms / 6,177 shared hits. Oversized minutes stay unsealed; source-local stall outcomes let the same bounded worker batch visit later sources. An all-stalled batch rests instead of spinning. |

F1–F7 preserve ownership, history values, alert revision fencing and atomic
raw-to-summary movement. Those changes add no schema migration, global PostgreSQL setting,
container memory cap, collection-frequency reduction or replacement database.
The larger redesigns (separate durable metric-alert queue, shared physical I/O
storage, statement-cache/release-profile tuning and small-install PostgreSQL
configuration) remain measurement-gated follow-ups.

This review does not establish an end-to-end demo memory reduction, a Core allocator
peak reduction, or multi-day catch-up capacity under arbitrary source churn. Cache,
shared buffers and already allocated database files may remain visible after
execution work falls. Re-measure an equal deployment workload before claiming that
the original production memory graph is fixed.

### Swarm Platform I/O

`stats_batch_persistence::swarm_platform_io_aggregates_nodes_without_losing_history_or_overwriting_host_gauges`
uses the disposable statistics database to verify node-scoped baselines, counter
resets, task replacements, duplicate writes, manager double-count prevention,
revoked/superseded Edge sessions, retained traffic after inventory deletion and
node-only history buckets. Host gauges remain null when only nodes report.
`maintenance::tests::stats_retention` covers the node-history retention index.
Run the full retention group against a disposable `CITADEL_STATISTICS_DATABASE_URL`:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib maintenance::tests::stats_retention -- --ignored --test-threads=1
```

It checks the cutoff, indexed selection, 5,000-row limit, and pending-alert
protection for the remaining raw host/node history tables. Partitioned measurement
retention has separate storage tests. A large-history fixture verifies
the executed plan deletes a small expired batch without scanning retained rows.
Two-session fixtures verify locked rows are skipped, updated tuple locations are
selected on the next pass, and rolled-back deletions remain eligible. Timing is
not asserted. Never use `EXPLAIN ANALYZE DELETE` against a live instance; live
investigation should use non-executing `EXPLAIN` in a read-only transaction.

Frontend `platform-io.test.ts` verifies equivalent live aggregation and simultaneous
node updates; `platform-stats.test.tsx` covers chart scope and gaps. Realtime node
updates retain the existing Platform group permission checks and host metadata.

### Maintenance query plans and expiry

Against a fresh disposable `CITADEL_STATISTICS_DATABASE_URL`, run:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib maintenance::tests:: -- --ignored --test-threads=1
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test maintenance_retention -- --ignored --test-threads=1
```

The additional retention test uses large retained histories and checks executed
scan row counts, batch limits, active/current runs, build reservations, completion
queues and nested deployment/stack references. Copy expiry exercises the real
repository with a concurrent locked review that becomes a started operation.
The database `migrations` suite includes an upgrade from migration 0003 that
preserves history/checksums and verifies indexed Activity ordering with timestamp
ties. These are disposable-fixture tests; do not execute cleanup plans with
`EXPLAIN ANALYZE` against deployed databases.


### Instance settings

`cargo test --locked -p citadel-instance` covers all promoted
fields, partial patches, ranges, coupled effective values, explicit overrides,
typed log-format JSON and deployment HTTPS prerequisites.
The update checker tests also verify that first-run setup immediately wakes its
Instance settings subscription, before the periodic repair poll.
`cargo test --locked -p citadel-server --lib config::instance` checks deployment
presence independently of parsed defaults and lowered environment maxima combined
with compatible saved defaults. The existing configuration parser process
suite remains database-independent. Actual startup logging now resolves saved
choices first, so its format/color test uses the disposable database too. Both it
and the restart test run in the existing database-regression CI fixture.

With a fresh disposable PostgreSQL instance and CREATE DATABASE permission:

```bash
CITADEL_IDENTITY_DATABASE_URL=postgres://postgres@127.0.0.1:PORT/postgres SQLX_OFFLINE=true cargo test --locked -p citadel-server --test mfa_http -- --ignored --test-threads=1
CITADEL_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:PORT/postgres SQLX_OFFLINE=true cargo test --locked -p citadel-server --test bootstrap_process instance_settings -- --ignored --test-threads=1
CITADEL_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:PORT/postgres SQLX_OFFLINE=true cargo test --locked -p citadel-server --test configuration log_format_and_color -- --ignored --test-threads=1
CITADEL_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:PORT/postgres SQLX_OFFLINE=true cargo test --locked -p citadel-database --test migrations instance_settings -- --ignored --test-threads=1
```

The restart test starts isolated Core processes and verifies persisted choices,
real logging, omitted Script timeouts persisted with the active default, early
validation phase errors and unchanged existing session expirations. The backup
dispatcher fixture checks that disabled workers retain queued work until a newly
constructed enabled worker starts. Instance
HTTP tests check atomic validation/auditing/notification, deployment HTTPS rejection
without partial settings or success events, conflicting overrides,
concurrent partial saves and enabled-human-administrator reauthorization. Migration
tests retain the preceding checksums and saved installation choices. Frontend
coverage lives in `features/instance/settings.test.tsx`, the sidebar tests and the
existing installation-settings mutation and Script form suites.


### Inventory, health and execution overhead regression checks

The follow-up to F1–F7 applies to ordinary installations, including large inventories:

- `platform_inventory_persistence::overhead` counts real projection updates during
  unchanged Swarm reconciliation, verifies absent-resource removal and zero derived
  counts, and exercises shared health/statistics admission while ordinary SQL remains
  available. `network_references` also exercises more than two network batches.
- `citadel-database`'s `parity::health_retention` upgrades populated history through
  migration `0005`, preserves observations, checks rollback and both local caps, and
  compares indexed expiry with an OFFSET scan using native EXPLAIN buffer counts.
  On the disposable PostgreSQL 18 fixture with 10,080 retained samples, the new
  expiry lookup touched two shared buffers. This is a query-shape check on the
  upgraded schema, not a before/after production memory benchmark. The global
  retention lock remains; this optimization adds a narrow counter write.
- `build_execution::attempts` verifies a multi-row log transaction, ordering, rejected
  stale attempts and rollback of a failed batch. `backup_execution::completion`
  exercises multiple row/byte-limited batches through completion retries and lost
  acknowledgements for both backup and restore.
- `citadel-docker`'s `builds::output` tests byte bounds, UTF-8, redaction across chunks,
  quiet-stream flushing, EOF and producer cancellation on persistence failure.

Run native suites against disposable databases as described above. The new migration
is additive; original migration checksums stay unchanged. No production memory
reduction is inferred from these checks: PostgreSQL cache and allocator retention
need equivalent-load measurements after deployment.

### PostgreSQL review follow-up regressions

Use disposable databases with the existing statistics/execution fixture variables:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test stats_batch_persistence health_retention -- --ignored
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test stats_batch_persistence telemetry_budgets -- --ignored
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib history_admission -- --ignored
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib mixed_log_batches -- --ignored --nocapture
SQLX_OFFLINE=true cargo test --locked -p citadel-server --lib workers::maintenance::tests
```

The retention regression overlaps real insertion and cleanup transactions, checks
commit/rollback, retained ordinals and local caps, and preserves deleted-container
history. The history admission fixture silently withholds TCP responses and checks
bounded cleanup, connection replacement and recovery after repeated faults.
Paused-time worker tests cover source-local stalls, healthy work, all-stalled rest,
physical progress, database-error backoff and cancellation. The mixed-log probe
compares variable-arity inserts with stable typed arrays on one connection, reports
latency and prepared-statement counts, and verifies UTF-8 order and rollback.
Production log inserts retain the existing row/byte budgets and completion fences;
`build_execution` and `backup_execution` exercise their durable completion paths.
These checks do not estimate production RSS savings.

A local PostgreSQL 18.6 follow-up run (unchanged server settings, one connection,
256 rollback transactions with batch lengths 1–128 repeated twice) measured 100
cached statements and 1.156 seconds for variable `VALUES`, versus one statement
and 0.575 seconds for typed arrays. This is a single diagnostic run, not a general
throughput guarantee. With 100 ms delays on database responses, the native
compaction probe committed prefixes of nine and ten complete minutes in 2.95 and
2.64 seconds respectively, within the worker's five-second budget. Worker cadence
is tested separately with paused time. Core/PostgreSQL RSS, database size and
retained queue bytes were not measured in this follow-up.

### Scoped coverage, scheduler projections and Swarm payload guards

Use separate disposable PostgreSQL databases for `CITADEL_PLATFORM_DATABASE_URL`
and `CITADEL_EXECUTION_DATABASE_URL`:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test platform_inventory_persistence -- --ignored --test-threads=1
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test backup_source_discovery --test backup_execution --test script_execution -- --ignored --test-threads=1
SQLX_OFFLINE=true cargo test --locked -p citadel-backups -p citadel-scripts
```

Column-specific audit triggers distinguish unchanged Swarm payloads from required
freshness updates; native tests retain stale-failure, removal and ownership checks.
Schedule discovery traverses more than one page, including 256 KiB Script bodies,
while existing scheduler tests verify due times, timeout validation and idempotency.
Coverage compares scoped and full-platform results, retains shared-volume policy
workloads and incomplete evidence, and excludes unauthorized policy inventory.
No production memory-reduction estimate is implied by these regressions.

### Realtime admission and Stack statistics follow-up

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-server --lib realtime::retention::tests
npm --prefix src/frontend run test:run -- src/features/stacks/form/hooks/useStackInfoGroup.test.tsx
```

The cache tests run eight concurrent readers against fitting, oversized and
capacity-exhausted projections. They verify one loader call per event, shared
successful values, admission-rejection markers without retained projections,
permit release, recovery on a fresh event, and retry after loader errors or
cancellation. The Stack regression starts with an HTTP-only Swarm container,
then delivers compact statistics before lifecycle patches; it checks node identity,
control state, statistics precedence, and unknown/destroyed-container exclusion.
These checks do not measure production CPU or RSS savings.

The 2026-11-13 follow-up run passed all four Stack hook tests, four cache tests,
six maintenance-worker tests, and ten native PostgreSQL regressions (one health
retention, five telemetry budget, three history admission, and one mixed-log test).
Native checks used the commands above against an isolated PostgreSQL 18.6
container with `--ignored` and serial execution for the statistics/admission groups.
Frontend lint, `tsc -b --pretty false`, Rust formatting and `git diff --check` also
passed. The new Stack sequence failed with Running instead of Exited before the
fix and passed afterward. The disposable database was removed after verification.

### Container memory breakdown and latest statistics

The `stats_batch_persistence` suite includes `memory_breakdown` coverage for
active-cache capture, corrected samples, rollups and unknown history. Its
`latest_container_stats_do_not_scan_other_sources_when_redeploy_has_no_samples`
test bounds database buffers with 80,000 unrelated samples, under custom and
generic plans. Use a disposable `CITADEL_STATISTICS_DATABASE_URL` as above.
The database `migrations` suite includes a five-migration upgrade fixture that
checks original sample values and applied checksums after adding the nullable
active-cache column. Use a disposable `CITADEL_TEST_DATABASE_URL`.

Frontend `memory-usage.test.tsx`, `stack-stats.test.tsx` and
`stats-buckets.test.ts` cover the default Breakdown view, persisted switch,
missing measurements and complete replica/bucket coverage. Docker mapping tests
cover cgroup v1/v2 and shared-memory separation.

### In-app update setup

`python3 -m unittest discover -s test/release -p test_update_setup.py` exercises
`deploy/install/enable-updates.py` with disposable private files and a simulated
Docker daemon. It checks ordered overlays, immutable helper images, rootless
executor identity, preflight-before-activation, Core-only recreation, idempotent
verification, secret-safe failures and preservation after drift/restart/refresh
failures. Existing adapter bootstrap tests own registration, database identity,
mount and recovery-state validation. Updates frontend tests cover setup messaging
even when Core has no available release and retain distinct external, unknown
and blocked PostgreSQL states.

Run the actual activation command against base and Edge Compose fixtures with:

```bash
python3 test/scripts/test-update-registration.py <local-core-image-tag> --enable-updates
```

This creates an isolated disposable Docker daemon, installs Python inside that
fixture before disconnecting its network, runs normal first-run setup and the
shipped activation command, then verifies unchanged PostgreSQL identity/start
time, unchanged Core image and recovery keys, valid refreshed registration and
no restart on a repeated activation. All fixture containers/volumes are removed.
