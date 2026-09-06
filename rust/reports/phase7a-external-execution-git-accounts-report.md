# Phase 7: external execution implementation status

## Outcome

The Rust server now owns the shared bounded-process boundary and the first
production-shaped local execution paths for Git, automation, builds, alert
channels, and backups. This report deliberately distinguishes those working
paths from connector and disaster-recovery work that is still open.

## Implemented boundary

- `citadel-execution` owns direct child-process invocation. It never invokes a
  shell implicitly, captures stdout and stderr concurrently, applies hard byte
  limits and a deadline, observes request/shutdown cancellation, and kills and
  reaps the child before returning from a non-successful control path.
- Process requests intentionally have no `Debug` implementation because
  arguments, environment variables, and stdin may contain credentials.
- `citadel-git` owns Git-account models and validation plus the bounded Git CLI
  adapter. Remote and branch values are validated before becoming arguments;
  first clone uses a sibling staging directory and exposes the cache only via
  rename after success; an existing cache fetches, checks out, and hard-resets
  its tracked branch.
- Git-account create/list/get/config/update/delete routes retain the existing
  React/.NET method, path, payload, response, error, and capability contracts.
- Git-account credentials are encrypted as one authenticated envelope before
  PostgreSQL receives them. Ordinary projections omit configuration. The
  explicit authorized configuration route decrypts it and all responses use
  `no-store`.
- List filtering happens in PostgreSQL against Actor, Team, global Role, and
  resource-override access. Every bulk-delete ID is authorized first, and the
  store locks and verifies the complete distinct set before deleting it.
- Successful mutations publish a metadata-free global realtime invalidation
  only after persistence succeeds.

## Failure and recovery semantics

- Output overflow, timeout, cancellation, spawn failure, I/O failure, and a
  non-zero exit status are distinct outcomes.
- Output buffers are bounded on both streams and drained concurrently, so a
  noisy child cannot deadlock Citadel or grow memory without bound.
- A cancelled or failed initial clone removes its private staging directory;
  it cannot replace a previously valid cache.
- Git account uniqueness conflicts map to HTTP 409. Missing batch members
  cause a transaction rollback rather than a partial delete.

## Ported reference tests

| .NET reference | Rust evidence |
| --- | --- |
| `GitAccountCreateTests` | Git-account unit validation plus the PostgreSQL-backed Axum lifecycle cover Basic/Token/SSH shape validation, encrypted persistence, duplicate-name conflict, and safe response projection. |
| `GitAccountPatchTests` / `GitAccountDeleteTests` / Git-account view tests | The same lifecycle exercises authorized reads/config, replacement, complete-set delete semantics, and capability projection. |
| `GitCliRepositoryTests` process-safety cases | `citadel-execution/tests/process_runner.rs` and `citadel-git::cli::tests` cover stdout/stderr, non-zero exit, timeout, cancellation, output overflow, option injection, branch parsing, and fetch/reset ordering. |

The .NET content browsing, compose discovery, synchronization-job, provider
credential transport, webhook, and Forgejo acceptance tests remain mapped to
the next Git execution slice. They are not replaced by mocks that report
success.

## Verification

Run `./rust/scripts/Test-Phase7AExternalExecution.ps1`. The gate runs formatting,
warning-free Clippy, process/Git unit tests, the PostgreSQL-backed authorized
HTTP lifecycle, and generated OpenAPI parity.

The destructive recovery differential requires two disposable PostgreSQL
databases and PostgreSQL client tools. Set
`CITADEL_PHASE7_RECOVERY_SOURCE_DATABASE_URL` and
`CITADEL_PHASE7_RECOVERY_TARGET_DATABASE_URL` to different databases, then run
`cargo test -p citadel-adapters --test citadel_system_recovery -- --ignored`.
Never point either variable at a database containing valuable state.

## Additional implemented slices

- Git repositories use exclusive durable synchronization claims, recover stale
  work, inject decrypted credentials only into the child process, atomically
  publish the initial clone, and expose bounded refs, trees, files, comparison,
  Compose discovery, and authenticated webhook routes.
- Automation Actions persist definitions and runs, schedule with five-field
  cron expressions and time zones, issue a run-scoped token only at execution,
  execute Deno with bounded output/time/cancellation, redact common credential
  forms, and recover interrupted claims.
- Build Projects and Agent Pool definitions retain the existing HTTP contract.
  Local and regular-Agent build runs claim durably, bind logs and child
  lifetime, resolve encrypted build secrets at execution, and persist terminal
  results. Agent builds transport one bounded immutable Git archive, signed
  requests, Docker-compatible Registry authentication, BuildKit secrets, and
  bounded Build/Push output. Platform-backed Edge builds now use the same
  canonical messages through authenticated inbound sessions.
- Alert channels, rules, events, acknowledgement, resolution, incident
  deduplication, and verification are persisted. Channel delivery uses the
  pinned Shoutrrr executable through the bounded process runner and supports
  every destination exposed by the existing frontend contract.
- Backup repositories, policies, scheduled/manual runs, restores, repository
  operations, retention, cancellation, repository leases, stale-run recovery,
  and bounded logs are persisted. Local Docker-volume execution resolves
  encrypted Restic/S3 secrets only at execution. The worker re-authorizes the
  current run-as identity and dependencies immediately before backup or restore.
- Resource list queries are authorization-filtered in PostgreSQL. Build and
  Backup dependency references and resource tags are validated and persisted
  transactionally.
- The production image pins and includes Git, OpenSSH, Docker CLI, Deno, and
  Shoutrrr. Image creation verifies Docker/OpenAPI/database generation and the
  complete Rust workspace test suite.
- Build and opted-in Automation failures pass through persisted Alert Rule
  evaluation. New Alert Events and active-channel deliveries commit atomically;
  a supervised worker claims deliveries, recovers stale leases, retries with
  bounded backoff, and dead-letters repeated failures.
- Backup runs resolve their source immediately before execution into immutable
  per-volume items. Deployment, standalone Stack, Web-editor Swarm Stack, and
  managed Swarm Service sources reject bind/anonymous or unstable placement,
  deduplicate shared Volumes, acquire child leases atomically, and persist each
  item outcome. Exact-node execution uses the selected authenticated node
  session, or verifies the connected manager's live Node ID before execution.
  Missing coverage fails closed. Git-backed Swarm Stack plans resolve the configured
  Compose files from one immutable synchronized commit before deriving mounts.
- Citadel-system policies produce a private PostgreSQL custom-format bundle,
  stream SHA-256 checksums, upload it with Core-local Restic, and remove staging
  data after the attempt. The offline `restore-system` command requires an
  explicit replacement acknowledgement, rejects symlinks/path traversal and
  any checksum mismatch before execution, and invokes `pg_restore` with
  clean/exit-on-error/single-transaction semantics. Database credentials are
  provided only through the child environment.
- Platform CPU, RAM, and disk threshold observations reuse the existing stats
  cadence. Required consecutive matches, cooldown, quiet hours, incident
  deduplication, normal-state reset, and automatic resolution are persisted per
  rule and resource. All applicable rules are evaluated independently.
- Failed managed Swarm Service operations and license transitions into grace or
  expiry now enter the same Alert pipeline after their authoritative state has
  been persisted.
- Git-backed Stack Apply materializes all configured Compose and environment
  files from one synchronized immutable commit. Local and regular-Agent
  execution receive the same bounded source bundle and generated ownership
  override. The successful release source and Git update baseline are committed
  together, so an applied commit is not immediately reported as an update.
- Stack and Deployment binding/configuration failures, explicit Stack drift
  evaluations and repairs, and Git webhook authentication/dispatch failures now
  use the durable Alert pipeline. Webhook Alerts contain bounded metadata only;
  request bodies and credentials are never copied into Alert persistence.
- A bounded Stack drift worker now evaluates eligible healthy/degraded Stacks
  every five minutes, isolates failures per Stack, and runs the existing
  policy-controlled reconciliation path for `AutoFix` Stacks. Intentionally
  stopped and currently-processing Stacks are excluded by the persistence
  query.
- Build-backed Deployment Apply resolves either the explicitly selected
  successful Build Run or the latest successful run, pulls that exact image
  through the Deployment Platform connector, and persists resolved/applied
  Build provenance only after the Container starts successfully. It never
  silently falls forward from a missing explicitly selected Build Run.
- Stack Build-image bindings now resolve retained or latest successful Build
  artifacts, pin the generated Compose override to the resolved digest, and
  persist exact applied Build provenance only after a successful Stack Apply.
  The user's Compose files remain unchanged.
- S3-compatible Volume backup and restore can now execute through the configured
  regular Agent using signed Container create/start/exec/delete calls. Output,
  execution time, and helper lifetime are bounded; credentials are carried in
  the exec environment rather than arguments. Exact-node plans verify the
  Agent's reported Swarm Node ID before touching the Volume.
- Standalone Stack Apply now executes configured pre/post commands locally and
  transmits them through the regular Agent. Registry credentials are resolved
  only for the Apply attempt, written to a private transient Docker config for
  local execution, and sent in the existing signed Agent request. Swarm Apply
  rejects Compose-only destroy/pre/post options instead of silently ignoring
  them.
- `citadel_system_recovery` provides an opt-in differential acceptance test
  that creates a bundle from one dedicated PostgreSQL database, restores it
  into a different clean database, and verifies persisted Citadel state.

## Remaining Phase 7 work

The 2026-09-05 statistics follow-up is tracked separately in
`runtime-statistics-parity-report.md`. It closes the missing history endpoints
and Service-history attribution, not the execution/acceptance gates below.
The Edge follow-up is documented in `phase7-edge-execution-report.md`. It adds
enrollment, authenticated sessions, Platform-backed builds, exact-node Volume
backup routing, and protocol/recovery tests. It does not close the full exit gate.

1. Finish Build Agent Pool provisioning/enrollment/execution, node-agent
   installation/bootstrap management and node-identity rebind recovery.
   Ordinary Platform-backed Edge builds are wired; pool-backed builds are not.
2. Finish node inventory/statistics supervision and the complete multi-node
   backup/restore acceptance matrix, including Citadel-system placement and
   interrupted operations. Exact-node Volume routing and failure cleanup are
   implemented, but they are not a substitute for real multi-node acceptance.
3. Port the remaining product-event Alert producers. Platform stats and
   reachability, managed Swarm Service operation failures, Build failures,
   opted-in Automation failures, license grace/expiry transitions, configuration
   resolution failures, periodic Stack drift evaluation, and Git webhook
   failures now use the durable pipeline. Git/image update failure producers
   remain.
4. Add real Docker/registry/Deno/Shoutrrr/Restic differential acceptance across
   supported connector types. The Phase 7 exit gate remains open until the full
   backup/restore matrix and interrupted-operation recovery pass.
5. Complete Edge workload Apply/mutation and interactive runtime transport,
   including logs/terminal compatibility and incremental Build log events.
