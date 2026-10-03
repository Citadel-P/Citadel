# OpenAPI ownership

Citadel uses Utoipa 5 and utoipa-axum. `#[utoipa::path]` on each HTTP handler
owns its method, path, operation ID, parameters, responses and exposure metadata.
A resource's `documented_routes()` factory registers these handlers with
`utoipa_axum::routes!`. Both the live Axum router and the offline exporter use
that factory. Documentation generation needs no database or Docker connection.

Request and response DTOs derive `utoipa::ToSchema` only in Server. Feature crates
must not depend on Utoipa or carry schema attributes. For directly serialized feature
values, `api/resources/schema_models` provides server-owned descriptions referenced
with `#[schema(value_type = ...)]` or route body annotations. Their native field
types and exhaustive conversions detect structural drift; enum values come from
native serialization. Runtime handlers continue using the feature values directly.
Serde field names, optional fields, defaults, skipped fields and tagged enums
are reflected in the document. Use explicit `#[schema(...)]` attributes for
custom serializers/deserializers. In particular, PATCH wrappers describe an
optional nullable value, not their internal missing/null/value enum.

Reusable Problem Details responses live in `errors.rs`. The small router
adapter normalizes Axum capture names while retaining documented parameter names;
without it, equivalent `{id}` and `{resourceId}` paths conflict in Axum.

`x-citadel-public` selects operations in the public document.
`x-citadel-setup-exempt` is also consumed by setup middleware.
`x-citadel-principal` records the required principal category; authorization
continues to run in the existing middleware and handlers.

## Export and verification

The **Citadel: Build Rust API** task compiles the backend and exports its specs.
Both **Ctrl+Shift+B** and **F5 → Citadel: Debug application** run this step
before starting the API. From a terminal, use `bash src/tools/build/build.sh`
from the repository root. Plain `cargo build` only compiles; it does not export.

To export separately in VS Code, use **Terminal → Run Task → Citadel: Generate OpenAPI spec**.
Use **Citadel: Verify OpenAPI spec** to check freshness without writing files.
The tasks set the Rust workspace directory and `SQLX_OFFLINE=true` automatically.
Generated files are `schema/v1.json`,
`schema/public-v1.json`, and
`src/frontend/src/api/generated/foundation-api.ts`. The exporter also copies the full
`schema/v1.json` to the frontend `src/api/schema/swagger.json` and runs
`npm run api:generate` there. Install frontend dependencies with `npm ci` first.
The frontend needs the full document, including internal authentication and setup
operations. Its generation script always copies the Rust document before producing
the TypeScript client and resource map.
CI verifies freshness without regenerating first, so stale committed specs fail the check.

From the repository root:

```sh
SQLX_OFFLINE=true cargo run --locked -p xtask -- openapi
SQLX_OFFLINE=true cargo run --locked -p xtask -- openapi --check
```

These export full/public documents and the operation catalog, verify artifact
freshness, references and operation metadata. The frontend check compares routes, parameter names and
requiredness, and error status coverage. It does not claim field-level parity:
the former check only compared matching `$ref` names and missed field drift.
Serialization and HTTP tests cover request/response behavior.

With `EnableSwagger=true`, the running API serves the same Utoipa documents at
`/openapi/v1.json` and `/openapi/public/v1.json`. It no longer embeds potentially
stale generated JSON files. The existing setting still defaults to disabled. The frontend uses this same full contract. `openapi --check` remains read-only and
does not require Node.js; `npm run api:verify` checks generated frontend freshness.

## Native contracts

Every endpoint references its native Rust request and response types. There is no
frozen schema snapshot or fallback schema registry. The exporter rejects unresolved
references and removes components unreachable from documented operations. When adding
an endpoint, derive `ToSchema` on its server DTO and nested server descriptions; use distinct schema
names when different Rust modules contain unrelated types with the same short name.

HTTP and realtime projections share resource DTOs and checked vocabulary conversions.
PATCH DTOs preserve missing/null/value semantics with `MetadataPatch`; existing feature
validation and authorization remain authoritative. Collection responses document their
actual arrays and envelopes. Incremental JSON progress streams document their serialized
item arrays, and raw webhook bodies remain bytes for signature verification. Deliberately
extensible Docker and worker payloads retain their JSON contracts.

Activity schemas derive from the feature's closed `ActivityEventInfo` enum and its
snapshot types. The public schema applies the same property-casing projection as the
HTTP/realtime presentation layer; persisted activity records keep their storage format.
Discriminators and user dictionary keys are preserved. Statistics query values come from
`StatsWindow::HOURS`, the same list used by runtime validation.

Platform query parsing intentionally preserves repeated tags, case-insensitive keys,
and duplicate rules; ordinary `Query<T>` does not implement those semantics. Shared
webhook configuration uses `citadel_resources::RepoWebhookConfig`; signature verification
continues to consume the unmodified request body.

## Operation metadata and Swagger UI

Cookie parameters, actual `Set-Cookie` response headers and named registry examples
live in the handler annotations. Shared examples use Rust wire casing and are tested
against the request DTOs and registry validation. The contract gate checks cookie parameters, example names for each media type
and response headers.
Bearer documentation describes both User JWTs and Service Account tokens; shared
error responses document 429 using `application/problem+json`.

With `EnableSwagger=true`, `/swagger/` serves embedded Swagger UI assets, the custom
operation-ID stylesheet, deep links, operation IDs, request durations, enabled
Try it out and persisted authorization. No CDN is needed. Both JSON endpoints add
the request origin after transport Host/proxy validation and return `no-store`;
static exports remain independent of the build machine's hostname. The existing
transport mode supplies the HTTP/HTTPS fallback for direct connections.

Endpoint tags identify resource groups and are declared on each handler.
The frontend compatibility gate checks tags and exact PATCH request media types.
Merge-patch operations advertise both `application/merge-patch+json` and
`application/json`; command-style PATCH operations retain their JSON contract.
The Swagger UI content-type component puts merge-patch first when advertised,
so it is the initial selection without removing the JSON option.
