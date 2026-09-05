# ADR: Rust declarative PostgreSQL schema tooling

Status: **accepted**
Date: 2026-08-16

## Context

Citadel needs one Rust-owned declarative PostgreSQL schema, generated immutable
structural migrations, an auditable migration runner, and no retained EF
migration project. Citadel is not released, so the first Rust baseline may
replace the development database and does not need to preserve column-rename
history from unreleased builds.

The accepted input is the EF-generated
`src/Citadel.Infrastructure/Scripts/script0001.sql`. It is the exact executable
PostgreSQL snapshot. `ApplicationDbContext.cs` remains a semantic cross-check
for conversions, constraints, indexes, and seed intent while importing the
baseline; it is not copied as a runtime abstraction or retained as a migration
dependency.

## Candidate evidence

| Candidate | Evidence | Decision |
| --- | --- | --- |
| Diesel CLI `diff-schema` | Diesel documents generated diffs as a starting point and cannot represent parts of Citadel's accepted schema, including custom checks. | Rejected. |
| Atlas Community 1.3.0 | The pinned Community binary preserved the measured relational surface, but is not Rust-native and is unnecessary once the pre-release rename constraint is removed. | Rejected. |
| `declarative-postgres-migrate` 0.3.2 | Rust-native; accepts executable SQL as the declarative source; introspects real PostgreSQL catalogs; emits reviewable SQL; independently gates destructive generation and execution; and verifies convergence. The complete Citadel proof preserved 82 product tables, 126 foreign keys, 9 checks, 140 unique indexes, 29 partial indexes, 43 JSONB columns, 147 defaults, and 3 identities. | **Accepted.** |
| `drizzle-migrations` / `drizzle-cli` 0.1.15 | Introspected all current tables and generated Rust declarations, but could not diff its own generated complete schema against its snapshot (`missing field increment`). | Rejected. |

Reproduce the selected proof with:

```powershell
./rust/scripts/Evaluate-Phase1DpmCandidate.ps1
```

The proof deterministically removes only the EF history table and history row
from the accepted generated SQL. It retains all 82 product tables and all 92
seed inserts, applies that baseline to disposable PostgreSQL, verifies a
migration from an empty database, applies the generated structural migration,
and requires a zero diff plus matching catalog metrics. Its containers, network,
and temporary files are removed in `finally` blocks.

The Atlas and Drizzle scripts remain negative evidence for the rejected
candidates. They are not dependencies of the Rust product.

## Decision

Use pinned `declarative-postgres-migrate` 0.3.2 as the build-time structural
schema differ and convergence verifier. SQLx remains Citadel's application query
layer. The Rust binary will use Citadel's embedded migration runner, not the DPM
CLI, in production.

For the initial baseline:

1. copy the latest accepted generated `script0001.sql` deterministically;
2. remove only `__EFMigrationsHistory` DDL and its final journal insert;
3. retain the complete product DDL and generated seed data;
4. record the source hash and verify the resulting 82-table baseline; and
5. make the copied result the Rust-owned `schema.sql` and generated Rust-v1
   baseline migration.

After that one-time import, EF and `script0001.sql` leave the Rust schema
pipeline. Until the first Rust release, structural changes regenerate the sole
`0001_initial.sql` baseline from the Rust-owned schema. The first release freezes
that baseline; subsequent structural changes generate reviewable immutable
migrations with DPM.

Before Citadel's first public release, a rename may intentionally appear as a
destructive drop/add because development data is disposable. Destructive SQL is
still explicit and reviewed. Once persisted user data becomes a compatibility
contract, a rename must either preserve the existing database name or include a
reviewed explicit rename/data step followed by DPM convergence verification.
Lack of inferred rename support is therefore not a pre-release blocker.

## References

- [`declarative-postgres-migrate` crate documentation](https://docs.rs/declarative-postgres-migrate/0.3.2/dpm/)
- [Diesel CLI `diff-schema` documentation](https://diesel.rs/guides/configuring-diesel-cli.html)
- [Atlas versioned migration documentation](https://atlasgo.io/versioned/diff)
- [`drizzle-migrations` crate documentation](https://docs.rs/drizzle-migrations/0.1.15/drizzle_migrations/)
