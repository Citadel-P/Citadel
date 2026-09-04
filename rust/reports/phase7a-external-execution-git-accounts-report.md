# Phase 7A: external execution and Git-account foundation

## Outcome

Phase 7A establishes the external-process boundary used by the remaining
Phase 7 workflows and ports the complete Git-account HTTP lifecycle. It does
not claim that Git repository synchronization, webhooks, builds, automations,
alerts, or backups are complete.

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

## Remaining Phase 7 work

1. Durable Git synchronization claims, encrypted credential injection,
   repository refs/content/compare/compose discovery, polling, and webhooks.
2. Builds and external build agents.
3. Automation/Deno execution.
4. Alert evaluation and bounded delivery.
5. Backup repositories, policies, backup/restore workers, and interrupted-run
   recovery—the Phase 7 exit gate.

