# Citadel Rust Phase 0A

This workspace is the isolated Phase 0A viability prototype described in
`Citadel.Internals/specs/dotnet-to-rust-migration.md`. It does not own product
routes, mutate customer state, migrate the database, or replace the .NET Core.

The prototype proves a small release server, Citadel's existing authorized
Platform read, and a generated Docker Engine read/stream subset over a Unix
socket. See `reports/phase0a-decision-report.md` for the provisional gate and
`scripts/README.md` for repeatable build and soak commands.

