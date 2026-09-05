# Phase 1 Rust database baseline plan

Status: **accepted; ready for Phase 2 implementation**

## Accepted input

The current generated development baseline contains 83 public tables:

- 82 Citadel product tables; and
- `__EFMigrationsHistory`, which is .NET tooling metadata and is not a product
  table.

`src/Citadel.Infrastructure/Scripts/script0001.sql` is pinned in the Phase 1
contract manifest as the exact executable PostgreSQL snapshot.
`ApplicationDbContext.cs` supplies semantic evidence when reviewing the import.
Neither remains part of the Rust migration pipeline after the baseline is
accepted.

## Rust v1 baseline

Phase 2 will:

1. deterministically copy the pinned generated SQL and remove only the
   `__EFMigrationsHistory` table and its journal insert;
2. require exactly 82 product tables and retain all 92 generated seed inserts;
3. store that result as the Rust-owned declarative `schema.sql` and generated
   `0001_initial.sql` baseline, with source and output checksums;
4. apply it with the Rust migration runner under a Citadel-specific PostgreSQL
   advisory lock and record checksum, name, start time, completion time, and
   failure evidence in a Rust-owned history table;
5. verify a zero structural diff with pinned DPM 0.3.2; and
6. run clean install, concurrent startup, failed migration, checksum mismatch,
   restart, and restore tests on supported PostgreSQL versions.

Before the first Rust release, structural changes update the Rust-owned SQL
schema and regenerate the single `0001_initial.sql` baseline. Development data
that has applied an older checksum must be reset. After release freezes that
baseline, later structural changes use immutable DPM-generated migrations.
Generated destructive statements require explicit review. Seed and other data
changes are explicit bounded data steps because a structural schema differ does
not manage table contents.

## Pre-release cutover

Development and dogfood environments may reset their database for Rust v1.
Renaming or dropping a column may therefore use generated drop/add SQL before
the first public release. There is no EF-history importer and no promise to
replay the unreleased .NET migration chain.

Once released data becomes a compatibility promise, destructive migrations
must preserve user data through an explicit reviewed rename or transformation
and prove convergence. This later safety rule does not block creating the clean
pre-release baseline.

The .NET migration project remains authoritative only for the running .NET
implementation until the Rust candidate replaces it. There is never mixed
production schema ownership.
