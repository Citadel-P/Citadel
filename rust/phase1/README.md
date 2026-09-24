# Phase 1 inventory and contracts

This directory contains machine-readable inputs for the .NET-to-Rust migration.
It does not make Rust a production owner.

Generated files:

- `inventory.json` inventories the current HTTP, frontend, repository, job,
  queue, serialization, realtime, connector, protobuf, database, Docker, and
  external-process surfaces.
- `accepted-contracts.json` pins the accepted OpenAPI, frontend, Docker
  OpenAPI, protobuf, and current database-baseline evidence by SHA-256.
- `configuration-defaults.json` records repository-owned defaults. Local
  `.env` files are excluded and sensitive values are redacted.

Reviewed files:

- `traceability.json` assigns each specification in migration-spec Section 5.1
  to current evidence, a Rust owner, a phase, and a disposition.
- `breaking-changes.json` is the closed ledger of intentional Rust-cutover
  divergences. An unlisted divergence is a defect.

These inventories are archived migration evidence. Their source-dependent
generators have been retired. Current protocols belong to
`rust/crates/infrastructure/contracts/proto`; Docker API generation uses
`rust/crates/infrastructure/docker-api/codegen/v1.49.yaml`.

Phase 1 is complete. Pinned `declarative-postgres-migrate` 0.3.2 passed the
complete 82-table product-schema and convergence proof after the pre-release
rename constraint was correctly removed. See
`rust/reports/phase1-schema-tooling-adr.md`.
