# OpenAPI ownership

Citadel uses Utoipa 5 and utoipa-axum. `#[utoipa::path]` on each HTTP handler
owns its method, path, operation ID, parameters, responses and exposure metadata.
A resource's `documented_routes()` factory registers these handlers with
`utoipa_axum::routes!`. Both the live Axum router and the offline exporter use
that factory. Documentation generation needs no database or Docker connection.

Request and response DTOs derive `utoipa::ToSchema` in their owning crate.
Serde field names, optional fields, defaults, skipped fields and tagged enums
are reflected in the document. Use explicit `#[schema(...)]` attributes for
custom serializers/deserializers. In particular, PATCH wrappers describe an
optional nullable value, not their internal missing/null/value enum.

Reusable Problem Details responses live in `errors.rs`. The small router
adapter normalizes Axum capture names while retaining .NET's documented names;
without it, equivalent `{id}` and `{resourceId}` paths conflict in Axum.

`x-citadel-public` selects operations in the public document.
`x-citadel-setup-exempt` is also consumed by setup middleware.
`x-citadel-principal` records the required principal category; authorization
continues to run in the existing middleware and handlers.

## Export and verification

The **Citadel: Build Rust API** task compiles the backend and exports its specs.
Both **Ctrl+Shift+B** and **F5 → Citadel: Debug application** run this step
before starting the API. From a terminal, use `bash rust/scripts/build.sh`
from the repository root. Plain `cargo build` only compiles; it does not export.

To export separately in VS Code, use **Terminal → Run Task → Citadel: Generate OpenAPI spec**.
Use **Citadel: Verify OpenAPI spec** to check freshness without writing files.
The tasks set the Rust workspace directory and `SQLX_OFFLINE=true` automatically.
Generated files are `schema/v1.json`,
`schema/public-v1.json`, and
`rust/generated/frontend/foundation-api.ts`.
The .NET reference documents in `src/schema/` remain separate. CI verifies
freshness without regenerating first, so stale committed specs fail the check.

From `rust/`:

```sh
SQLX_OFFLINE=true cargo run --locked -p xtask -- openapi
SQLX_OFFLINE=true cargo run --locked -p xtask -- openapi --check
SQLX_OFFLINE=true cargo run --locked -p xtask -- parity
```

These export full/public documents and the operation catalog, verify artifact
freshness and references, and compare .NET operation IDs, methods, paths and
public exposure. The frontend check compares routes, parameter names and
requiredness, and error status coverage. It does not claim field-level parity:
the former check only compared matching `$ref` names and missed field drift.
Serialization and HTTP tests cover request/response behavior.

With `EnableSwagger=true`, the running API serves the same Utoipa documents at
`/openapi/v1.json` and `/openapi/public/v1.json`. It no longer embeds potentially
stale generated JSON files. The existing setting still defaults to disabled. The existing frontend client is kept
unchanged; its separate generation command is not part of this migration.

## Compatibility boundary

`compatibility.json` preserves wire schemas for endpoints that have not yet
switched their response annotations to Rust DTOs, including dynamically
assembled JSON and Docker payloads. It is a migration snapshot, not a second
catalog to extend for new endpoints. Utoipa-derived components take precedence,
and the exporter removes components not reachable from the document's paths.
An empty schema is represented by an equivalent union of JSON types, and
`const` discriminators by single-value enums, for Utoipa deserialization.

When converting an existing endpoint, reference its actual DTO in `request_body`
or `responses(... body = ...)`, derive `ToSchema` on that type and its nested
DTOs, and remove its obsolete compatibility entries. Use distinct schema names
for different DTOs that happen to share a Rust short name. Keep a compatibility
schema when a handler still builds a response using arbitrary JSON; a bare
`Value` cannot infer those fields.
