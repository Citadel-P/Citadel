# OpenAPI ownership

`#[utoipa::path]` on each handler owns method, path, operation ID, parameters,
responses and exposure metadata. Resource `documented_routes()` factories use
`utoipa_axum::routes!` for both the live router and offline exporter. Generation
requires no database or Docker connection.

Request/response DTOs derive `ToSchema` in Server, never feature crates. For directly
serialized feature values, `api/resources/schema_models` supplies server-owned
schemas referenced with `value_type` annotations. Native field types, exhaustive
conversions and serialization preserve the wire contract. Use explicit schema
attributes for custom serialization; PATCH wrappers expose optional nullable
values, not their internal missing/null/value enum.

Reusable Problem Details responses live in `errors.rs`. The router adapter
normalizes Axum capture names while retaining documented parameter names to avoid
conflicting equivalent paths.

| Metadata | Consumer |
| --- | --- |
| `x-citadel-public` | Public-document operation selection |
| `x-citadel-setup-exempt` | Setup middleware |
| `x-citadel-principal` | Required principal category; middleware/handlers still authorize |

## Export and verification

Follow [API generation](../../../../docs/DEVELOPMENT.md#api-changes) for commands,
outputs and sequential frontend verification. Plain `cargo build` does not export
contracts. CI checks committed artifacts without regenerating first.

`openapi --check` is read-only and needs no Node.js. Frontend `api:verify` checks
generated freshness, routes, parameters, requiredness, tags, media types and errors;
it does not establish field-level runtime parity. Serialization and HTTP tests
cover request/response behavior.

## Native contracts

Every endpoint references native Rust request/response types. The exporter rejects
unresolved references and prunes unreachable components. Use distinct schema names
for unrelated types sharing a Rust short name.

HTTP/realtime share resource DTOs and vocabulary conversions. Preserve
`MetadataPatch` missing/null/value semantics, collection envelopes, serialized
progress items and intentionally extensible JSON. Webhook signature verification
consumes the unmodified body. Platform queries retain repeated tags,
case-insensitive keys and duplicate rules; ordinary `Query<T>` is insufficient.

Activity schemas derive from `ActivityEventInfo` and snapshots, applying the same
public casing as presentation while preserving discriminators/dictionary keys and
stored formats. Statistics ranges come from `StatsWindow::HOURS`, shared with
runtime validation.

## Operation metadata and Swagger UI

With `CITADEL_HTTP_OPENAPI_ENABLED=true` (disabled by default), Core serves
`/openapi/v1.json`, `/openapi/public/v1.json` and embedded `/swagger/` assets without
a CDN. Router construction initializes the document cache before listeners open.
JSON endpoints add the validated request origin and return `no-store`; static
exports do not depend on the build machine's hostname.

Handler annotations own resource tags, cookie parameters, actual `Set-Cookie`
headers and named examples. Examples use Rust wire casing and are checked against
DTOs/validation. Bearer documentation covers User JWTs and Service Account tokens;
429 responses use `application/problem+json`.

Merge-patch endpoints advertise `application/merge-patch+json` and
`application/json`, with merge-patch selected first in Swagger UI. Command-style
PATCH endpoints retain their JSON contract. Swagger supports operation IDs, deep
links, request durations, Try it out and persisted authorization.
