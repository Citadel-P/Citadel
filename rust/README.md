# Citadel Rust Phase 0

This workspace is the isolated Phase 0A viability prototype described in
`Citadel.Internals/specs/dotnet-to-rust-migration.md`. It does not own product
routes, mutate customer state, migrate the database, or replace the .NET Core.

The prototype proves a small release server, Citadel's existing authorized
Platform read, a generated Docker Engine read/stream subset over a Unix socket,
the Phase 0B signed gRPC handshake/read/stats stream against the active .NET
Agent, and the Phase 0C authorized versioned WebSocket subscription with
bounded overflow/resync behavior. See the corresponding decision and effective
configuration reports under `reports/`, and `scripts/README.md` for repeatable
build, interoperability, realtime, and measurement commands.
