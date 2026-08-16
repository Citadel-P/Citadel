# Phase 1 decision report

Date: 2026-08-16
Decision: **complete; Phase 2 database work is unblocked**

## Completed

- Deterministic machine-readable inventories cover the HTTP, frontend,
  protobuf, worker, queue, serialization, connector, Docker, process, and
  database surfaces.
- Accepted OpenAPI, frontend, protobuf, Docker, configuration, and current
  database evidence are pinned by SHA-256.
- Repository configuration defaults are captured without reading local `.env`
  files and with sensitive values redacted.
- Every specification named in migration-spec Section 5.1 has a scoped
  traceability entry, current evidence, Rust owner/phase, and disposition.
- Product compatibility and obsolete internal compatibility are separated.
- The intentional breaking-change ledger and Rust-v1 database baseline plan are
  recorded.
- The complete baseline proof retained all generated seeds, and pinned DPM
  0.3.2 produced zero-diff structural convergence for all 82 product tables.

## Database decision

The previous blocker incorrectly treated inferred data-preserving renames as a
requirement for an unreleased, resettable development database. That constraint
has been removed from the Rust-v1 baseline gate.

The Rust baseline is a deterministic one-time import of the EF-generated
`script0001.sql`, excluding only EF history metadata. DPM becomes the Rust
build-time structural differ and verifier after that import. EF is not retained.
Destructive pre-release changes remain visible and reviewed; post-release
renames require an explicit data-preserving step and convergence proof.

Phase 1's inventory, traceability, compatibility, baseline, and tooling exit
criteria are met. Phase 2 may implement the database crate, embedded runner, and
Rust-v1 baseline when that phase is authorized.
