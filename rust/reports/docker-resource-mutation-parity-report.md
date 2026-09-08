# Docker resource mutation compatibility fixes

## Scope

Closes the three UI-reported Rust API gaps without changing frontend source or
the database schema:

- `GET /api/v1/containers/{id}/adoption-draft` and
  `POST /api/v1/containers/{id}/adopt` now support the existing adoption form.
- `DELETE /api/v1/images` accepts the existing platform-scoped Docker image IDs.
- `POST /api/v1/networks` accepts optional IPAM fields, including the form's
  empty placeholder rows, and preserves `enableIPv4` / `enableIPv6`.

## Reference behavior

The implementation follows the .NET `GetContainerAdoptionDraft`,
`AdoptContainer`, `DeleteImages`, and Docker `NetworkService` paths.

Adoption inspects Docker but does not create, restart, or recreate a container.
It checks permissions, ownership, representable settings, and image repository
compatibility. A signed preview fingerprint rejects stale drafts. Deployment
creation, container linking, encrypted sensitive-value bindings, the adoption
activity, and unmanaged-alert resolution commit in one database transaction.
Concurrent adoption attempts cannot create two Deployments for one container.

Image deletion uses the existing Local, Agent, and Edge operations, bounded
concurrency and timeouts, and no automatic destructive retries. Fresh inventory
reconciles partial failures; successfully deleted images do not remain persisted
and failed images do not stay marked Processing. As in .NET, this platform-level
endpoint rejects Swarm image deletion without an explicit Node target.

Empty IPAM entries are omitted from the Docker request. Nonempty optional fields
are preserved. Neither a frontend workaround nor relaxed JSON parsing is needed.

## Regression coverage

- Adoption mapping: Docker defaults, unsupported settings, sensitive-value
  redaction, global binding references, and Agent/Edge port/network shapes.
- PostgreSQL-backed adoption HTTP tests: authorization, protected containers,
  Local and External image selection, stale previews, invalid tags, atomic
  concurrent adoption, encrypted storage, and the .NET activity payload.
- Image deletion HTTP test: authorization, missing IDs, duplicate IDs, successful
  deletion, partial Docker failure, and persisted inventory/control state.
- Edge transport test: deletion options, result mapping, and no failed-command replay.
- Network HTTP and Docker transport tests: the reported UI payload, exact IP
  version flags, and omission of empty IPAM rows.

HTTP tests use a disposable PostgreSQL database and fake Docker sockets. They
do not exercise a production daemon or a live Agent installation.

Verified on 2026-09-07: 42 platform HTTP integration tests, 5 adoption unit
tests, 10 Docker transport tests, 12 Edge session tests, and the network request
unit test passed. Server Clippy passed with warnings denied. Generated Docker
and OpenAPI contract checks passed (336 full / 284 public HTTP contracts).

```sh
cargo test -p citadel-deployments --test container_adoption
cargo test -p citadel-adapters --test docker_transport
cargo test -p citadel-adapters --test edge_sessions
cargo test -p citadel-server --lib network_create_accepts_the_existing_ui_ipam_and_ip_version_fields
# CITADEL_PHASE4_DATABASE_URL must point to a fresh, disposable test database.
cargo test -p citadel-server --test platforms_http -- --ignored --test-threads=1
cargo clippy -p citadel-server --tests -- -D warnings
cargo run -p xtask -- docker --check
cargo run -p xtask -- openapi --check
```

Restart the rebuilt Rust backend before testing the three existing UI flows.
No database reset or frontend regeneration is required to use these fixes.
