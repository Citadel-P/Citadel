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

## Instance configuration

The `citadel-instance` feature owns typed, non-secret installation settings,
patch validation, deployment policy and change notifications;
Adapters merge partial choices against the locked singleton, recheck the current
enabled human administrator, validate effective coupled fields against explicit
deployment overrides, and record old/new settings in the same transaction.
Server supplies a trusted boolean indicating whether the deployment Core Agent
connector requires HTTP. The same locked mutation rejects an effective HTTPS-only
policy in that case, before any settings or success audit are written. No Agent
inventory is scanned and no deployment address is exposed by the settings API.
Instance configuration and MFA changes write one `InstanceConfigurationUpdated`
activity against the installation UUID, with the
administrator actor and non-secret before/after values. Settings and activities
commit atomically; no-op saves emit no activity. Instance activities are
administrator-only. Older `InstallationSettingsUpdated` User records remain readable.
Core restart uses a persisted singleton claim and an `InstanceRestarted` activity.
The claim is serialized with update admission by the existing identity mutation
lock. Hosting polls the claim with bounded cadence, drains existing owned work,
closes the database and releases the job lease before Linux re-exec. New startup
acknowledges the activity only after both listeners bind. A boot UUID fences
replayed requests; browser disconnects cannot cancel persisted admission. Pending
restart activities are retained until acknowledgement. No Docker permissions or
container restart-policy assumptions are required.
Restart admission does not broadcast a settings-change invalidation while Core
is shutting down; the existing realtime reconnect reconciles settings, restart
status and activity history after startup.
Notifications follow commit. The administrator Instance page reuses resource
registration, FormShell, installation-settings mutations and existing realtime
invalidation/reconnect handling. Configuration and Administration are bottom
sidebar popup menus with permission-filtered destinations; personal security remains in Profile.
The crate follows the existing `model`, `commands`, `validation`, `repository`,
`error` and `service` roles. Instance uses Identity principals, MFA vocabulary and
its clock port; Identity has no Instance dependency. Authentication retains only
first-run setup choices/restrictions and a narrow persisted MFA-policy read.
Server composition joins Instance changes with first-run setup notifications for
the update checker, preserving immediate scheduling without an ownership cycle.

MFA retains its live minimum-policy behavior. The other 25 controls require Core
restart. Server resolves saved configuration after migration/recovery admission
and before constructing Identity, execution and monitoring consumers, logger and
OpenAPI routes. Explicit environment presence takes precedence over stored choices,
then built-in defaults. Saved, effective and running values are separate public
projections; no secret or complete startup Config is exposed. Logging for other CLI
commands uses deployment settings without requiring a settings database. Execution
permit pools and claimed work keep their existing ownership/recovery behavior;
there is no hot-reload or automatic restart controller. Script creation reads only
the active timeout defaults from authenticated application information.

The executable `app` module coordinates admission, serving and shutdown. `startup`
owns the migration/settings-resolution sequence; `config/instance` maps deployment
overrides and resolved choices into Server runtime options. Only promoted override names live in
`citadel-instance::environment`; database, bootstrap, path and listener identifiers
remain with their hosting/recovery consumers. Server parses explicit promoted
overrides once, preserving absence, before admission; after migrations it merges
saved choices and validates the effective values before constructing consumers.
The initial admission configuration uses typed built-in defaults for these consumers,
which are not started until resolution succeeds. There is no coupled-value clamping.
Instance persists log format as a `Text`/`Json` enum, mapped explicitly at HTTP and
logging boundaries; deployment verbosity remains a string to preserve module filters.
Early failures return bounded phase context even before a subscriber exists.
`logging` installs the subscriber from the selected verbosity and format, without depending on the Instance
feature or persistence. Both CLI commands and maintenance mode use its environment
entry point.

## Instance update work in progress

The Updates feature owns the release-check service, policy validation, release
selection, compatibility descriptors, reviewed plans and operation/journal invariants.
Its release-source interface is implemented by the bounded GitHub HTTP adapter;
Server owns scheduling, composition and HTTP presentation. Adapters own release HTTP access, private
filesystem journals and transactional PostgreSQL state. Instance owns the
installation-wide check setting; its write transaction emits the
Updates activity and an Instance configuration activity when MFA or other Instance settings also change.

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

Release discovery and artifact reads each own a long-lived HTTP client with their
respective redirect policy, request timeout and response-size bounds. Each pool
retains at most one idle connection per host for 90 seconds. Agent artifact checks
still fetch and validate current publication metadata and manifest checksums on
every request. Successful Agent signature verification retains only one
version/full-image-digest pair for 15 minutes in process memory; expiry, a changed
key or restart requires verification again. Concurrent verifiers share that result
through a cancellable lock. Failed or cancelled verification is never cached, and
runner cancellation cleanup is awaited before releasing the lock. Core artifact
verification remains uncached. This reduces repeated check work; it does not evict
filesystem cache populated by the first verifier execution.

Workload image metadata checks in Core and Agents lazily initialize a separate
registry HTTPS client on the long-lived Docker client. Docker client clones share
its connection pool (one idle connection per host, expiring after 90 seconds).
Every inspection still reads current metadata and owns its credentials and bearer
token; authentication is never cached on the shared transport. Redirects remain
disabled, with the existing timeout, response-size and digest checks preserved.

Core and Agent select their executable command before constructing Tokio. The
container healthcheck uses a current-thread runtime and probes the actual HTTP/TLS
listener; serving and other commands retain the multithreaded runtime. Automatic
loopback probes bypass proxies, reject redirects and require HTTP 200. Agent also
validates its healthy response body. Core's explicit `healthcheck --url` retains
its custom URL redirect/status behavior. TLS chain validation and the narrowly
scoped loopback hostname handling are unchanged, as is the entrypoint privilege drop.

Update availability uses the existing Alert Rules, observation outbox and delivery outbox.
Core observations follow successful official release discovery for the running Core's
release channel, including installations that require manual updates. Notices do not
claim installation eligibility; artifact/transition verification and approval remain
part of installation review. Update events and their counts are administrator-only;
ordinary Alert grants do not expose them. Agent observations follow a successful compatible release
check and inventory refresh, and refresh again after a batch settles. Installation
aliases are deduplicated; only names, versions, counts and opaque correlation hashes
enter alert details. Offline/unknown previously advertised targets retain pending
confirmation, while removed registrations and verified current versions can resolve.
An empty Agent inventory skips release verification and suppresses irrelevant check
errors. It still publishes an empty observation against the cached verified release
so removing the last Agent can resolve its availability alert.
The latest observation is stored alongside each existing update-check state and
revalidated under a row lock during alert evaluation, fencing delayed queue entries.
Same-release summary changes preserve the event and enqueue no delivery; new releases
reuse existing supersession. System-owned Info rules work in Community, with external
channels assigned through the usual Alert Rules configuration.

Agent updates share the Updates page, release verification, Activities and realtime
notifier. The shared **Review and update** dialog selects Core, PostgreSQL or Agents;
manual operations stay separate, and Agent progress remains in that dialog using
existing realtime invalidation. It does not coordinate a browser-owned combined rollout.
They have explicit installation targets and one durable sequential selection;
Core and Agent claims share the identity mutation lock and cannot overlap. Reviews
expire after five minutes, approval request IDs replay exactly, and progress uses
revision-checked repository writes with Activity events in the same transaction.
The supervised Agent worker uses the existing single-database background lease.
Interrupted targets are observed, never redispatched. Each new target rechecks the
requester's administrator access or the scheduled operation's Team entitlement.
Shutdown during dispatch or verification preserves the target's Updating state.
Standalone settlement uses the reviewed source container/image and operation ID.
Before helper creation is attempted, a completed preparation failure (including drain
or image-pull failure) leaves one operation-fenced receipt in the live Agent gate.
The unchanged running source can acknowledge that failure repeatedly, without
requiring a helper, and the same operation cannot be dispatched again. A new operation
replaces the receipt; Agent process loss removes it and requires manual recovery
unless helper evidence establishes replacement. Helper absence alone is never proof
that a request was cancelled.
After helper creation is attempted, settlement locates the owned helper. The authenticated
replacement must match the operation label and the helper's exact command/image;
the helper must have exited successfully before ownership is released. Exited
helpers provide idempotent evidence after lost replies and Core restarts
until Core persists the verified Succeeded target. Core then explicitly requests
removal of the exact stopped source and helper, without force or volume removal,
and persists `cleanupCompleted` in the existing operation JSON. The operation
retains ownership until cleanup is acknowledged; retries after lost replies or
restarts use durable success instead of re-verifying a deleted helper. Completed
targets with acknowledged cleanup are not contacted again. Failed attempts retain
their containers for inspection.
An unresolved or failed helper keeps the operation owned for manual recovery;
timeouts never establish that a Docker mutation was cancelled.
Swarm cleanup uses the reviewed node IDs stored in the operation. Missing sessions
or resume acknowledgements keep the operation owned; the worker retries after
reconnect and cannot start the next target or release the claim before cleanup.

The supported standalone Docker executor drains affected Agent work and launches a
separate container from the verified target image. It preserves configuration,
identity mounts and host ports, carries the source's standard or rootless Unix
socket into the helper, and pins custom network attachments by ID while retaining
aliases and gateway priorities. Dynamic addresses are reassigned; fixed addresses
and legacy links require manual handover. It retains previous containers during replacement and on failure, and
requires authenticated reconnect with the expected image/version and changed
container identity. Managed Swarm services reuse NodeAgentSetupService with a
bounded node-local drain; Repair uses the stored approved digest. Mixed or stale
node observations cannot establish successful rollout. Unsupported profiles remain
manual. Additive update RPCs do not relax the Swarm mutation allowlist: node Agents
allow metadata and drain/resume, but reject standalone self-replacement.
Drain blocks new work while allowing the existing identity-fenced data, build and
script reconciliation handlers to release retained admission capacity.

The shared Team update schedule reuses CronSchedule and the Script form controls.
Core is opt-in (`includeCore`, default false when reading stored schedules). The
existing supervised Agent worker claims each due occurrence under the identity
mutation lock, freezing explicit membership before network preparation. A small
`updatescheduleruns` record holds the pinned Core plan, Agent release, installation
identities, phase and execution IDs. Its active uniqueness constraint and the
manual Core/Agent claim guards preserve one owner through the phase transition.
No transaction spans Agent RPCs. Schedule edits affect future unclaimed occurrences;
an owned occurrence may finish preparation across minute boundaries.

Scheduled Core authorization is a dedicated installer command with a fresh Team
check, not a fabricated administrator approval. It reuses qualification, source
identity checks and the normal helper/drain/checkpoint/replacement execution.
Core claim and occurrence linkage commit atomically. After restart, continuation
waits for successful Core verification and released maintenance before atomically
claiming the Agent phase. Agent execution uses the pinned release matching the new
Core and rechecks the frozen target identity before each dispatch. Lost claim
responses are reconciled, never blindly replayed. Uncertain execution keeps the
shared owner. Agent failure does not undo Core success.

Every occurrence resolves a verified release even if periodic discovery is off.
Policy and Team entitlement fence new starts but allow existing execution cleanup.
Busy/missed occurrences are skipped without catch-up. Manual selection never edits
the saved schedule. Next-run presentation uses the same cron/time-zone parser as
execution, including DST folds and gaps, and always describes saved settings.

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
shares the exact PostgreSQL container's network namespace for Core updates (the installation bridge for an explicit database update). Its profile prohibits
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

When `CITADEL_UPDATE_STATE_DIRECTORY` is configured, the Linux adapter
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
Rootless installations record `/run/user/<uid>/docker.sock` as the host socket;
all executor containers map it to `/var/run/docker.sock`. Rootless Core mounts only
the daemon's data root under its matching `/host` path. Observation binds this path
to live `DockerRootDir` and verifies daemon mode without changing existing rootful
registration fingerprints. Private input and journal ownership is checked inside
the executor namespace; rootless container UID 0 maps to the daemon's host user.
The Core volume `!override` Compose tag is permitted, with all other tags rejected
and the final resolved mount profile still checked.

Database update status distinguishes deployment topology from update eligibility.
A bounded read-only runtime matches the actual PostgreSQL connection endpoint to
Core's Compose database container and shared network, independently of service names.
Ambiguous endpoint matches stay unknown; detection does not relax the supported
registration profile. SQL stays in the PostgreSQL adapter.
Missing Docker evidence stays unknown; an unregistered bundled database
reports in-app updates not configured, without implying an available database upgrade,
while only an external database is labelled in-app unavailable.
Registered status reads use the startup-verified installation, without launching
helpers or acquiring update ownership.

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
Registration is local operator authority, not an administrator-approved update or
user Activity. The supplied Compose file sets the fixed internal
`/var/lib/citadel-updates` path and a read-only `.citadel-updates` bind. Startup
resolves its actual host path from Docker metadata, without relying on shell PWD.
Only a database without completed administrator setup may seed pending registration
in an empty dedicated directory. The helper makes that directory private; pending
and verified registration use the same atomic file, so lost verified state cannot
be mistaken for an interrupted first setup. Invalid state still blocks admission.

A supervised, cancellable task waits for persisted administrator setup and uses
the existing bounded host helper and bootstrap/preflight checks to complete
registration. It retries briefly for Docker health, then stays idle until shutdown;
restarting Core retries pending setup. Readiness is published in memory only after
registration is durable, followed by the existing Updates invalidation. Status
reads never launch registration helpers. No container recreation, `.env` mutation,
database migration or update approval occurs during registration. Custom deployments
without the automatic mount retain the explicit `enable-updates.py` utility;
it uses the same Rust diagnostics and preserves configuration and recovery evidence.
Startup checks verified registration identity and private evidence.
The local `check-update-installation` diagnostic now revalidates
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

### Deployment-managed update registration

Demo/preview deployment removes the standard automatic state setting and bind
from its generated base Compose input. CI retains ownership of the existing
explicit host-state registration and guarded refresh; it never starts a second
first-install registrar or moves existing recovery state.

Before pulling images or replacing Core, deployment validates the selected state
directory's ownership and private permissions and probes actual write access.
Unregistered rootless targets use a dedicated directory under the deployment
account's home when no sibling state directory exists; configured and interrupted
state paths are preserved. Registration verifies each volume's Compose project
and volume-role labels alongside its exact exclusive container mounts. Citadel
system labels are optional on existing volumes but must agree when present;
registration never relabels or recreates persistent volumes.

Demo/preview release deployment transports the activation script from the same
release checkout. Core and Agent pins are written into the captured private
`.env`; `.release.env` remains a release record and is not an additional Compose
input. The running PostgreSQL image is preserved across Core deployments.
CPU/memory ceilings and a single default-IPAM subnet/gateway are supported and
compared against live Docker settings during registration and replacement.

A registered CI deployment verifies the current registration and recovery
preflight before replacement. Under the existing installation lock it rechecks
unchanged evidence and database update ownership, then stops only Core. It releases
the lock before target startup, whose normal database-claim and recovery checks
remain authoritative. After the intentional replacement, the existing refresh
command verifies unchanged database/storage identity and binds the new inputs.
An interrupted replacement or failed refresh preserves registration and requires
operator recovery; CI does not blindly refresh drift on retry. Activation never
approves a Core or PostgreSQL update.


### PostgreSQL engine updates

The administrator-only Updates status reports the live `databaseVersion` through
its existing repository. A registered Compose installation can also offer the
PostgreSQL image qualified by this Core build (currently Alpine 18.6). External
or unregistered databases remain manually managed. Unknown versions never imply
that an engine is current.

Status uses the startup-validated registration and a bounded engine-version read;
it does not launch preflight helpers or acquire the installation lock. Availability
is advisory until review and approval recheck the live installation.

An explicit database review uses the existing instance update service, review,
claim, journal, checkpoint, Activity, and target-acceptance lifecycle. Its optional
`UpdatePlan.database` pins the engine versions and official image digest while
requiring identical source/target Core identities. Scheduled Core updates cannot
implicitly upgrade the database. Review and approval check replication, tablespaces,
cluster layout/ownership, and the permitted Compose selectors before saving a review
or claiming ownership. These read-only checks return a specific blocker and run
again in the executor; status reads remain advisory.

The detached executor uses the installation bridge for database operations so it
survives replacing PostgreSQL. Core-only operations retain the exact PostgreSQL
network namespace and unchanged-database invariant. Database replacement permits
only `PG_IMAGE` and `PG_DATA` selector changes. It retains daemon, volume and
network identity, records mutation intent before stopping the engine, verifies the
replacement container/version, and binds its incarnation before target acceptance.
Target startup distinguishes a database operation by its actual engine version;
it still verifies the exact Core container, catalog, keys and committed receipt.
No timeout replays replacement or silently restarts the source. Temporary Docker and inspection files
use a bounded `/tmp` tmpfs for both Core and database operations, including helper
commands. Abrupt helper termination cannot leave disposable files in the durable
update-state directory; startup still rejects unexpected entries there. Database
replacement rechecks the approved input fingerprint after shutdown and immediately
before submitting Compose, so edits during shutdown cannot be applied silently.

Citadel's minimum PostgreSQL version is 18. The current executor performs only
qualified minor updates within 18, retaining the existing data directory. It rejects
older engines, replication and external tablespaces; major-version transitions are
not enabled without separate qualification. Core needs only the PostgreSQL client
utilities already used for checkpoints, with no bundled database server toolchains.
Engine updates are restricted to Alpine clusters owned by UID 70 in the versioned
`/var/lib/postgresql/18/docker` layout. External databases and custom layouts are
never replaced. Failures retain evidence and maintenance ownership for explicit recovery.

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
and durable queues retain their fallback scans. Job-alert observations and Swarm
pruning share the existing notification hub's coalescing wakeups instead of holding
separate PostgreSQL connections. Startup and fallback scans still recover their
durable work; hub reconnects wake both consumers, and a closed hub cannot spin them.
Copy progress, backup coverage and Stack drift also share that LISTEN connection.
Each topic carries resource IDs through a bounded 256-event fanout; slow subscribers receive a
resync indication. Copy and coverage readers refresh persisted state after a gap;
Stack drift retains its bounded event batches and five-minute reconciliation.
Coverage collects up to 256 distinct platform IDs over 50 ms before loading each
platform's affected resources once per batch. Unmanaged-container creation keeps
its dedicated listener: its grace queue cannot reconstruct missed creation events
from a durable queue, so it must not use the bounded, lossy fanout.
Edge observation persistence keeps session fencing inside the adapter. Alert sample flushing preserves its advisory
lock and tuple-revision acknowledgement as one adapter operation, while threshold
and version evaluation belongs to the Alerts feature.
Alert evaluation retains the rule/resource advisory lock and durable observation
receipts. A non-matching observation creates or resets its counter only when
needed; repeated healthy observations preserve an already-zero state tuple and
its cooldown timestamp. Incident resolution still runs when the counter is zero,
because triggering an alert also resets that counter.

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
The copy worker runs both lanes at startup and wakes idle lanes from committed
copy/child-restore changes, with a 30-second fallback for missed notifications.
While parents are active, or after errors, both lanes retain a two-second delay;
notifications from their own writes cannot bypass that delay. In-flight current-
data copies retain their two-second cancellation/authorization checks. Only the
reconciliation lane prunes expired unstarted reviews; pending-work reads do not
perform cleanup. Reconnects wake both lanes, and shutdown still joins both.
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
resource subscriptions to reload. Published events and their lazy shared read caches
also share a 32 MiB retained-allocation accounting budget, in addition to the event
count limit. JSON payloads account for their allocated trees; typed projections use
a conservative serialization-size estimate without allocating an encoded buffer.
Reservations live until the last event/cache reference is released. This is not a
total process-memory limit: transient query results, subscriber views and allocator
RSS are outside the accounting. At capacity, replaceable statistics samples are
dropped; committed changes send a small resync marker and clients reconnect through
the existing snapshot lifecycle. Cache admission failure likewise fails delivery
closed and reconnects. A cache remembers its own admission rejection for that event,
so waiting subscribers do not repeat the rejected read. Rejected projections are
dropped; fresh events can retry. Loader failures and cancellation remain retryable
and do not become shared rejection markers. Durable persistence does not depend on
this budget.

The authorization safety interval reauthenticates and revalidates group access
without constructing list snapshots or presentation enrichments. Container detail
rechecks read only current container identity/ownership; build, backup and restore
run rechecks resolve only the parent permission resource. Other detail groups still
resolve current ownership/existence, and log/terminal groups retain runtime
target validation. Actor-generation fences continue to invalidate access immediately.
Container/Stack detail statistics use shared narrow container identities and send
only matching samples; Stack updates use `ReceiveStackContainerStats`, merging into
known containers without replacing lifecycle metadata. Connection-local platform
scope hints skip unrelated samples, survive permission-cache clears, and refresh on
metadata reads and safety checks. They filter work only and never grant access.

Platform-scoped Deployment/Stack invalidations pass that scope to the authorized
repository query and select typed health owners before coverage/copy enrichment
and JSON conversion. Truncated member lists refresh conservatively. Scoped rows
remain update patches; initial, reconnect and workload CRUD reads retain full
snapshots. Scalar Platform reads and live telemetry context borrow their event's
cached projection, whose reservation remains owned by that event. Actor-specific
output construction and access checks remain per subscriber.

Container process lists load through HTTP and
refresh only for subscribed Inspect viewers on the existing scoped statistics/state
events. Each event shares one bounded runtime read per container after actor-specific
authorization. A process event read that exceeds the configured realtime read timeout
is cancelled and clears only that process panel; other subscriptions stay connected.
Authorization failures still fail closed. Process snapshots are transient; no database
history or separate polling worker is involved. Health history loads through its HTTP query;
the authenticated health group sends small invalidation notifications on join,
reconnect and committed changes, rather than carrying the retained records.
Health reads use the same admission and transaction-local timeout as statistics
history. Network enrichment reads authorized saved workload configuration once
per request and parses each admitted Compose document once, then matches it across
the existing 100-network batches. Endpoint SQL arguments borrow only the first
500 identities per network. Existing row, Compose-byte and per-network result
limits still report truncation; no cross-request permission cache is introduced.
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
remains available when realtime is disabled. Coverage reads carry the requested
workload/container identities or volume targets into persistence. They also read
authorized policy-owned workloads, preserving shared-volume matches and uncertainty
from incomplete policy inventory. Latest run items and successful-history aggregates
are restricted to requested targets; history remains authorized even when a policy's
source has subsequently changed. Run items and workload assembly use indexed lookups.
The router assembles endpoint groups
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

Build process output is redacted before entering a batch of at most 128 rows and
64 KiB, with a 250 ms flush deadline and a final flush on EOF. The existing output
consumer owns the buffer; persistence failure cancels its producer. PostgreSQL
checks the active attempt once per batch, commits all rows together, then permits
log notifications. UTF-8 chunking and the 4 MiB per-attempt log budget remain.
Backup and Restore completion insert logs in chunks of at most 128 rows, targeting
64 KiB per statement (an existing oversized individual log remains intact), inside
the same completion transaction. Retries retain the existing completion fence.
Build, Backup and Restore log inserts use a stable typed-array statement per table,
so different batch lengths do not create separate prepared-statement variants.

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

Ordinary Direct Platform scripts, builds and updates reuse the existing Platform
runtime registry. Registered Direct Build Pools have one composition-owned registry
shared by health checks, scripts, builds and updates. It rechecks registration on
selection, normalizes endpoints and shares a lazy channel per resource ID under an
immutable transport/trust policy. Channel creation holds only a short map lock;
connects and RPCs run outside it. Signer clones observe key rotation. The existing
pool-health worker retires deleted or reconfigured registrations; operations already
holding a client keep ownership. These registries cache neither authorization nor
health. Recovery and cleanup of retained attempts still connect to the persisted
original endpoint and verify the original execution authority; they never select a
replacement solely because it has the same resource ID.

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

Statistics snapshot ownership when samples are admitted. `statisticssources`
identifies a physical container/task; `statisticsowners` records the workload,
service and replica/slot identity. `statisticssamples` stores each capture once.
`containerstats`, `workloadstats` and `swarmservicestats` are read projections over
shared captures, not separate metric copies. Container reads still require a live
container projection; workload/task history outlives that projection. Deployment
and standalone Stack owner records retain their deletion foreign keys; discovered
Swarm Stack identity remains a historical snapshot even after its Stack is gone.
Queries use the Deployment ID or Stack ID plus service and replica identity, never
an unscoped container name. During a same-cluster manager handover, host and node
inventory can contain different projection UUIDs for one Task. Source reassociation
requires the persisted Task, both live projections' Docker container IDs, and
Platform identity to agree. An explicit node ID must match the Task; a former host's
nullable node is resolved through its already associated Task rather than the
new manager's descriptor. A missing old projection can be
rediscovered only against that Task's physical identity. Unrelated live projections
cannot take over history. The same seven-day bounded retention applies.

Capture batches resolve repeated projection identities once, in last-occurrence
order. Ordinary containers reuse an exact existing source through a set-based
lookup; unchanged Workload owners also reuse the complete nullable ownership key,
with the existing creation helper on a miss. Capture sorts/materialized rows retain
only metrics and capture/ownership identity, rather than the full Platform descriptor
and connector/session admission fields. Swarm Task reassociation still uses the
physical-identity validator.
The Platform lock is acquired before the capture statement, so existing sealing
watermarks can be read together from its snapshot. Newly created sources have a
zero watermark. Each sample is checked against its source watermark before writes,
and only identities with accepted samples resolve an ownership snapshot.

Container memory keeps `memoryActive` as Docker working set and `memoryCache`
as excluded inactive filesystem cache. Optional `memoryActiveCache` records active
file LRU pages through Local, Direct and Edge collection, live views, raw history
and rollups. Non-cache memory is derived by subtracting active file cache from the
working set; it is not process RSS. Missing or inconsistent counters and preexisting
samples remain unknown. Weighted history buckets, compaction and replica totals
require complete active-cache coverage; they never treat absent values as zero.
The shared memory chart defaults to Working set and persists its view selection
in browser local storage, using the same segmented control as I/O charts. Memory
alerts and table values retain working-set semantics.

Container inventory's latest-stat enrichment resolves the physical source first,
then selects at most one raw and one compacted observation through source/time
indexes. It does not scan global timestamp history for new or idle containers.

Telemetry locking binds sorted, deduplicated UUID arrays, never serialized capture
payloads. Platform metadata uses a narrow borrowed projection; only the capture
statement encodes the complete I/O list. Writers recheck closed sessions after
acquiring scope locks and preserve last-occurrence ordering and missing-disk rules.

The two Core statistics writers share a 64 MiB conservative reservation budget.
A collection reserves immediately before constructing totals/cloned I/O; envelopes
keep their reservation through queues, in-flight writes and per-platform retries.
The container and platform portions release independently, so a completed container
batch does not consume capacity while platform totals wait for their flush. Estimates include string/vector capacity, owned scope data, growing
vectors and serialization/driver scratch allowances. Serialization is counted into
a non-allocating sink once per sample before admission; the same estimate travels
through reservation and enqueue. The temporary estimate vector is included in
accounting and stays aligned when invalid readings are filtered. This does not
build a second JSON tree. An encoded batch is bounded
by an 8 MiB estimate as well as the configured item count. Batches split only
between complete captures. Oversized or capacity-exhausted collections are dropped
as complete cycles, counted separately and logged at most once per minute; they
never stop the sampler or publish partial totals. The existing two-minute expiry,
retry order, polling/flush cadence and bounded shutdown remain unchanged.
This is an ingress accounting bound, not a process RSS limit: Docker/Agent
collection and transport decoding allocate before admission. Existing collector
concurrency/session ownership still bounds that additional work; allocator-retained
pages, realtime subscribers and other Core services are outside this budget.

Metric-alert flushes prepare cached rules and live entitlements before taking the
flush transaction. Evaluation, rule state, outbox changes and exact-`xmin`
acknowledgement then share that one transaction/connection. PostgreSQL notifications
are transactional; in-process change callbacks run only after commit. A correction
concurrent with evaluation remains pending. Once per minute, bounded diagnostics
seek at most 10,001 pending timestamps and report count (a lower bound at the cap),
oldest age, time since acknowledged progress, and whether pending work blocks raw
history compaction. Diagnostics never clear pending flags or bypass retention.

The operator controls polling through `CITADEL_STATS_POLL_INTERVAL_SECONDS`;
storage does not change that setting or the realtime capture cadence. The existing
lease-owned maintenance worker compacts captures older than 24 hours into minute
rows in `statisticsrollups`. Weighted sums/counts preserve gauge averages; first
and last counters, internal reset-aware deltas and valid elapsed durations preserve
network/disk totals and rates. CPU/memory maxima are retained internally. Readers
combine internal deltas with the bridge between adjacent physical-source blocks.
They never average averages without their original counts. Ownership changes
split a minute into contiguous segments. Segments with fewer than four samples
stay raw to avoid increasing storage for sparse polling. The newest source minute
also stays raw, preserving current usage even for offline containers. Empty
compacted raw partitions are dropped after a locked recheck. Sparse or latest
samples can keep a nonempty partition's allocated heap/index space until it empties
or expires; routine maintenance does not rewrite live history tables to shrink them.

Compaction runs within the existing lease-owned maintenance job, independently of
the configured retention sweep. Its first check waits five minutes plus the shared
scheduler's subsecond offset. After finding no eligible work or encountering an
error, it waits five minutes again. Reaching the pass limit or time budget instead
schedules another batch after a full 60-second pause, measured from completion so
slow work cannot trigger immediate catch-up ticks. Each iteration permits at most
128 passes within five seconds, with a cancellation branch; a pass commits at most
one container-source-hour and a prefix of one host/node-hour. Container candidates
are ordered by oldest unsealed observation. Host/node candidates rotate through
source IDs using one process-owned cursor; each starts at its oldest eligible
observation. Restart resets the cursor without changing durable progress. Container
sample deletion repeats the source and time bounds explicitly, allowing partition
pruning for both custom and generic prepared plans instead of scanning unrelated
retained days. Retention is not repeated for each source-hour. Scheduler and native
fixtures cover 1,024 sources plus backfill; actual throughput still depends on
PostgreSQL resources and source density. Polling and retention durations are not
changed to meet this budget.

Compaction takes the same Platform scope lock as captures, rereads after acquiring
it, and moves at most one source-hour in a transaction. Its watermark fences late
corrections to sealed minutes; cancelled/failed transactions leave raw history and
the watermark intact. No additional task or lease is introduced. Historical reads
include only complete summary blocks inside the requested interval: arbitrary
second boundaries can omit less than one minute at either edge after compaction,
rather than report traffic from outside the requested interval. Every response whose start precedes the 24-hour raw boundary uses at least
60-second buckets for its entire range, including any raw tail. Recent ranges
retain their 10/30-second detail. `StatsWindow::bucket_seconds_at` and the frontend
range helper apply the same age rule to bucketing and chart gaps. This conservative
rule also coarsens older sparse ranges that stayed raw; it needs no response
contract change or extra scan to discover summaries. Older short-range charts
therefore have minute resolution, while the newest 24 hours stay raw.

Container history resolves authorized live projection IDs to source IDs before
reading the raw and summary tables. Explicit source/time predicates remain on
both physical reads under custom and generic prepared plans. Gauges aggregate
separately from narrow physical-counter boundaries, avoiding per-raw-row summary
arrays in the window sort. Summary starts bridge to the preceding observation;
summary ends only supply the next baseline. Stored internal deltas and durations
are added once, with the original sample counts used for weighted gauges.

Frontend live statistics accumulate sums and counts at the selected history
resolution before rendering. Replica averages are summed after temporal
aggregation. Historical buckets remain authoritative on overlap because history
responses do not include the counts needed to combine partial averages. Current
usage summaries read the latest observation separately from chart averages.
Network history additionally returns `rxBytesDelta`/`txBytesDelta`: PostgreSQL
calculates reset-aware differences per physical Container or
Swarm Task before summing traffic into buckets and across replicas. The first
sample inside the requested range is a baseline; absent deltas mean insufficient
data, not zero traffic. The shared container network panel accumulates these
bucket totals for the selected window. Live observations calculate differences
before bucketing as well; CPU and memory retain their existing averages. Missing
observations can still hide traffic before an unobserved counter reset.
Container statistics also retain optional Docker block-device read/write byte
counters. They use the same reset-aware history and live aggregation as network
traffic. History returns per-second network and block-I/O rates calculated from
the sum of raw counter differences divided by their measured elapsed time in
each bucket; intervals without a counter baseline do not enter the denominator.
Live buckets use the same time weighting. Replica rates are summed only after
each replica has been aggregated in time. Instantaneous usage summaries retain
the latest raw observations independently of chart averages.
The shared container charts offer accumulated bytes and throughput views. Disk
I/O is unavailable when Docker omits the counters; old samples are not backfilled
with zeros. These additive protobuf fields use optional values on Direct and Edge
transports. Existing generation/session fencing and seven-day retention apply.

Platform samples retain the physical container I/O counters from each host capture
in `platformstats.containerio`, independently of inventory lifetime. Platform
history derives reset-aware differences per container before bucket aggregation;
live charts keep the same per-container baselines. Replacing or removing a
container never subtracts its previously measured traffic from the period totals.
The four statistics history readers and container health history share one per-Core admission limiter, acquired
after cheap validation and before a query connection. It allows at most two reads
(or one with a one-connection pool), rejects overload immediately with the existing
capacity error, and applies a transaction-local five-second statement timeout and
`jit=off`. The ordinary pool size is unchanged. A one-connection pool cannot serve
ingestion concurrently with a history read. This is not a cluster-wide limit.
Cancellation transfers the connection and permit to existing process-owned cleanup;
SQLx drains the pending response before rollback. Admission remains held through
rollback, with a six-second client I/O deadline. On error or deadline expiry the
connection is detached from the pool and its local transport dropped; it is never
reused. This bounds local cleanup ownership, not remote backend shutdown time.
Shutdown drains that cleanup before closing pools. Local
settings are restored before connection reuse; timeout returns the existing typed
error rather than an empty chart. Gauge and I/O projections read the indexed history directly
instead of materializing a shared JSON capture relation; Docker identities use
byte ordering for the per-container windows.
Platform history checks for I/O summaries inside the same query snapshot. Windows
without summaries use a narrow raw-counter calculation; mixed windows retain
summary endpoints, durations and weights. Only the selected path expands raw
captures. Within mixed windows, summary-only fields stay NULL on raw observations
during sorting, with endpoints and unit weights derived afterward. This reduces
temporary I/O without increasing PostgreSQL's memory allowance, changing missing-
counter semantics or racing compaction between separate reads.
Workload and service history resolve historical owners before scanning physical
raw/summary tables. A single owner-array InitPlan avoids repeating an underestimated
generic-plan date scan once per owner. Gauges retain their original weights, while
the counter window carries only physical IDs, timestamps and scalar endpoints.
Owner changes and replacements are combined into logical replica buckets only
after physical deltas are calculated; current projections never filter history.
Both counter paths project their bucket key before ordering by node, container and
bucket. The following aggregation reuses that order, including with generic
prepared plans, rather than building a window-wide hash table from underestimated
JSON row counts. Native tests bound temporary writes at unchanged query memory
and compare all returned counter values across raw/mixed and custom/generic paths.
The initial observation sort still scales with the requested history; this does
not eliminate all temporary I/O or impose a database memory ceiling.
The shared I/O panel is owned by `components/custom/stats-io-usage`, used by both
platform and container charts. These are container traffic and block-I/O metrics,
not host-interface or host-disk totals. Swarm node captures are retained separately
in `platformnodeiostats`, with the same bounded seven-day retention, independently
of node bindings and container inventory. History partitions counters by source
node and physical container before summing rates and deltas across the connected
host and reporting node Agents. Samples from a node Agent on the connected host
are excluded to prevent double counting. Existing Edge session, revocation and
Platform scope locks fence writes. Node captures never update manager metadata,
host gauges or host alerts. Live Platform updates use the existing authorized
group with an I/O-only payload; node identity and capture times keep independent
baselines, including when several nodes update between renders. Client capture
timestamps are retained per source even for empty captures, so delayed observations
cannot resurrect deleted containers or rewind host metadata. Empty node captures
also advance I/O retention without refreshing the host gauges. History retains
node-only buckets with null host gauges rather than fabricating zero CPU/memory.
Missing Agents or observations leave gaps; no extrapolation or host-wide totals
are inferred. CPU, memory and disk capacity still describe the connected host.
Old platform samples without physical counters remain
unknown for I/O, rather than inferring reset-safe deltas from aggregate counters.
The additive migration preserves existing samples and applied migration checksums.

Shared raw measurements and summaries use daily range partitions. Maintenance
precreates the current and next two days, verifies catalog ownership/bounds before
dropping fully expired partitions, also releases empty compacted raw days after
rechecking under exclusive locks, and retains a default partition as an ingestion
fallback. A populated default range is never detached or hidden to create a new
partition. Boundary/default partitions delete at most 5,000 expired rows per
physical child through the timestamp index and `FOR UPDATE SKIP LOCKED`.
Statement-local `ctid` arrays target each child individually: tuple locations are
not globally unique across partitions. Empty retired source/owner metadata is
cleaned in bounded batches after its measurements expire.

Platform history uses the same 24-hour raw window and seven-day retention.
`platformhistorysources` identifies the connected host (empty node ID) or reporting
node independently of bindings/inventory, and seals compacted ranges against late
Direct/Edge captures. The same maintenance lease compacts at most one host/node-hour
per pass, using the Platform scope lock and a fresh snapshot after acquiring it.
`platformgaugerollups` preserves host gauge means with sample and nullable-disk
counts; `platformiorollups` preserves each physical container's first/last observation
times, counters, internal deltas and valid durations. Both use daily partitions and
the existing bounded expiry path. Node summaries never fabricate host gauges.
The reader combines raw and summarized history, retaining per-node/container
baselines before summing rates. Empty captures and removed containers do not reset
another source's counters. Platform minutes require at least five captures, at
least five observations per distinct physical container on average, and a conservative
replacement-cost check. The check compares stored (possibly compressed/TOASTed)
JSONB sizes plus a lower raw-row allowance with 448 bytes plus twice the ID's byte
length per replacement I/O row and 384 bytes per minute for gauges/page overhead.
It reads only the selected source-hour; it performs no table-size measurement in
production. Whole unsuitable minutes remain raw, with no overlapping summaries,
and their watermark still advances so they are considered only once. The newest
host/node minute also remains raw; partial summary ranges have the same minute-resolution
limitation as container history.

Host/node compaction sizes a prefix of complete minutes under the existing scope
lock before expanding records: at most 100,000 I/O observations and 32 MiB of
uncompressed JSON per transaction, still within the existing one-hour limit.
Sizing inspects at most one minute (60 distinct timestamps) per statement; it does
not expand a whole hour to discover that it is too large. A single oversized minute
remains unsealed, emits a stalled-source diagnostic and advances the cursor so a
later source can proceed. Such a source cannot compact until the input fits;
normal protected retention still applies. A source-local stall lets the same batch
continue to other sources and preserves committed container progress. A bounded
visited set stops repeated stalled-source scans; an all-stalled batch rests for
five minutes, while a batch with remaining useful work rests for one minute.
Database errors still use the five-minute failure pause. Sizing stops after a
complete minute once its one-second elapsed budget is reached, leaving time for
compaction and commit within the worker's five-second batch budget.
Fixed metrics distinguish discovery, sizing, lock wait/hold, selected captures/input,
committed progress and stalls. Density/storage-benefit gates still decide whether
a whole selected minute is summarized or retained raw.

Raw host/node tables remain in place: host UUIDs and pending-alert claims/revisions
are not rewritten. Compaction stops before the earliest pending-alert minute, while
other sources can continue. Processed raw rows are deleted atomically with summary
insertion and watermark advancement; cancellation rolls all three back. Raw expiry
uses the bounded indexed tuple-deletion pattern and still protects pending alerts,
even beyond seven days. Vacuum reuses the deleted raw space; this extension does
not perform a blocking raw-table rewrite or promise immediate filesystem shrinkage.

Partition/compaction transactions use bounded
lock and statement timeouts and roll back on cancellation. Parent partition locks
are released before row cleanup/analysis; daily rotation also analyzes the
partitioned parents because autovacuum handles their children.
Locked rows are left for a later maintenance pass. Health transitions, Activities,
Script/Build history, refresh-token expiry and expired copy reviews use the same
statement-local locked-tuple deletion pattern with their existing cutoffs and batch
limits. Active/current runs, reserved or referenced builds and started copies stay
protected. Expiry indexes match terminal run states and unstarted copy reviews;
Activities also has a global `(createdat DESC, id DESC)` index for stable page ordering.
Exact authorized Activity counts still inspect all matching rows.

Shared measurements use `(sourceid, created)` as their primary key; node I/O
samples use `(platformid, nodeid, created)`. They do not maintain a second UUID
index for unused sample IDs. The append-only migrations preserve measurements,
owner snapshots and applied checksums. Normalization refuses conflicting copies
instead of silently choosing a value. Unchanged retries do not rewrite tuples;
corrections to unsealed captures still update under the same key. Physical history
partitions, host/node history and health history become vacuum-eligible after
1,000 plus 5% dead tuples, retaining PostgreSQL's configured vacuum cost limits.
Daily expiry releases complete partition files; ordinary row deletion makes space
reusable and does not guarantee immediate filesystem shrinkage.

Health-history retention uses per-container sample/change ordinals and partial
indexes to delete only entries beyond the 10,080-sample and 200-change caps.
Migration `0005` backfills ordinals without replacing observations or applied
migration checksums. A narrow counter row is updated for each admitted observation;
the existing global admission lock and 100,000-ID window remain. Counter rows have
no container foreign key because retained history outlives removed containers.
Global eviction and bounded maintenance remove unused counters. Maintenance takes
the admission advisory lock in a separate statement before checking for empty
counters, so its fresh snapshot includes writers that committed while it waited.
The lease-owned
maintenance sweep handles age expiry; reads independently enforce seven days.

## Runtime routing and observations

Inventory persistence serializes borrowed runtime projections directly into SQL
arguments. Swarm refreshes mark only missing identities stale; present projections
advance freshness once. Derived task counts and resource service-name lists update
only when their values change, including transitions to zero or empty. Ownership
checks, snapshot fencing and complete-snapshot removal remain transactional.

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

Backup subprocess caches also use owned directory cleanup: creation and recursive
removal run outside async worker threads, and abandoned operations retain a tracked
cleanup reservation through shutdown. Cache deletion failure does not replay a
completed backup mutation.

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
Completed cycles remount the animation component before queued work starts,
so delayed animation ticks cannot leave a visible bar after requests settle.

Realtime snapshots or scoped invalidation events own synchronization after changes
and on reconnect. Avoid parallel polling or mutation-triggered refresh-all flows.
Authoritative mutation responses may update query caches while disconnected; cancel
stale reads before applying them. Explicit error retries remain available.
Public startup status uses the shared query cache and refreshes on realtime connection
and reconnection, so password guidance follows the policy enforced by the running Core.
Background refresh failures preserve the established startup gate and last known policy.
Service-account token limits also refresh on connection and reconnection, so open
forms use the limits enforced after a Core restart; inactive reads are invalidated
and refreshed when next used.

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

Stack, Deployment and Swarm Service binding resolution selects effective definitions
before validating or resolving values. The existing case-normalized ordering and
scope precedence determine the winner; shadowed external secrets make no Vault
request. Invalid winning secrets still fail without falling back. Composition
creates one cloneable secret resolver for catalog validation, bindings, backups and
builds. Its Vault connection pool is shared, while grants, provider configuration,
tokens and plaintext values are resolved per request. Tokens stay on individual
requests; TLS, redirect, deadline and response-size policies are unchanged.

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
The shared process runner captures bounded stdout/stderr by default. Docker build
commands opt into stream-only retention: byte limits still bound forwarding and
excess output is drained, but no second transcript survives in `ProcessOutput`.
Git, probes and scripts retain capture because their callers consume final output.
On cancellation or producer failure, build output consumers have up to five seconds
to drain admitted output and finalize redaction, keeping any in-flight append alive.
Execution cleanup is still awaited and the original producer failure is preserved.
A drain timeout or persistence failure emits a warning that log delivery may be
incomplete. Successful execution flushes all admitted output. An interrupted,
unacknowledged log batch is not replayed because its commit outcome can be ambiguous.
Build and script cancellation-map entries are scoped to their controller futures;
removing an entry does not release a durable claim or an uncertain executor reservation.
Stack apply drops copied Compose text and parsed/resolved preparation data before
execution. The source bundle remains borrowed by the runtime until execution ends,
then is released before completion retries; immutable release provenance and
binding snapshots remain available. Staged release files keep their existing lifetime.

Core and Agents load the same build-owned script SDK from `scripts/citadel.js`
beside the executable; native Cargo development/tests use the generated asset in
`src/infrastructure/scripts/generated/`. `xtask openapi` bundles the HTTP helpers
with the existing handler-derived endpoint catalog and verifies it in CI. Agents
consume the generated JavaScript without depending on Server. Development builds,
debugger startup and `npm run dev` regenerate the SDK and frontend contracts through
one serialized generation wrapper. Each run's source
contains only its client context, `args`, `run` and user code. A private import map
resolves `citadel:sdk` to the executor's asset, with read permission for that exact
file and write permission only for the run workspace. The trusted bootstrap still
dynamically imports user code under Deno's permission checks. SDK files are shared;
authenticated clients, JavaScript processes and caches are not.

Core and Agent Deno caches live inside each private run workspace and are removed
with its generated source on completion, failure, timeout or cancellation. The
in-memory generated source is released once the workspace has written it. The
cache is not shared across runs: generated module paths and credentials are unique
to each execution. This bounds retained cache by active work rather than run count;
per-run imported modules are fetched again. Existing shared-cache directories from
older installations are not used or automatically deleted.

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

### Bounded schedule discovery and Swarm refresh writes

Script and Backup schedulers page at most 64 narrow candidates at a time. Candidates
contain identity, cron/time zone and (for Scripts) timeout validation fields. Code,
arguments and full policy definitions are loaded only by the existing transactional
enqueue path, which retains its control-state and scheduled-minute fences. Scheduling
cadence and entitlement checks are unchanged.

Swarm node, service, task, config, secret and network upserts compare semantic fields
before replacing payloads. Unchanged rows receive only a timestamp update, matching
the container inventory pattern. Freshness still advances on every accepted refresh:
stale failure reports and older successful observations cannot overwrite newer state.
Ownership, missing-resource removal and derived counts retain their existing rules.
This reduces payload rewrites; it does not eliminate freshness writes.
