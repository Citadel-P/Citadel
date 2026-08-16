# Citadel Rust migration workspace

This workspace contains the isolated migration work described in
`Citadel.Internals/specs/dotnet-to-rust-migration.md`. Rust now owns its
generated v1 database baseline and migration runner, but it does not yet own
product routes or replace the .NET Core.

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

Phase 2 adds the production foundation: validated Direct TLS, trusted reverse
proxy, and explicitly insecure transport modes; request/security middleware;
generated full/public OpenAPI and frontend metadata; embedded checksummed
database migrations; SQLx offline query checking; a generated Swarm inspect
operation; bounded queues; setup/readiness behavior; SPA fallback; and cgroup
diagnostics. Run `./rust/scripts/Test-Phase2Foundation.ps1` for the disposable
PostgreSQL, startup, restart, generation, and Actor-scoped query checks. See
`reports/phase2-foundation-report.md` for scope and evidence.

## Repository direction

The completed Core-first phases stay isolated under `rust/` while the existing
.NET Agent repository remains compatibility evidence. Before Rust Agent work
begins, its relevant Git history will be imported into this repository. The
target is one Cargo workspace with separate Core, Agent, and Edge Agent
binaries, not one combined process and not one source repository per binary.

Regular Agent and Edge Agent will share a bounded Agent runtime while retaining
separate transport composition, images, configuration, security profiles,
component tags, release jobs, installation, and rollback. The Contracts
submodule remains authoritative only for active .NET compatibility; the final
Rust workspace will own the language-neutral contract sources and verify every
affected binary on contract changes.
