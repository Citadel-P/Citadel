# Phase 2 Rust foundation report

Date: 2026-08-16

## Outcome

The Phase 2 foundation is implemented without transferring any Citadel product
mutation route to Rust. The .NET Core remains the shipping product while later
slices migrate behavior behind differential tests.

## Implemented

- `citadel-database` embeds one generated Rust-v1 baseline, validates the
  generated manifest/checksum, serializes startup with a PostgreSQL advisory
  lock, records partial failures, rejects unknown or modified history, and
  exposes both `citadel-server migrate` and migrate-before-listen `serve`.
- `cargo xtask database import-baseline` verifies the accepted .NET baseline hash and removes
  only the EF history wrapper. The output retains 82 product tables and 92 seed
  inserts. DPM 0.3.2 remains the selected declarative schema differ.
- The server validates `ReverseProxy`, `Direct`, and `Disabled` transport
  configuration. It implements trusted forwarding, explicit host validation,
  Direct-mode TLS/HSTS, CORS, request IDs, bounded request bodies, a bounded
  global foundation rate limit, centralized Problem Details, setup gating, and
  optional SPA fallback.
- One explicit typed route catalog supplies Axum registration and generates
  deterministic full/public OpenAPI plus frontend operation/type metadata.
  Production startup does not generate API artifacts.
- The representative Actor-scoped PostgreSQL query uses `query_file!` and
  committed SQLx offline metadata. Application queries remain in `adapters`;
  the database crate is not an ORM or generic repository.
- The pinned Docker Engine generator now includes Swarm inspect in addition to
  the Phase 0 read/stream subset.
- The application crate owns a fixed-capacity queue helper with explicit wait
  or reject overflow behavior and cancellation. The Docker event worker uses
  the wait/backpressure policy.
- The CLI exposes bounded Linux cgroup/process diagnostics for memory, pids,
  threads, RSS, and file descriptors. Effective configuration output contains
  no database password, token, private-key path, or trusted proxy value.

The Rust listener currently hosts the HTTP API foundation. The Edge Agent gRPC
listener and its service implementation remain in the later Agent/Edge slice;
Phase 2 validates its public origin and reserves a distinct configured port but
does not expose a placeholder listener.

## Verification

The following passed in the pinned Linux Rust environment:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace`
- `rust/scripts/Test-Phase2Foundation.ps1`

The foundation script uses disposable, uniquely named Docker resources. It
proves concurrent migration startup, clean installation, seed state, restart
idempotence, unknown/checksum refusal, offline Actor-scope SQL, generated
artifact freshness, migrate-before-listen startup, health/readiness/OpenAPI,
and graceful shutdown.

## Phase boundary

Phase 3 starts identity and access. It must not bypass the checked
Actor-authorized SQL pattern, introduce an unbounded credential/session cache,
or publish product routes outside the explicit catalog. No product write
ownership is authorized by this report.
