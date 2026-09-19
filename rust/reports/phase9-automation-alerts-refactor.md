# Phase 9 — Automation and Alerts

Status: Phase 9 complete and validated. Phase 10 has not started.

## Files created, moved and removed

- Replaced Automation's mixed `lib.rs` with `actions/`, `runs/`, `jobs/`, `repository.rs`, `runtime.rs`, `tasks.rs`, `permissions.rs`, `error.rs`, and operation modules under `service/`. Moved action patches, progress, log redaction, cron matching and generated Deno source into named modules. Retained `options.rs`.
- Replaced Alerts' mixed `lib.rs` with `channels/`, `rules/`, `events/`, `repository.rs`, `runtime.rs`, `service.rs`, `permissions.rs`, `error.rs`, and configuration helpers. Rule metadata moved under `rules/`; quiet-hours and time-zone helpers retain their semantic names. Pure scope/threshold evaluation now belongs to `rules/evaluation.rs`.
- Removed flat `automation_store.rs` and `alert_store.rs`. PostgreSQL persistence now lives under `adapters/src/postgres/{automation,alerts}/`, separating resource operations, claims, row decoding, recovery, validation, SQL and activity recording. Moved Shoutrrr delivery into `adapters/src/alert_delivery.rs`.
- Removed flat `automation_http.rs` and `alerts_http.rs`. Server requests, views, explicit conversions, capabilities, task registration and resource handlers live under `api/{automation,alerts}/`.
- Updated state construction, realtime mapping, consumers, test fixtures, Cargo dependencies and `ARCHITECTURE.md`. Extended architecture guards; added ordinal-permission regression coverage and direct-run lifecycle coverage.

No helper scripts, generated audit inventories or test logs were added to Git.

## Architectural decisions

`AutomationAction`, `AutomationRun`, `AlertChannel`, `AlertRule` and `AlertEvent` are business resources. Read results and progress messages are semantic data; server DTOs own serialization and Utoipa schemas. Configuration and partial-update types retain Serde for JSON interpretation. `AutomationRunClaim` remains workflow state, and token issuance, entitlements and process policies remain runtime contracts.

`AutomationRepository` retains atomic enqueue/claim/completion and action ownership. `AlertRepository` retains incident deduplication, rule-state/outbox transactions, claim fencing and retries. Resource namespaces do not split transactions across unrelated ports.

Direct Automation runs enter the process `DynamicTasks` through `AutomationTaskSpawner` before acquiring a claim. The accepted task owns claim acquisition and completion even when the requesting future disappears during a database call. It combines viewer-disconnect cancellation with the root shutdown token. Shutdown drains completion before persistence closes; rejected admission creates no claim. Scheduled/worker executions remain independent of viewers.

Automation consumes Git's semantic webhook configuration. The server reuses its Git webhook DTO with explicit conversions. Automation no longer consumes Platform-owned capability views; server capability projection uses typed permission levels and explicit administrator access. Migrated SQL binds accepted ordinal levels instead of permission bit masks.

## Compatibility and behavior differences

The refactor preserves routes, JSON fields/defaults/null handling, progress framing, realtime payloads and historical schema identities. No database migration is introduced. The OpenAPI check passed against the unchanged baseline: 404 full and 305 public operations.

Direct-run client disconnect still cancels and persists `Cancelled`. Root shutdown now also owns cancellation/drain. Admission after shutdown returns the existing conflict error form without creating a run.

Malformed stored permission values such as `3` and `7` no longer grant list access or Automation capabilities. Valid Read/Write/Execute hierarchy remains intact.

Paid trigger/alerting checks, run-as tokens, Deno sandbox and output limits, credential redaction, schedules, webhooks, audit events, alert severity selection, cooldown, quiet-hours, scoping and delivery retry behavior retain their existing contracts.

## Intentionally retained legacy

- Shared tags and Resources' compatibility webhook schema remain for Phase 10. Automation itself uses Git's semantic configuration and server-owned wire types.
- Platform/Identity/shared-resource restructuring remains Phase 10. Transitional `domain`/`application` umbrella crates remain Phase 11.
- Global server/runtime adapter placement remains Phase 12; the existing process runner and shared transport tree are retained.
- Cross-feature observation, tag, webhook and process contracts are documented in `ARCHITECTURE.md`; they do not authorize feature-owned HTTP views.

## Validation

Validation disables debug information and incremental compilation to limit artifact growth. External suites use dedicated databases in a disposable PostgreSQL container. Deno is extracted from the image digest pinned in `rust/Dockerfile` (2.5.2); Shoutrrr 0.19.0 uses the archive and checksum pinned in `install-server-deps.sh`.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed |
| `cargo test --locked --workspace` | 658 passed, 0 failed, 239 ignored by the default workspace run; includes expanded architecture guards and typed capability coverage |
| `cargo run --locked -p xtask -- openapi --check` | Passed: 404 full / 305 public operations; baseline unchanged |
| Adapter `automation_execution` with PostgreSQL | 1 passed: exclusive claims, interrupted-run recovery and scheduling contracts |
| Adapter `alert_persistence` with PostgreSQL | 3 passed: severity selection, suppression/thresholds, atomic mutations and incident deduplication |
| Adapter `phase9_permissions` with PostgreSQL | 1 passed: ordinal ACL filtering and typed capability aggregation reject invalid levels |
| Adapter `automation_external_acceptance` with Deno and Shoutrrr | 1 passed: real execution, sandbox/output limits, tokens/redaction, results/audit, cancellation and delivery retry |
| Server `automation_http_execution` with PostgreSQL and Deno | 1 suite passed: execution, PATCH/defaults, webhooks, disconnect/backpressure, dropped request during a locked claim, connected-viewer shutdown and rejected admission |
| Server `resources_http` with PostgreSQL | 1 authorization/catalogue/webhook suite passed |
| Server `phase7_resources_http` with PostgreSQL | 1 multi-resource lifecycle suite passed, including Alert create/config/metadata/authorization |

The selected external suites total 9 passing tests. All required Phase 9 checks and
external fixtures were available; no Phase 9 external suite was skipped for a missing
dependency. Formatting, strict Clippy, the full workspace suite and OpenAPI verification
passed again after the fixture corrections. The disposable PostgreSQL container was
removed, and `git diff --check` passed.

The first Deno/Shoutrrr run exposed a pre-existing fixture conflict: the seeded
Critical failure rule correctly outranked the fixture's Warning rule. The fixture
now temporarily disables that seeded rule in its disposable database and restores
it afterward. Its original delivery assertions pass; production severity selection
is unchanged and remains covered by `alert_persistence`.

## Exemptions

Retired all 87 Phase 9 entries from the external Phase 0/1 allowlists (37 and 50 respectively). No new architecture exemption is added. The expanded Rust architecture guards cover both migrated feature crates and their PostgreSQL adapters. Retained cross-feature contracts and later-phase ownership are documented above and in `ARCHITECTURE.md`.
