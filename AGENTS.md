# Citadel — Agent instructions

Citadel is a self-hosted Docker Standalone and Swarm management platform built with Rust, PostgreSQL, and React/TypeScript. Optimize for **correctness, operational safety, and architectural consistency**, not maximal abstraction or minimal line count.

This is a short working contract, **not** a second architecture specification. When touching a subsystem, first read the relevant parts of:

- `docs/ARCHITECTURE.md` — ownership, dependencies, consistency, and lifecycle (authoritative design guidance).
- `docs/DEVELOPMENT.md` — local development, generators, and commands.
- `docs/agent-protocol.md` — Direct/Edge Agent authentication, transport, and Swarm-node restrictions.
- `test/README.md` — isolation requirements and targeted verification.

## Before changing code

1. Trace the existing path through the API, feature service, persistence/runtime adapter, worker/realtime boundary, and frontend **only where relevant**.
2. Inspect an equivalent implemented feature and reuse established patterns; do not infer architecture from filenames alone.
3. Identify invariants, affected API/protocol/schema contracts, cancellation/recovery behavior, and the smallest useful test scope.
4. Prefer a focused change. Do not opportunistically rename modules, redesign the architecture, or change product semantics while solving another task.

For nontrivial changes, provide a concise implementation plan in the conversation; do **not** generate a hierarchy of specification documents unless explicitly requested.

## Documentation and release context

- User-facing product docs belong in `docs/content/docs/`. Architecture and development guidance live in the repository documents listed above; keep changes aligned with those sources.
- Update affected documentation when behavior changes. Describe implemented behavior and actual limitations; do not invent historical compatibility restrictions or infer backup coverage from a UI label.
- `v0.1.0` was published on 14 October 2026. Use `version.json`, published release records and the release workflow for subsequent version decisions. Earlier development/demo versions do not establish public compatibility guarantees.
- Preserve application storage during deployments. Keep credentials, private data captures and operational journals outside this repository.

## Code ownership and boundaries

| Area | Owns |
| --- | --- |
| `src/features/<feature>/` | Business models, commands, read models, use-case services, repository/runtime **interfaces**, permissions, typed errors |
| `src/infrastructure/adapters/` | Implementations of persistence, Docker/Agent connectors, filesystem and security ports |
| `src/infrastructure/database/` | Declarative PostgreSQL schema and migration tooling |
| `src/infrastructure/contracts/`, `src/infrastructure/docker-api/` | Protobuf and Docker wire contracts / generated clients |
| `src/infrastructure/processes/`, `src/infrastructure/runtime/` | Bounded subprocess execution, task ownership, supervision and concurrency controls |
| `src/server/src/api/routes/` | Thin HTTP endpoints: extract, authorize, invoke use case, map response |
| `src/server/src/api/resources/` | HTTP DTOs, views, patch semantics and presentation mappings |
| `src/server/src/composition/`, `src/server/src/workers/` | Explicit dependency wiring, background-job construction, execution and lifecycle |
| `src/agent/` | Agent transport, authenticated execution, Direct/Edge session behavior; **no database** |
| `src/frontend/` | React screens, shared UI, state/query integration and generated API consumption |

- Features depend on features, **never** on Server, Agent, SQLx, Axum, or concrete infrastructure. Infrastructure implements feature ports; Server and Agent compose dependencies and never depend on each other.
- Cross-feature dependencies must remain acyclic. Test fixtures may use concrete implementations; SQLx in Server composition is allowed for wiring, not for moving business queries into hosting code.
- Do not reintroduce generic `domain`, `application`, or `resources` umbrella layers. Use existing resource-based modules (`model`, `commands`, `read_models`, `repository`, `service`, etc.) only where needed.
- Business resources can include the metadata they own; do not automatically invent `*Details` wrappers or duplicate read/domain types. Create a smaller projection when a consumer genuinely needs less data.
- Keep SQL, transactions, row mapping, and query authorization in `adapters/src/persistence/postgres/`. Repositories return feature types, not HTTP views or SQLx rows.
- Keep transport-specific Docker/protobuf types out of feature models. Use `PlatformRuntimeRouter` and existing narrow capabilities instead of branching on Local/Direct/Edge inside services or realtime.

## Product invariants: preserve these when changing behavior

- **Operations and recovery:** Durable changes use persisted claims, concurrency/version checks, and atomic repository operations. A browser disconnect does not cancel a claimed durable operation. Do not split one atomic operation into unrelated writes, blindly replay an ambiguous mutation, or let daemon observations finalize a parent operation.
- **Runtime and observations:** Core owns authorization, persistence, scheduling, and target selection; Docker and Agents execute. Known container lifecycle events must not trigger unnecessary full inspection. Bound work by platform/node; reject stale generations, revisions, and Edge sessions; reconcile on reconnect and missed events.
- **Realtime:** Persist state **before** broadcasting invalidations. Publish only for affected resources; keep authorization actor-specific. Notifications are hints, not the durable source of truth. Don't introduce direct transport selection or HTTP-route imports into realtime code.
- **Security:** Authorize mutations and resource reads, including catalog filtering at SQL level when required. Preserve permission-cache fencing/invalidation, secret redaction, signed Direct requests, Edge session identity and Swarm-node allowlists. Never weaken TLS/SSH verification or widen Agent operations to make a test pass.
- **Background work:** Every spawned task has an owner, a cancellation path, and defined shutdown behavior. Keep bounded channels/retries; loss-tolerant telemetry must not replace durable operation or alert state. Preserve Core's single background-job lease per database.
- **Licensing:** Preserve existing Community behavior and Team capability enforcement. Do not move a free capability behind a paid gate or introduce a license bypass incidentally. Use current product entitlements, not hardcoded UI-only checks.
- **Backup and destructive actions:** Preserve source/target identity checks, path/mount validation, data safety, cancellation cleanup and explicit restore semantics. Test destructive flows only against disposable fixtures.

## Contracts, data, and frontend

- Rust Server owns OpenAPI/`utoipa` definitions. For API changes, run `cargo run --locked -p xtask -- openapi`, then verify with `cargo run --locked -p xtask -- openapi --check` and `npm run api:verify --prefix src/frontend`. Never hand-edit generated schemas or TypeScript API models.
- The schema source is `src/infrastructure/database/src/schema/schema.sql`. `database refresh-baseline` changes new-install baselines; **it is not an upgrade migration for existing installations**. Don't change applied checksums or migration history to suppress failures.
- Preserve frontend architecture: API/shared primitives must not import resource screens or app routing; reuse resource UI patterns and shared appearance tokens. Follow the existing React Query and realtime invalidation flows rather than creating a parallel state-sync mechanism.
- Before adding a selector, filter, status indicator or hover card, inspect the shared components and an equivalent resource screen. Reuse their selection behavior, accessibility and semantic colors consistently across resource types.
- For Agent protocol or Docker API changes, review version compatibility, generation checks, Core/Agent mappings and Direct/Edge behavior together.

## Verification: proportional to the change

Choose checks for the affected area; this is not a mandatory full-suite checklist. Run from the repository root with dependencies installed, the pinned toolchain and `--locked` for Cargo:

```bash
# Rust formatting and ownership/dependency changes
cargo fmt --all -- --check
SQLX_OFFLINE=true cargo test --locked -p citadel-server --test resource_structure --test resource_architecture --test workload_architecture --test multi_resource_architecture
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test architecture

# Targeted crate validation (replace <crate>)
SQLX_OFFLINE=true cargo test --locked -p <crate>

# Frontend changes: run affected tests; broaden when scope warrants it
npm run lint --prefix src/frontend
npm --prefix src/frontend run test:run -- path/to/affected.test.tsx
# Shared frontend dependency changes
npm --prefix src/frontend run test:run -- src/test/architecture.test.ts

# Product documentation content and links
npm run validate --prefix docs
```

`SQLX_OFFLINE=true` uses checked-in query metadata; it does not isolate runtime tests from databases. Cargo's separate `--offline` flag is optional when dependencies are cached. For broad frontend changes, use `npm run test:run --prefix src/frontend`; that already includes the architecture test. Documentation-only edits do not require Rust/frontend suites.

Run broader Clippy, workspace/database, Agent acceptance, or browser suites **when the change warrants them**, following `docs/DEVELOPMENT.md` and `test/README.md`. Integration tests must use disposable databases and Docker resources; never target personal or production hosts by default. Do not modify failing tests or weaken checks merely to obtain a green run.

## Final response for each task

Summarize **what changed**, **which invariants/contracts were affected**, **tests run and their results**, and **known risks or unverified paths**. If a new abstraction or architectural exception was necessary, explain why it fits Citadel better than the established pattern.
