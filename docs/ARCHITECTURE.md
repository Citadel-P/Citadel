# Citadel architecture

This guide defines where code belongs and the boundaries to preserve when changing
Citadel. For setup and commands, see [Development](DEVELOPMENT.md). For the wire
contract between Core and Agent, see [Agent protocol](agent-protocol.md).

## System overview

Citadel Core serves the HTTP API, frontend and realtime connections. It stores
configuration, authorization, activities, operation claims and runtime projections
in PostgreSQL. Background workers collect observations and execute queued work.

Core reaches Docker through one of three paths:

| Path | Connection | Execution owner |
| --- | --- | --- |
| Local | Core connects to Docker | Core's Docker adapter |
| Direct Agent | Core calls an Agent over gRPC | Agent |
| Edge Agent | Agent maintains an outbound gRPC stream to Core | Agent |

Agents execute Docker and external-tool operations without a database.
The `citadel-docker` crate implements feature ports for local execution; it is not
foundational infrastructure. Agent normal dependencies exclude Core adapters,
SQLx, database migrations and identity cryptography. Core keeps Direct/Edge
routing, target selection and database-aware build and backup orchestration. Core owns
persistence, user authorization, target selection and operation scheduling. Build
Pools use the same Agent executable; Swarm-node Agents use a restricted profile.

## Repository layout

| Location | Responsibility |
| --- | --- |
| `src/server/` | HTTP/realtime presentation, composition, startup and workers |
| `src/agent/` | Agent configuration, authentication, Direct/Edge dispatch and lifecycle |
| `src/features/` | Business models, use cases, typed errors and consumed interfaces |
| `src/infrastructure/adapters/` | PostgreSQL, remote Agent routing, filesystem and security implementations |
| `src/infrastructure/docker/` | Shared local Docker, Compose/build execution and transport mapping for Core and Agent |
| `src/infrastructure/database/` | Declarative schema, migrations and migration runner |
| `src/infrastructure/scripts/` | Shared private script workspace and owned cleanup adapter for Core and Agents |
| `src/infrastructure/contracts/` | Core/Agent protobufs and generated bindings |
| `src/infrastructure/docker-api/` | Generated Docker protocol client and pinned source schema |
| `src/infrastructure/processes/` | Bounded subprocess execution, cancellation and cleanup |
| `src/infrastructure/runtime/` | Task supervision, queues, I/O budgets and metrics |
| `src/frontend/` | React application and generated API client |
| `src/tools/` | Generators, build scripts and development tooling |
| `src/.sqlx/` | Offline SQLx query metadata |
| `test/` | Cross-system fixtures, test runners and performance tools |
| `deploy/` | Compose files, environment templates and local settings |
| `schema/` | Generated full and public OpenAPI documents |

Crate-specific tests stay beside their crates. Cargo's workspace and toolchain
files remain at the repository root.

## Instance update work in progress

The Updates feature owns the release-check service, policy validation, release
selection, compatibility descriptors, reviewed plans and operation/journal invariants.
Its release-source interface is implemented by the bounded GitHub HTTP adapter;
Server owns scheduling, composition and HTTP presentation. Adapters own release HTTP access, private
filesystem journals and transactional PostgreSQL state. Identity remains the
owner of the installation-wide check setting; its write transaction emits the
Updates activity and a separate MFA activity when both settings change.

The running application currently exposes persistent release checks and the
administrator-only Config/Activities page. The page uses the shared resource registry,
tabbed resource view and FormShell; it does not define a separate application route.
Committed settings mutations publish `InstallationSettingsChanged` to administrator
browsers, while persisted checks publish `ApplicationInfoChanged`. Shared realtime
handling refreshes the affected HTTP reads on notification and reconnect. Activity
history uses its existing scoped subscription; mutation responses keep the initiating
browser's saved values current during a disconnect. Registered Linux Compose
installations also expose review, approval and retained operation progress. The
executor is wired through target completion, but no release transition is qualified
yet; packaged source-to-target acceptance remains required.

The Updates UI keeps Config and Activities in the standard resource page.
Config offers automatic release checks at six-hour, twelve-hour or daily intervals
(six hours by default), with Check now beside the frequency selector. Disabling
automatic checks disables the frequency selector but preserves its value and still
allows manual checks. Frequency is persisted with installation settings, with six
hours as the migration default. The supervised worker uses the last persisted
attempt to calculate the next check and reschedules on settings changes. Manual
checks and restarts preserve this schedule. Server policy can prohibit both
automatic and manual checks. History remains visible independently of this setting.

The installation UI offers Review update when a release is available and the
installation runtime is configured. The server verifies registration, signatures,
transition eligibility and Agent requirements before returning a review. Approval
starts a durable operation shown in the Updates status alert. Channel switching
and scheduled Core installation are deferred. Closing it, navigation or a browser refresh must neither cancel nor
repeat the operation. Realtime reports progress while Core is available; during
restart the loaded page retains the last confirmed phase and reconnects, without
inventing progress or offering actions that require Core. The external executor
retains durable evidence for reconciliation when Core returns.

During a backend-confirmed update restart, keep the header connection indicator
visible with a neutral spinner and “Updating · Reconnecting…”. Suppress repeated
connection-error toasts only for that expected interruption. Ordinary disconnects
retain the existing warning. A prolonged restart becomes “Taking longer than
expected”, with operation details and host status/recovery instructions; loss of
connectivity alone proves neither failure nor success. After verified completion,
the Updates page reloads the document once to load the new frontend, preserving
the URL. It requires no active maintenance owner, the retained
successful outcome, a connected realtime session and the matching running version.
A session marker is persisted before reload; storage failure leaves a manual action
instead. After reload, confirmed completion clears the session hint and banner and
shows a success toast. Failed and recovery-required outcomes show an error toast
once per operation per tab, retaining the recovery banner and operation hint.
The durable operation remains in Activities. Release-check notices are hidden while
an operation owns maintenance. A committed success still owning maintenance is shown as Finishing update. A fresh browser load may be unavailable while Core is down;
the first version adds no independently hosted updater status endpoint.

Release-check cancellation stops lock waits and network fetching. Once persistence
begins, the supervised worker awaits cache publication and notification before
exiting or applying a settings change. Policy monitoring and the worker are joined
on shutdown; neither drops a committing check. The check endpoint returns the
service result without rereading check state.

Agent updates share the Updates page, release verification, Activities and realtime
notifier. They have explicit installation targets and one durable sequential selection;
Core and Agent claims share the identity mutation lock and cannot overlap. Reviews
expire after five minutes, approval request IDs replay exactly, and progress uses
revision-checked repository writes with Activity events in the same transaction.
The supervised Agent worker uses the existing single-database background lease.
Interrupted targets are observed, never redispatched. Each new target rechecks the
requester's administrator access or the scheduled operation's Team entitlement.
Shutdown during dispatch or verification preserves the target's Updating state.
Swarm cleanup uses the reviewed node IDs stored in the operation. Missing sessions
or resume acknowledgements keep the operation owned; the worker retries after
reconnect and cannot start the next target or release the claim before cleanup.

The supported standalone Docker executor drains affected Agent work and launches a
separate container from the verified target image. It preserves configuration,
identity mounts and host ports, retains previous containers for recovery, and
requires authenticated reconnect with the expected image/version and changed
container identity. Managed Swarm services reuse NodeAgentSetupService with a
bounded node-local drain; Repair uses the stored approved digest. Mixed or stale
node observations cannot establish successful rollout. Unsupported profiles remain
manual. Additive update RPCs do not relax the Swarm mutation allowlist: node Agents
allow metadata and drain/resume, but reject standalone self-replacement.
Drain blocks new work while allowing the existing identity-fenced data, build and
script reconciliation handlers to release retained admission capacity.

The Team Agent schedule reuses CronSchedule and the Script form fields, saves
explicit membership, and claims only the current minute. Busy/missed occurrences
are skipped, with no queue or catch-up. Manual selection never edits the schedule.
Core-current and Agent-current status are evaluated independently; the Agent
release must match the running Core's verified release identity.

The operation repository, detached executor and target startup share persisted
ownership through draining, checkpoint creation, replacement and acceptance.
Packaged source-to-target upgrade and recovery tests are still required before
a release descriptor can authorize installation.
Matching journal identity fields alone is not target readiness verification.

On execution failure, the coordinator first attempts verified unchanged-source
reconciliation. If safety cannot be established, it imports an available owned
terminal helper journal before notifying the UI and retains the claim. The host
utility reads an exited helper's journal under the installation lock after checking
its exact identity and exit. This fallback read alone never releases maintenance
or retries replacement; missing or ambiguous evidence remains unresolved.

Core now shares a process-local `AdmissionGate` between HTTP handling and work
dispatch. A closed gate rejects ordinary HTTP requests with a no-store 503 and
`Retry-After`, including readiness; liveness, authorized update/activity reads,
application information and realtime remain available. New container terminals
also require admission; existing terminal streams retain it until they close and
can still receive input while maintenance waits. Response streams retain
their admission until completion or disconnect. Detached durable operations keep
their existing task ownership independently of that response lifetime.

Build, script, ordinary backup/restore, Git, workload-update, drift-repair,
pruning and alert-delivery dispatch wait at the same gate before claiming work.
Admitted executions retain their permits through finalization. Closing the gate
does not cancel them; queued work stays queued. Drain timeout or cancellation
leaves admission closed, and only the matching maintenance owner may reopen it.
Cleanup, operation reconciliation and Agent callbacks remain available to finish
existing work. Resource-copy start and cleanup requests are gated at HTTP; their
committed parents remain drain blockers while execution and reconciliation continue.
During maintenance the restore dispatcher may claim only queued children of
`Restoring` copy parents, selected in the same transaction as the normal claim.
Unrelated restores stay queued; cancellation, authorization and lease checks remain
unchanged. Remaining volumes and parent finalization must complete before idle.

`UpdateDrainService` binds the gate to an already committed approval through
feature-owned persistence and runtime ports. The PostgreSQL reader checks the exact
operation, fence, revision, instance and retained ownership in a bounded, read-only
snapshot. Released, corrupt, foreign or advanced claims are rejected. Its work
counts include active workload operations, run dispatch, multi-stage copies,
executor reservations and backup leases. Expired leases and terminal run status
do not erase retained cleanup obligations; ordinary queued work stays queued.

The Server runtime adapter also counts detached `DynamicTasks` and cleanup
reservations after HTTP response lifetimes end. Local observations bracket each
database snapshot so a local execution finishing with a new remote reservation
requires another observation. The bounded drain returns idle or named blocker
counts; cancellation, timeout and observation failures never release ownership or
reopen admission. Counts are per category and can overlap; they are not a total
number of independent operations. The drain coordinator must run under a separate
supervised owner, outside the `DynamicTasks` tracker it is waiting to empty.

This drain step does not advance journal phases or authorize a checkpoint. The
operation driver must preserve the complete journal transition history, quiesce
remaining observation writers and revalidate evidence before checkpointing. Safe
postponement/resume still requires unchanged-source and cleanup verification. The
installation coordinator invokes the drain service after helper readiness; release
checks alone do not close admission. The startup safeguards below remain the authority across
process restarts.

The checkpoint adapter prepares local recovery artifacts under an already held
installation lock and an exact `BackingUp` journal. It binds registration, instance,
source version and ordered installer inputs to the approved operation. It reuses
the system recovery bundle (PostgreSQL custom archive, checksums and file-backed
keys), saves the original Compose inputs and private `.env`, and records external
key requirements. The checkpoint-specific builder reads the existing database
identity without creating or repairing it; ordinary first backups retain their
existing identity initialization behavior.

Each attempt gets a new private directory within the registered state directory.
Files and directories are synchronized before an exclusive preparation receipt is
published. Changed inputs/journal/registration, cancellation, dump failure or
storage failure cannot return successful preparation. An interrupted attempt is
retained and cannot be silently reused or deleted on retry. The preparation receipt
binds the bundle checksum inventory and installer input hashes; it is artifact
evidence, not runtime quiescence or restore verification. No operation phase or
maintenance ownership changes as part of this filesystem preparation.

Before replacement, `verify_checkpoint_integrity` rechecks the owned backup-phase
journal, registration, saved installer inputs, bundle checksums and source identity.
Preparation and verification share the same operation/registration validation.
The helper does not create a second PostgreSQL container or restore the checkpoint
as part of an update. The checkpoint is **integrity checked**, not **restore tested**.
Real restore and encrypted-data round trips belong in disposable release qualification
and the shared system-recovery tests. Integrity checks alone cannot prove that
PostgreSQL will restore an archive successfully.

The helper checks recovery-key availability before shutdown and validates the source
identity, retained claim and migration catalog before dumping. Target startup still
verifies the original keys, migration history and decryption of stored credentials
before recording completion. Credential reads use bounded parameterized SQL in the
target executable, without a Docker/JSON database-query transport. Missing or malformed
required envelopes fail verification; nullable OIDC client secrets remain optional.

The external helper must acknowledge ownership before stopping Core, then confirm
source writers have stopped before taking the checkpoint while PostgreSQL remains
available. Neither artifact integrity nor a successful dump releases maintenance.
The detached helper implements that boundary behind the review/approval API.
Host reconciliation and explicit checkpoint recovery are connected to the same
retained ownership; their packaged source-to-target acceptance is still pending.

**Execution ownership**

`run_update_helper` is the single owner of the installation lock and host journal.
Its sequence is target preparation → shutdown grant → clean source stop → checkpoint
integrity verification → Core-only replacement → target completion. The individual
phase methods are private implementation details, not separate executable APIs.
The launcher records exclusive launch intent, then binds the verified helper
immediately before starting the exact pinned source image. It releases the host lock before starting the helper. Ambiguous Docker
responses retain evidence and never permit an automatic relaunch or mutation retry.

Core and the replacement target use one private `update-helper-handoff` command.
The request carries the reserved operation, exact helper binding/start identity,
and one action: check acknowledgement, grant shutdown, inspect target evidence,
or submit target completion. The command validates ownership before dispatch and
cannot execute a phase. It only reads evidence or publishes the exclusive grant or
completion mailbox. The driver alone writes journal transitions. Core uses bounded
Docker exec, addressed directly from the request's helper binding; it does not
open private host paths or need an additional host mount. Running phases share
one verification routine for the full helper profile and acknowledged start time,
so preparation, shutdown, replacement and completion retain the same process identity.
Protocol types remain private to the adapter; the public ports expose use cases.

Each operation has five records: launch intent and start binding from the launcher,
one helper-owned execution journal, Core's shutdown grant, and the target's completion
mailbox. The execution journal contains the phase history, helper incarnation,
preparation, stop identity, replacement intent/submission and target identity.
Acknowledgement/history, replacement intent/phase and target identity/phase are each
published atomically, with no separate phase markers to reconcile. The grant and
completion stay separate because other processes write them; they cannot overwrite
the driver's journal. Phase checks consume one complete execution snapshot, including
preparation and process identities, and compare it again after runtime I/O before
advancing or accepting target evidence. A readiness check may observe newly finished
preparation; it does not grant shutdown. Checkpoint files remain independently verified recovery artifacts.
Standalone checkpoint journals retain their format. Existing fragmented execution
state is preserved and blocks relaunch; it is never converted into permission to replay.

The helper has a read-only root filesystem, the registered installer/state/socket
binds at their original paths, and Core data mounted read-only at `/app/data`. It
shares the exact PostgreSQL container's network namespace. Its profile prohibits
privileged execution, restart policies, automatic removal and HTTP health checks.
Docker copies the network donor's hostname when this helper starts. The driver
therefore proves its process identity by comparing its private mount namespace
with a Docker exec in the exact validated helper, rather than treating the shared
hostname as a container ID. Registration utilities retain their default-hostname
identity check because they use a private network namespace.
Readiness waits for the initial journal while validating the same running helper;
shutdown grants still require its persisted preparation and acknowledgement.
A root helper may read official Core UID 65532 keys only below `/app`; symlinks,
permissive key modes and other owners remain rejected. Ordinary registration/key
checks keep their original ownership restrictions.

**Preparation and replacement**

Before shutdown authorization, preparation verifies recovery-key/database access,
the exact signed target and supported source transition, and the reviewed Agent
versions. It compares the actual ordered Compose inputs with the proposed image
change. Only the Core image and its `CITADEL_IMAGE` selector may differ. The durable
receipt belongs to the exact helper incarnation and must be fresh when granting
shutdown. Core's read-only readiness check waits for this preparation before draining;
a raw helper acknowledgement by itself does not authorize shutdown. Source preparation
and target binding use the same bounded pending exchange, which checks helper
incarnation between reads. Grants and completion writes are never retried by it.

After Core drains, the helper records stop intent before sending SIGTERM to the
registered Core with Docker's `stop --timeout -1`. The client has a bounded deadline;
there is no daemon-side SIGKILL escalation. Only a clean exit with unchanged
container/image/start identity and a healthy PostgreSQL advances to `BackingUp`.
Checkpoint creation reuses the system recovery builder; integrity is checked before replacement.
Source/helper identity and Agent versions are rechecked; pools close before return.

Replacement validates the checkpoint integrity, pinned image and
approved configuration. It records replacement intent and `ReplacingCore` before
atomically publishing the private `.env` image selector. The original inputs remain
in the checkpoint. The original Compose invocation runs only
`up --detach --no-deps --no-build --pull never server`; PostgreSQL, volumes and Compose
files remain intact. A successful response records submission, not readiness.
There is no source-schema SQL after replacement submission.

`ExecutionJournal::read_replacement` centralizes the post-publication validation used by observation,
target startup and completion: operation/history, grant/stop ownership, approved
configuration, checkpoint identity and retained runtime. Observation requires
confirmed submission. It binds the exact replacement container/start identity after
two consistent reads and publishes that identity with `Verifying` in one atomic
journal write. A mismatched phase/identity is rejected, not repaired. It cannot
accept a restarted or different target. Docker health never establishes completion. Target inspection
may report pending only for the owned `ReplacingCore` phase, while Compose starts
Core before the driver publishes the binding. Retrying that read is bounded and
never repeats replacement.

**Review and approval**

The Updates API exposes administrator-only review, approval and operation reads.
Review checks live registration, signed release identity, the embedded qualified
transition and current Agent versions, then saves a short-lived plan. Approval
rechecks the source and claims atomically with its Activity. A bounded supervised
worker owns review cleanup and approved execution independently of the HTTP
receiver. The same request key returns the stored operation, including after
release; it cannot launch another helper. One coordinator launches, waits for
acknowledgement, imports the driver's Draining journal, drains work and grants
shutdown. No separate handoff coordinator or browser-owned task is used.

The frontend uses the existing Updates page, generated API models and realtime
invalidations. It retains an operation ID in session storage for reconnection,
never synthetic progress. An approved restart gets a bounded explanatory header
status; browser offline and expired expectations retain the normal connection
warning. Public views omit host paths, keys, fences and private journals.

**Target acceptance and completion**

Linux target startup runs under the same monitored Core job lease, before ordinary
migrations or services. It requires the exact planned build/catalog/architecture and
its own verified container identity. It checks the source migration catalog as an
unchanged prefix of the target catalog, migrates using target-owned code, then
checks the complete catalog and decrypts stored credentials. It revalidates the
live execution around migration and completion. File-backed JWT/encryption/Agent
keys, including loaded values, must match the verified checkpoint even when there
are no encrypted database rows. External keys stay bound by the unchanged Compose
environment. Private key identities are redacted and never enter public views.

The target atomically commits outcome, receipt and Activities before submitting
completion through the helper command. Persistence maps operation rows through one
claim validator: indexed identity, actor, request key and revision must match the
serialized operation. Import and release hold the row lock throughout validation
and mutation. A released attempt cannot advance; exact replay of a released
successful or safely stopped outcome is read-only and cannot affect a newer owner. The driver independently asks the target's
`update-target-receipt` command to confirm that database commit; the source never
reads target-schema SQL. It then rechecks target identity, acknowledges completion
in the host journal and exits. Completion retains one execution snapshot across
both live checks and receipt verification; changed preparation or replacement
evidence prevents acknowledgement. Completion and failure update that complete
snapshot directly, without merging a phase into freshly read execution state.
That journal is the acknowledgement record; there is no second completion marker
to publish. Target startup verifies that exact clean
helper exit using the same container profile checks as creation and running-state
verification. The session retains its original helper acknowledgement; exit code zero
is insufficient if the image, process incarnation, labels, command or mounts changed.
The target invokes the installed image's bounded `update-host` reconciler after
clean helper exit. Under the installation lock it rechecks the committed receipt,
target and helper, removes only the exited helper (without force), updates the
registration to the target and archives active records alongside the unchanged
checkpoint. A completed archive is validated before startup treats the state as
idle. Only then does target-owned persistence release the claim and normal startup
continue. A fully reconciled archive can finish an interrupted database release on target
restart after rechecking image, container, configuration, storage, keys, catalog and
committed receipt. A private finalization record is persisted after clean helper exit
and target receipt verification, before cleanup. It permits exact interrupted archive
moves to finish after revalidation, without replaying container replacement. Pre-stop
failures retain maintenance as `Failed` until unchanged-source reconciliation succeeds;
ambiguous post-stop failures record `RecoveryRequired`, preserve evidence and never
restart the source or roll back the database automatically.

Core acquires its database job lease and checks update ownership **before**
migrations, setup, abandoned-operation recovery or service construction. The
PostgreSQL adapter uses a bounded, read-only inspection; a fresh database without
update tables is allowed, but missing update state after its migration is not.
An owned operation blocks startup even when its JSON is unreadable, its phase is
terminal, or release checks are disabled.

Lease ownership is verified before each initialization phase and monitored while
it runs, including migrations, setup, recovery and worker registration. Losing
the original database session aborts startup; it does not reacquire the lease or
fall back to maintenance listeners. Each subsequent phase checks ownership again
so a completed migration cannot bypass a lost lease between periodic checks.
Normal shutdown retains the lease through worker and dynamic-task cleanup.

When `CITADEL_UPDATE_STATE_DIRECTORY` is explicitly configured, the Linux adapter
also checks the existing private registration under a short, nonblocking host lock.
Containerized Core uses a short-lived utility from its actual installed image,
with exact host-path binds for installer inputs (read-only), state and the local
Docker socket. The utility runs as the registered root executor, with no network
and a private temporary filesystem. Core does not need writable host-root access. Missing, unsafe, corrupt, foreign-instance or
locked state blocks startup. The check never creates or repairs evidence, holds
the host lock while awaiting a helper, or treats a restored database without a
claim as permission to ignore external evidence. Active or orphaned evidence requires recovery verification. Completed checkpoint
archives are permitted only with validated completion or unchanged-source recovery
and registration identities; a standalone successful journal does not authorize startup.

Blocked startup serves a restricted listener: `/health` reports process liveness,
`/ready` returns 503, application routes are unavailable, and no Edge RPCs are exposed.
Existing transport host/proxy checks, request limits and security headers still
apply, and static frontend serving is disabled.
The database job lease remains watched; loss of that lease stops the listeners.
No normal workers, setup, recovery, webhook or scheduled dispatch is constructed.
This mode does not reopen itself; restarting reevaluates the evidence. Automatic
source/target resume still needs live runtime, migration, receipt and helper-cleanup
verification. No target migrations or journal imports run in this mode. This is
a startup safeguard, not the live admission/drain mechanism for an already running
Core. Explicit operator migration/restore CLI commands remain separate maintenance
tools. Registration is explicitly configured; without that directory only database evidence can be inspected.

The standalone `verify-update-release --version VERSION` command resolves a
completed official publication and verifies its exact Core image using Cosign.
The feature validates publication identity; the adapter owns bounded HTTP and
signature-tool execution. Expected certificate identity and issuer come from
Citadel's policy, not downloaded trust settings. The verifier uses a temporary
private HOME, no inherited Docker credentials, and cancellation-aware process
cleanup. Checksums and publisher receipts alone never establish authenticity.
The command honours the server release-network restriction and does not construct
Core, access the database, pull images or install anything. Its result proves an
artifact signature, not catalog, architecture or installation compatibility.
The verifier feeds the target-inspection service used by review and detached execution.

Core also exposes `upgrade-descriptor-json` from its compiled version and embedded
migration catalog. The catalog identity is SHA-256 of the exact embedded
`migrations.json` bytes, after validation of every embedded SQL checksum; it is
not a fingerprint of a live database. Candidate acceptance checks the descriptor
on each architecture and retains it with the accepted archive evidence. The public
manifest includes that retained copy as metadata, not as independent trust proof.
The compiled descriptor currently has no qualified source transitions. A consumer
must verify the image signature before reading its descriptor, then establish
installation and live database compatibility before entering maintenance.

The Linux `inspect-update-release --version VERSION --architecture amd64|arm64`
diagnostic verifies the official image signature before pulling its exact digest.
The feature service validates requested and returned identities and target descriptor
metadata. Pull and configuration inspection both select the requested platform
(inspection requires Docker API 1.49+). The Docker adapter verifies the selected
Linux image architecture/digest,
rejects image-declared volumes, and runs only `upgrade-descriptor-json` in a uniquely
labelled, non-root, read-only container. The probe has no network, host mounts,
credentials or Docker socket, drops capabilities, disables healthchecks/restarts and container log storage,
and bounds CPU, memory, PIDs, runtime and output. A separate identity-only inspection
resolves the locally addressable image ID, since a platform manifest ID need not be
addressable in Docker's image store. Creation uses that local image ID and the
requested platform with pulling disabled; it does not resolve a mutable tag.

The probe owner awaits bounded cleanup, including after cancellation, malformed
output and ambiguous create results. Cleanup requires the generated name, ownership
label and image identity before removing the container by ID. A cleanup failure
returns no successful result and identifies the probe for operator inspection;
it never removes unrelated containers or the cached image. Cancellation does not
prove a timed-out daemon-side create has finished: a failed diagnostic can require
manual inspection for a late-created, unstarted probe. Process/host crashes likewise
require checking the labelled probe. The diagnostic does not touch installation
containers, volumes, registration, journals or the database. Its validated target
identity and embedded descriptor are not a reviewed plan, live database compatibility
proof or installation approval. An empty source list still qualifies no transition.

The Linux `check-update-compose` diagnostic validates the explicitly supplied
installer directory, project and ordered Compose paths. The adapter captures
protected inputs, resolves a private snapshot through the bounded ProcessRunner,
and rejects changes to the originals during validation. No installer templates
are embedded in Core or Agent. A bounded YAML profile check rejects includes,
builds, extra environment files and other unsupported dependencies before Compose
can read them. Resolved configuration must preserve the supported service,
network and persistent-storage layout; comments, filenames, labels, ports and
explicit resource names need not match the shipped examples.
The result contains a configuration fingerprint and desired image/storage names,
never resolved environment values. This check does not contact the Docker daemon,
bind the configuration to a live instance, persist registration, or establish
upgrade eligibility.

`register-update-installation` binds those inputs to a stable, healthy local
Compose installation. Bounded Docker inspections verify the explicit invocation,
service environment, ports, mounts, exclusive local volume ownership and bridge
network identity. The command reads the existing initialized instance ID through
Core's read-only `instance-identity-json` diagnostic; SQL remains in the PostgreSQL
adapter, and neither diagnostic initializes Core, runs migrations or creates an
identity. Repeated observations and an input fingerprint reject concurrent drift.
Containerized registration also checks its own Docker identity and exact host
binds for read-only installer inputs, private update state and the socket.

Registration retains the installation lock and atomically creates a private
`installation.json` without replacing existing evidence. An identical retry
returns the same registration; different or corrupt registration and orphaned
recovery state are refused. It records the executor UID/GID, `DockerCompose`
installation kind and configuration and runtime fingerprints, not environment
values. The registration model and
bootstrap adapter are explicitly Compose-specific. Shared release verification,
operation state and journals do not depend on Compose templates. Their current
helper-image, container-ID and absolute checkpoint-path contracts still describe
the container/host executor; they do not qualify another execution backend. Future
Podman or Kubernetes installation support requires its own qualified adapter and recovery
semantics; no unsupported kind is accepted or routed through Docker Compose.
Registration is local operator authority, not an administrator-approved update or a user Activity. The installer
does not yet invoke it automatically. Startup checks registration identity and
private evidence. The local `check-update-installation` diagnostic now revalidates
the saved invocation and live runtime using the same bounded inspection as registration.
It requires existing protected registration and lock files, the recorded executor
UID/GID and socket, and no execution evidence. It compares stable observations of
the daemon, services, storage, network and initialized instance plus input fingerprints
against registration, and rereads registration/evidence before returning. It holds
the nonblocking installation lock for the check and releases it on every result;
it never repairs or rewrites registration or recovery evidence. The source Core's
`instance-identity-json --verify-migrations` reads identity, update ownership and
migration history in a bounded read-only transaction. Every embedded migration must
be present, complete, without failure evidence and with its exact checksum; unknown
entries are rejected. The database crate owns catalog comparison, reused by the
PostgreSQL adapter. This checks the migration journal, not arbitrary manual schema edits.

Preflight also assesses the registered state directory as local checkpoint storage.
It requires available space of twice the current database size plus 64 MiB and
creates, synchronizes and removes a private 4 KiB probe through the anchored
directory descriptor. Registration stays unchanged; a crash can leave a probe,
which is retained as evidence and blocks admission until inspected. Space is an
admission budget, not a dump-size guarantee or reservation against concurrent use.
Preflight also invokes `check-update-recovery-keys` inside Core as the packaged
service UID/GID (65532:0). It checks the existing JWT, secret-encryption and Agent
signing material using the same sources and encodings as startup and system
backup: UTF-8 JWT material of at least 32 bytes, a Base64-encoded 32-byte encryption
key and a raw 32-byte Agent signing seed. It never generates, copies or prints
keys. File reads are bounded to 16 KiB, zeroized, protected against symlink traversal
and require private permissions and trusted ownership. Missing, changed or
malformed material blocks preflight. Explicit JWT/encryption settings take
precedence; their sources must match protected installer configuration already
compared with the running container environment. An image-only environment key
cannot silently become an unrecorded recovery dependency.

The result contains registration, validated database identity/catalog/size,
`recoveryKeys` source classifications and checkpoint storage capacity. Key
availability does not prove that the values match Core's in-memory keys or decrypt
all stored ciphertext, and it is not an independent copy. This is a point-in-time
preflight input, not target compatibility, a created/restorable checkpoint or
replacement approval. The executor repeats validation under ownership before preparing
the checkpoint and replacing Core.

Migration `0002_instance_updates` appends update state without changing the shipped
baseline. Discovery markers and operation receipts are separate from Activity
retention. Update state and the corresponding Activities commit together; a
completion journal cannot reopen an older restored source identity.
An attempt that fails or is postponed before source shutdown can release ownership
after its outcome commits, helper cleanup completes and the running source's
version, image, catalog and architecture are verified unchanged. Its original
outcome stays terminal. Replacement uncertainty and `RecoveryRequired` retain
ownership; successful replacement still requires the target completion receipt.
These repository checks do not substitute for live runtime readiness and cleanup
verification. An explicit `restore-update-checkpoint` command supports recovery into
an empty database using the exact source build. It holds and monitors the Core lease,
validates the immutable checkpoint, restores through the shared system recovery
implementation, then checks catalog, claim identity and credential decryption before
atomically recording failure and releasing the restored claim. Original host evidence
and the original database remain intact; recovery never fabricates target success.

### Update interruption reconciliation

Installation requests use one nonblocking reservation covering queued and executing
work; accepted replay reads the authorized durable operation before review expiry
or queue admission. The worker retains the reservation after the HTTP reply.

Recovery separates executor history from database ownership. Before source shutdown,
the host reconciler stops only the exact owned helper, verifies that no stop or
replacement was recorded, verifies the unchanged source and archives the attempt.
A permanent recovery record fences delayed launches and grants. The repository then
checks the source catalog and claim, records failure and releases ownership atomically;
only afterward may process admission reopen. Unresolved failures retain ownership
and can be retried with `recover-update-source` in the original Core.

For a committed target receipt, startup reconstructs the exact completion from the
retained Verifying history and verifies the receipt through target-owned code. It
can retire a failed executor and finish archival without rewriting that executor's
history or claiming a clean exit. Same-container policy restarts are allowed only
in this receipt-backed path; first target acceptance still rejects restarts. Target
identity, configuration, storage, retained PostgreSQL incarnation and recovery keys
must match before database ownership is released.

Update compatibility includes enabled, nonarchived build pools. Direct pool health
checks persist the signed Agent version under the existing configuration fence;
unobserved pools block eligibility. Edge bindings clear their observation on
reconnect and accept version updates only from the authenticated current session.
The binding, rather than duplicate ordinary Platform metadata, supplies Edge version
evidence. No production source transition is qualified by the synthetic lifecycle
fixture.

## Dependency rules

- Features may depend on other features, but never on Server, Agent or infrastructure.
- Infrastructure implements feature interfaces and may depend on features or other
  infrastructure crates. It must not depend on Server or Agent.
- Server and Agent compose features and infrastructure. They must not depend on
  each other.
- HTTP types, `utoipa` schemas and API views belong in Server. Generated Docker and
  Agent protocol types stay at infrastructure/transport boundaries.
- Cross-feature dependencies must be acyclic and have a clear owner. Prefer an
  existing narrow interface over a new shared framework.

`primitives` owns small shared concepts such as actor IDs, permissions, audit
metadata and patch vocabulary. `execution` owns process request/result types and
the `ProcessRunner` interface. Concrete process execution belongs in infrastructure.

The architecture tests inspect Cargo dependencies as well as source boundaries:
[resource_structure.rs](../src/server/tests/resource_structure.rs) and
[resource_architecture.rs](../src/server/tests/resource_architecture.rs).
They discover every workspace crate, including target-specific, renamed, build and
development dependencies. Features cannot import database, HTTP or protocol
implementations. Foundational infrastructure stays independent of business features;
the process runner's dependency on the Execution port is intentional.
Feature task checks parse production items and supported macro token groups,
including `select!`, and resolve ordinary imported task aliases. Comments and
string literals are not executable dependencies. Definitely test-only items are
excluded; potentially production `cfg` branches remain checked. These checks do
not expand macros or perform compiler name resolution. Injected task-owner ports
remain allowed. The sole feature-owned blocking-task exception is Identity's
`CredentialWork`, whose semaphore and cancellation lifetime have behavioral tests.
Request cancellation is verified through HTTP/Edge cleanup, not guard spelling.

Source checks cover all features, HTTP/realtime handlers, workers and Agent code,
and keep SQL execution and row mapping in PostgreSQL persistence modules. Test-only
fixtures may use concrete implementations. These guards detect dependency regressions;
they do not replace review of authorization, transactions or business ownership.

## Organizing a feature

Use responsibility-based files within the owning feature. Add only the roles the
feature needs; split a large file into a module when that makes navigation clearer.

| Role | Contents |
| --- | --- |
| `model` | Resource state, value objects and invariants |
| `commands` | Transport-neutral mutation inputs |
| `service` | Use-case orchestration |
| `repository` | Durable persistence interfaces and atomic operations |
| `read_models` | Summaries, configuration results, filters and previews |
| `runtime` | Interfaces for Docker, Agent or external execution |
| `permissions` | Named operation requirements |
| `jobs`, `adoption`, `webhooks` | Feature-specific workflows |

Keep crate roots and module facades focused on declarations and deliberate exports.
Use `name.rs` for a single-file module. When a source module needs child files,
keep its entry point at `name/mod.rs` alongside those children, including private
unit tests in `name/tests.rs`. Keep new or refactored `lib.rs` and `mod.rs` files thin:
module declarations, deliberate re-exports, crate/module documentation and applicable
attributes (such as lint or conditional-compilation attributes). Place implementation
in named sibling files organized by responsibility. Separate orchestration,
external I/O and lifecycle management when they are distinct responsibilities;
keep cohesive helpers together rather than introducing a file per function.
Existing large entry points are not a precedent for new work, and unrelated
modules do not need restructuring as part of a focused change. Preserve public
import paths and keep internal visibility as narrow as practical when splitting.
Integration tests retain Cargo's `tests/name.rs`
entry points and may keep their supporting modules in `tests/name/`.
Do not introduce empty services or generic base entities just to make every
feature look identical.

### Models and persistence

Use the business resource name, such as `Deployment` or `BackupPolicy`. Resources
may include tags and related metadata loaded efficiently by their repository;
a separate `*Details` wrapper is not mandatory. Use smaller projections when a
list, worker or lookup only needs a subset of the data.

Reuse shared vocabulary where semantics match. Resource-specific state and
validation stay with the feature. Decode persisted enum values and JSON into
feature types at the repository boundary.

Use `Repository` for durable business resources, `Store` for projections, samples
or transient state, and `Reader` for read-only queries. Keep claims, row-version
checks, idempotency and multi-resource transactions in cohesive operations.
Do not replace one atomic operation with several unrelated repository calls.

Worker dependency construction lives in `server/src/composition/workers.rs`.
Executable background-task startup lives in `server/src/composition/background.rs`;
worker implementations remain in `server/src/workers/`. HTTP extraction and
validation belong to `server/src/api/request_validation/`. Citadel release
notifications are scheduled in `server/src/workers/updates/`, separate from workload image updates.
Changes to the cached release result notify administrator browsers through
`ApplicationInfoChanged`. Application info is fetched initially and refreshed on
that event and realtime connection recovery, without periodic browser polling.
Running workers receive feature persistence interfaces or typed notification sources,
never SQLx pools, rows or listeners. PostgreSQL notification decoding and dedicated
LISTEN connections live in the persistence adapters; reconnect events trigger resync
and durable queues retain their fallback scans. Edge observation persistence keeps
session fencing inside the adapter. Alert sample flushing preserves its advisory
lock and tuple-revision acknowledgement as one adapter operation, while threshold
and version evaluation belongs to the Alerts feature.

SQLx remains available to the Server composition root and database test fixtures.
Do not move SQL into hosting code to avoid adding a narrow feature interface.

SQL and row mapping live under `adapters/src/persistence/postgres/`. Repositories
return feature models, not API views. Catalog queries apply authorization in SQL;
loading an unrestricted catalog and filtering it in memory is not acceptable.
Infrastructure routers may compose persistence helpers from the same adapter crate,
but those helpers return typed data rather than SQLx rows. Do not add a feature port
solely to wrap a private query shared by infrastructure implementations.

The Resource Copies feature coordinates snapshot-backed workload duplication.
Its repository atomically records the reviewed plan and child restores, then
creates the undeployed destination through the existing transactional workload
creation helpers only after every restore succeeds. Backup workers own existing-
backup restore execution. For policy-independent current data, the copy plan stores
per-volume jobs, which the copy worker claims atomically before execution through
its storage port. The runtime adapter takes an encrypted temporary Restic snapshot
and restores an owned fresh volume using a bounded, network-isolated helper.
Current-data jobs require Platform Download authorization and an explicit source
consistency acknowledgement. Cancellation and permission changes cancel the owned
execution token. The worker owns separate reconciliation and execution futures
under the same shutdown token. Current-data execution is bounded to one helper globally, so
long copies do not block other parents' finalization, cancellation or cleanup.
The execution lane retains completed results until their idempotent database
write succeeds. Before claiming new work it fails abandoned claims, including
ambiguous claim commits, without replay; queued jobs remain claimable. Both paths
share finalization and volume ownership receipts.
Copy plans retain separate source and destination identities. The authorized
SQL catalog limits destinations to the source platform type. Existing-backup
copies may restore to another platform or Swarm node using S3-compatible storage;
current-data copies remain on the original daemon/node. Destination authorization,
routing identity and remapped Swarm references are fenced at review, start and
finalization. Snapshot selection always uses source identity; restore children,
volume ownership receipts and cleanup use destination identity. Swarm references
are resolved by name in destination inventory without copying secret/config values.
Optional destination fields preserve older same-platform durable plans.
The copy worker reconciles durable parent state. Runtime storage
checks and ownership-verified cleanup use the Platform runtime router, outside
database transactions. Destination creation commits before resource invalidation.

Copy progress uses an actor-owned `resource-copy:<id>` realtime group.
Persistence publishes operation IDs on `citadel_resource_copies` inside the
transaction that changes a parent or child restore. A supervised listener
invalidates only the affected group; reconnects request a resync. Group reads
reuse the copy service's initiating-actor authorization and persisted progress.

### HTTP and realtime presentation

`src/server/src/api/routes/` owns endpoint registration, extraction and HTTP responses.
`src/server/src/api/resources/` owns shared HTTP/WebSocket public views and mappings,
plus resource-owned HTTP request DTOs and patch handling. Both transports reuse
these representations and actor-specific enrichment.
Keep routes thin: authorize, call the feature, then map the result.

Use `From` for context-free projections and named mapping functions when actor
permissions or other context are required. Preserve field names, nullability and
secret redaction. Feature errors retain their meaning until Server maps them to
sanitized HTTP errors; unexpected failures are logged internally.

Realtime consumes feature interfaces and shared presentation mappings. It does
not import HTTP route modules, request states or HTTP error rendering, execute SQL,
or select Local, Direct and Edge transports itself. WebSocket upgrades remain in
realtime; HTTP mutation-notification middleware lives in
`api/mutation_notifications.rs` and calls the existing realtime publication boundary.
The WebSocket write buffer accommodates the bounded outbound queue's largest
message, including framing overhead. Group snapshots remain capped at 4 MiB;
large supported snapshots must not disconnect the shared connection and force other
resource subscriptions to reload. Container process lists load through HTTP and
refresh only for subscribed Inspect viewers on the existing scoped statistics/state
events. Each event shares one bounded runtime read per container after actor-specific
authorization. A process event read that exceeds the configured realtime read timeout
is cancelled and clears only that process panel; other subscriptions stay connected.
Authorization failures still fail closed. Process snapshots are transient; no database
history or separate polling worker is involved. Health history loads through its HTTP query;
the authenticated health group sends small invalidation notifications on join,
reconnect and committed changes, rather than carrying the retained records.
Script run history loads through HTTP, then an action-authorized realtime
group replaces the latest 50-run window on join, reconnect and committed run or
cleanup changes. Live snapshots update the frontend query cache and cancel older
HTTP reads. Resource subscriptions own synchronization; execution, cancellation
and cleanup controls do not invalidate those queries. Disconnected views retain
their last data until reconnection or an ordinary HTTP refresh.

## Composition and lifecycle

Server parses and validates configuration before constructing services.
`src/server/src/composition/` builds typed components once and passes narrow handles
to routes and workers. Constructors receive settings explicitly rather than
reading environment variables themselves. Composition constructs health/Git/Edge
adapters and one shared backup-coverage service before route assembly. HTTP coverage
remains available when realtime is disabled. The router assembles endpoint groups
and middleware from these constructed dependencies.

Core applies migrations and acquires its background-job lease before running
workers. Only one Core process may own that lease for a database. Shutdown cancels
listeners and workers, drains tracked work, then closes database resources.
Every spawned task needs an owner, cancellation policy and shutdown behavior.

Build, Backup and Restore workers retain known execution results in their current
slot while retrying completion storage failures, with cancellation-aware backoff
capped at five seconds. They never repeat execution or cleanup to retry persistence.
Each Build claim persists a fresh attempt UUID in the same transaction as its
Preparing transition. Completion and streamed log writes require that attempt,
an active run, and the Project's current-run ownership. A stale controller cannot
write into a newer attempt of the same run. This token fences database writes;
it does not by itself reserve executor capacity or confirm remote cleanup.
A rejected guarded transition is read back: recovered terminal state wins, while
superseded or removed claims end the attempt. Terminal progress follows confirmed
storage. Build failure observations are optional and enter the durable alert queue
in the successful completion transaction. Shutdown may leave an unresolved claim
for existing recovery; the in-memory result does not survive a Core crash.

Build selection admits one undispatched preparation/probe per configured target at a time.
After dispatch, canonical daemon capacity governs concurrent execution.
A deferred target's cooldown also applies to its queued sibling projects, and
untouched queue entries precede deferred retries. Other targets can use the
remaining preparation permits while a connection probe is pending. Backup and
restore claims scan at most 16 candidates per dispatch when source ownership
blocks an earlier candidate.

Agent build admission is shared across command contexts and replacement Edge
sessions. A slot covers the build, its declared pushes and confirmed local
cleanup. Proven pre-execution failures release slots after confirmed cleanup;
uncertain execution retains occupancy. Core defers only an explicit
pre-start capacity refusal (including its own Edge transfer admission), preserving the resolved commit and original queue
age with bounded backoff. Before remote dispatch, Core persists the resolved
commit and an attempt-owned
reservation keyed by Docker daemon identity. Platform/Pool aliases share capacity;
a second Agent authority cannot take over a daemon with outstanding reservations.
Completion releases capacity transactionally for success, verified pre-start
refusal, or a persisted same-authority Agent cleanup receipt. Cleanup checks fence
delayed admission before confirming absence or abandoning a pending push; they
never clear active or uncertain daemon work. Receipt persistence and a system run
log commit together, and receipt persistence alone does not release occupancy.
Startup recovery consumes that evidence; otherwise failures/restarts quarantine
reservations. Project serialization, pool policy and retention include unresolved
reservations. The shared claim query blocks replacement work across all builder
kinds, including Local, before source preparation. A supervised cleanup worker
sweeps terminal reservations in pages of 16, with five-second Agent checks and
30 seconds between pages. It uses the original run target and persisted authority,
never the Project's newly selected builder. Cleanup evidence, reservation release
and a system log commit atomically; the run outcome remains unchanged. Paging
advances past unavailable Agents, and existing receipts can settle offline attempts.
Active work, uncertain execution and changed authorities stay held. Human administrators
can resolve terminal holds after verifying/stopping old work and restarting the original
Agent. A bounded host check must reach the same daemon with a different admission
authority. The release transaction rechecks current administrator access, fences the
exact terminal attempt, and commits an activity and run log with the operator's notes.
Operator acknowledgment is never stored as an Agent cleanup receipt. Run outcome and
completion time remain unchanged; no execution is replayed. Unreachable or replaced
daemons cannot be manually released through this path. Core-local/data-operation
admission remains outstanding.

Interactive streams normally end with their caller. Durable operations use
persisted claims and may continue after a browser disconnect. Recovery inspects
and settles interrupted work; it must not blindly replay ambiguous mutations.
PostgreSQL notifications are wakeups, while claims and queue rows remain authoritative.

Platform and Build Pool connection transitions publish typed events on
`citadel_connection_events` within the transaction that persists the transition
and its activity. PostgreSQL delivers them after commit. The shared
`DatabaseNotificationHub` forwards these to `ConnectionEventHub`; consumers use
`notifications.connections.subscribe()` independently. The realtime subscriber
publishes resource invalidations, and authorized readers load the current state.
Repeated health observations and obsolete Edge sessions do not emit transitions.
Listener reconnects and subscriber lag trigger a resync of both resource types.
This bounded stream is for notifications; durable activities and alert work remain
in PostgreSQL. Adding a subscriber does not require wiring callbacks into stores.

Workload statistics snapshot ownership when samples are admitted. Deployment and
standalone Stack samples are stored transactionally in `workloadstats`, independently
of the container projection; Swarm keeps its existing `swarmservicestats` history.
Queries use the Deployment ID or Stack ID plus service and replica identity, never
an unscoped container name. The same seven-day bounded retention applies to both.
Raw container statistics remain tied to their individual container projection.
Frontend live statistics accumulate sums and counts at the selected history
resolution before rendering. Replica averages are summed after temporal
aggregation. Historical buckets remain authoritative on overlap because history
responses do not include the counts needed to combine partial averages. Current
usage summaries read the latest observation separately from chart averages.

## Runtime routing and observations

`PlatformRuntimeRouter` resolves persisted Platform identity or an exact Swarm
node and exposes the capability needed by the caller. Registered Direct consumers
share a connection registry; Edge owns a separate session registry. Registration
validates proposed addresses before they become persisted routing targets.

Keep information, health, statistics, inventory and mutation interfaces narrow.
A container collector should not enumerate images or volumes. Native Swarm
operations validate manager identity, ownership and the whole selection before
mutation. Confirmed successful removals remain committed even if a later item fails.

Preserve these rules in event and recovery work:

- Known container lifecycle events update state without fetching full metadata.
  Metadata changes and deletion use their own observation paths.
- Batches are bounded and scoped to the Platform/node. Generation, timestamp,
  row-version and Edge-session checks reject superseded observations.
- Daemon events must not overwrite operation claims. Operation completion owns
  final parent status, activity and claim release.
- Publish realtime changes after persistence commits, scoped to affected resources.
- Reconnect and periodic reconciliation repair missing observations. Health probes
  do not trigger full inventory collection while a Platform is stable.

Statistics use bounded shared writers and batch database writes separately from
sampling. Realtime can deliver a sample before its history is flushed. Telemetry
retries have finite retention and explicit discard metrics; durable operation
claims and alert observations must not use that lossy policy.

## Authorization and secrets

Identity owns permission evaluation and its process-local cache. Permission
mutations acquire the cache mutation fence **before** a database connection and
publish invalidation through the commit path. Readers retain the matching fence
through query and cache publication. Rollbacks do not publish a successful change.

Known-resource permission checks can be batched. Realtime connections bind their
permission decisions to an actor generation; invalidation closes affected
connections and streams. Raw observations may be shared across subscribers, but
permission decisions and presentation remain actor-specific. Manual ACL changes
outside these mutation paths require cache invalidation, such as restarting Core.

Credential files and Git/Script workspaces belong to infrastructure adapters.
Use private staging, bounded subprocesses and cleanup on success, failure and
cancellation. Git SSH requires verified host keys; do not disable host verification.
API responses, logs and activity snapshots must not disclose stored secrets.

## Frontend and generated contracts

The frontend uses the full Rust OpenAPI schema. Server owns `utoipa` definitions;
feature crates do not depend on them. Generate both API documents and the TypeScript
client together; see [API changes](DEVELOPMENT.md#api-changes). Do not hand-edit
schemas or generated client types.

Core advertises `WebSocketV1`; the frontend connects only when that capability is
enabled. HTTP queries and live snapshots retain distinct ownership, with explicit
reconnect reconciliation. Swarm HTTP and live overview payloads share the server
projection of persisted quorum, platform control and manager/node-local freshness.
Event-scoped aggregate reads are shared across recipients; capabilities and group
authorization remain actor-specific.

Task sheets rely on subscribed resource snapshots for Stack, backup policy, and
run status changes. Build logs load history initially, merge streamed entries,
and recover missed history on reconnect. Tag assignments invalidate the affected
resource lists/details and tag usage counts rather than the entire query cache.

Register feature components in `src/frontend/src/features/index.tsx` through the
shared list/form/detail factories. Resource page abstractions own routing, tabs,
layout and declarative access/task-sheet options. Forms reuse FormShell/FormBuilder.
Use a dedicated route only when the shared page model does not fit the workflow.

Shared resource pages and UI components own layout, status badges, loading states
and actions. Appearance uses CSS variables and shared density/radius tokens.
Semantic health colors keep their meaning regardless of the selected accent.
The shared top loading bar aggregates active React Query reads and mutations.
It shows even fast requests for a minimum duration and waits for a quiet period before completing;
individual responses and retry attempts do not restart the animation.

Realtime snapshots or scoped invalidation events own synchronization after changes
and on reconnect. Avoid parallel polling or mutation-triggered refresh-all flows.
Authoritative mutation responses may update query caches while disconnected; cancel
stale reads before applying them. Explicit error retries remain available.

Frontend API modules and UI primitives must not import resource screens or the
application router. Features may consume shared resource page templates and their
types; `pages/` is not exclusively a routing layer. The frontend architecture test
checks static imports, re-exports and literal dynamic imports using the existing
TypeScript parser, including relative paths and the `@/` alias.

The Agent protobuf inventory and Docker API generation inputs have their own
contract checks. Changes to either require verification of the generated output
and affected runtime mappings, not just successful compilation.

## Making a change

1. Put business behavior in its feature, external I/O in its adapter and wire
   presentation in Server or Agent.
2. Preserve authorization, transaction, cancellation and recovery boundaries.
3. Reuse an existing type or interface only when its semantics match.
4. Run focused tests, architecture checks and relevant schema verification.
   Use disposable external-service fixtures for integration tests.

Test runners and fixture requirements are listed in [test/README.md](../test/README.md).

### Execution capacity and renewable backup ownership

Build dispatch uses bounded controllers with a separate preparation semaphore.
A host reservation precedes Git preparation; `dispatchstarted` is committed with
the pinned input before transport submission. Preparation failures can release
an undispatched reservation; dispatched ambiguity requires authenticated cleanup.
Preparation ownership follows the actual Direct request or Edge outbound frame,
not the remote build stream. Completion retries retain the same result.

Backup/restore claims have attempt UUIDs. Repository and source leases renew
while execution is polled, and late owners cannot renew or finish a replacement
attempt. A resolved source plan is persisted before acquiring its complete,
sorted source set. Contention rolls back partial acquisitions and defers work.
Recovery interrupts lost owners but never frees storage based only on expiry.
Maintenance and credential validation use the same renewable repository locks.
Each backup/restore queue has one dispatcher and bounded, owned execution tasks.
The shared tracking permit travels with the durable claim through completion
persistence. Initial ownership is verified before execution starts; subsequent
lease renewals stay concurrent with execution while their database requests are
pending. Backup and restore results share the same bounded
completion retry policy. Restic dependencies are reusable; ownership and mutable
admission/cleanup state belong to an explicit operation context for each run.
Each claim call scans at most 16 source-blocked candidates, so independent backup
or restore work can start without another notification. Accepted repository and
source lease TTLs are at least 90 seconds, with a 30-second heartbeat and bounded
10-second renewal request.

Data reservations and build reservations share one authenticated daemon authority.
Agent data admission covers an item/restore/maintenance operation across helper
creation, execution and cleanup; the Agent heavy-work budget is shared with
builds. Core's backup tracking limit bounds controllers, independently of host
capacity. Local build/backup/staging work shares one process-wide heavy-work
budget. Local Docker execution also records daemon/authority reservations; the
Core process has one authority shared by its build and backup executors. A new
Core authority cannot overwrite occupancy retained by an old process or Agent.
Scripts select Core (default), a specific Agent, or automatic placement
among eligible Agents discovered using the run-as identity’s Execute permissions.
Automatic placement uses the existing Automated Operations entitlement, checked
when selecting it, starting a manual/test run, and before worker dispatch.
Downgrade retains configuration and permits switching to fixed placement; cancellation
and cleanup remain available. Discovery is bounded and rotates candidates between attempts; authorization is
rechecked before dispatch. Core Deno slots and remote tracking slots
are independent. Remote Deno processes share Agent heavy capacity with builds and
backups; Core persists ownership before sending the script over signed Direct or
authenticated Edge transport. Both require HTTPS for credential-bearing work.
Run-as tokens are bound to the actor, run and attempt, and stop authenticating after
cancellation, deferral or completion. Cross-host resource requests go through Core's
existing authorized HTTP API, never directly from one Agent to another.
Only a definitive pre-start refusal is deferred; ambiguous dispatch is not replayed.
Cleanup receipts fence delayed dispatch. Retained capacity after an Agent restart
requires an audited human-administrator acknowledgment and verification of a new
Agent authority on the original daemon. This is single-action execution, without
workflow orchestration. Manual, test, scheduled
and webhook runs all enter the same durable FIFO queue; only a worker claims a
run after acquiring the applicable local or remote tracking slot. Source, arguments and run-as identity are frozen
at enqueue. HTTP progress observers use a bounded broadcast and cannot cancel or
backpressure execution. Explicit cancellation is persisted, including during the
queued-to-running transition, and workers observe it before starting a process.
Queued runs survive restart; interrupted executions fail without being replayed.

Before the first external operation, a busy/offline backup host or busy restore
source returns the run to the queue with a 15-second eligibility delay. Retried
candidates sort by their next eligibility time so a full page of blocked work
cannot monopolize later dispatches. This releases tracking and business leases, preserves the resolved source
plan, and never defers a partially executed backup. After work starts, the parent
retains storage ownership while its sequential items wait for their next host.

An Agent records helper names before submitting Docker create. Cancellation of
that request is not a definitive create result: a later 404 alone cannot release
its reservation. Confirmed helper identity/removal or a definitive create response
is required. Same-authority reconciliation fences delayed commands. Restarted
Agents and Local helpers without sufficient identity or cleanup evidence require
manual operator review.

Backup storage recovery enumerates repository owners independently of host
reservations. Admission and recovery serialize on the run and repository lease;
recovery fences further dispatch. An explicit cleanup receipt survives removal
of the final host reservation, until parent completion or recovery releases
repository/source ownership. Missing reservation rows alone never authorize
release. Core staging and direct Restic work mark the owner unsafe before they
start and clear that marker only after owned work and staging cleanup finish.
An interrupted Core process with unconfirmed direct work remains quarantined.
Staging directories are not deleted solely because of their age.

Local Docker data helpers have an operation-owned journal. Creation intent is
persisted before Docker create; the immutable container ID is recorded before
start. Recovery verifies the original daemon and operation label and removes
only that helper ID. Delayed starts then fail against a removed ID, while fenced
attempts cannot create another helper. An unacknowledged create with no observed
container, unavailable Docker, or a changed daemon keeps ownership reserved.

Failed builds release capacity only when a matching BuildKit history record
confirms terminal execution and temporary files are cleaned. A successful CLI
cleanup or nonzero exit alone is insufficient. Pushes consume Docker's structured
response stream; complete terminal responses differ from truncated streams,
timeouts and transport loss. Unsupported build-history inspection retains the
reservation rather than guessing that execution stopped.
