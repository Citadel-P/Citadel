# Phase 1 inventory and contract report

Date: 2026-08-16

## Measured surface

The checked-in generated inventory currently records:

| Surface | Count |
| --- | ---: |
| Named full OpenAPI operations | 401 |
| Public OpenAPI operations | 304 |
| Active generated frontend operations | 401 |
| `IUnitOfWork` repository capabilities | 56 |
| Repository/query methods | 665 |
| Hosted services | 31 |
| Bounded queue construction sites | 27 |
| JSON source-generation roots | 1,103 |
| Realtime references | 661 |
| Connector operations | 317 |
| Protobuf files | 10 |
| Protobuf messages/enums | 232 |
| Protobuf RPCs | 69 |
| PostgreSQL tables, including EF history | 83 |
| Docker Engine HTTP operations | 105 |
| External-process launch sites | 4 |

These are omission counters, not Rust design targets. They deliberately expose
the breadth of the current product so a rewrite cannot appear complete by
implementing only the obvious routes.

## Accepted descriptors

`rust/phase1/accepted-contracts.json` pins:

- the full and public OpenAPI documents;
- the generated React operation catalog and generated API types;
- the Docker Engine v1.49 OpenAPI source;
- all ten shared Agent/Edge protobuf source descriptors; and
- the current EF-generated development SQL baseline as evidence only.

The language-neutral source documents remain canonical. Hashes detect an
unreviewed change; they are not a second copy of a contract.

The verifier requires every generated frontend operation to have the same
operation ID, HTTP method, and path in the full OpenAPI document. It also
requires every public operation to be present in the full document and checks
every pinned file hash.

## Configuration

`configuration-defaults.json` is generated only from `.env.example` and
`src/Citadel.WebApi/appsettings.json`. It excludes local `.env` files and
redacts password, token, credential, API-key, JWT signing-key, and encryption
key values. This inventory will drive Phase 2 configuration validation; it does
not copy secrets into Rust.

## Traceability

`traceability.json` contains one accepted migration entry for every one of the
36 specifications named in migration-spec Section 5.1. Each entry has current
.NET ownership/test evidence, a Rust owner and phase, and an explicit
preserve/change/deferral disposition. A row scopes itself to named relevant
sections and does not claim that unrelated future sections are implemented.
