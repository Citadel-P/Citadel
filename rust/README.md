# Citadel Rust migration workspace

This workspace contains the isolated migration work described in
`Citadel.Internals/specs/dotnet-to-rust-migration.md`. It does not own product
routes, mutate customer state, migrate the database, or replace the .NET Core.

The prototype proves a small release server, Citadel's existing authorized
Platform read, a generated Docker Engine read/stream subset over a Unix socket,
the Phase 0B signed gRPC handshake/read/stats stream against the active .NET
Agent, and the Phase 0C authorized versioned WebSocket subscription with
bounded overflow/resync behavior. See the corresponding decision and effective
configuration reports under `reports/`, and `scripts/README.md` for repeatable
build, interoperability, realtime, and measurement commands.

Phase 1 also contains a deterministic inventory, accepted-contract hashes,
configuration defaults, specification traceability, a breaking-change ledger,
and the accepted database-baseline plan. Pinned DPM 0.3.2 passed the complete
product-schema proof, so Phase 1 is complete and Phase 2 database work is no
longer blocked. See `phase1/README.md` and
`reports/phase1-decision-report.md`.
