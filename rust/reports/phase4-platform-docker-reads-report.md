# Phase 4 platform and Docker reads report

Date: 2026-08-17

## Outcome

The five Phase 4 read slices are implemented for Local Docker and the active
.NET Agent protocol: authorized Platform projections, Docker inventory,
event-driven observation with bounded periodic repair, Swarm inventory reads,
and transport/stability behavior. The Rust server exposes the existing Citadel
HTTP contracts and generates the matching OpenAPI/frontend metadata from the
same typed route catalog.

The implementation does not pretend that Edge Agent is Local or Agent. An Edge
Platform without a live Rust Edge command-session adapter returns the declared
409 capability response. The inbound Edge session is deliberately retained as
an explicit migration gate because implementing only its read half would weaken
the enrollment, binding, routing, and reconnect security boundary.

## Implemented slices

### 4A — Platform reads

- `citadel-platforms` owns the Platform, Container, Image, Network, Volume, and
  Swarm read models and the authorization-independent read service.
- SQLx adapters load authorized Platform lists/details, tags, capabilities, and
  persisted inventory without N+1 per-resource queries.
- All routes authenticate first, enforce Platform/resource ACLs, validate path
  and query input, return Citadel Problem Details, and declare those responses
  in OpenAPI.
- Network and Volume details use real runtime inspect operations. A requested
  Swarm node is never silently ignored; unsupported node routing returns 409.

### 4B — Docker inventory

- Citadel's generator owns the required Docker Engine API 1.49 models and
  operations. The runtime uses the handwritten Unix-socket transport and does
  not depend on Bollard.
- Local and Agent adapters normalize Platform info, Containers, Images,
  Networks, Volumes, ports, IPAM, connected peers/containers, labels, and
  ownership metadata into the same bounded application types.
- One transactional projection replacement persists a complete observation.
  Missing rows are removed only after the replacement writes succeed.
- Image repository extraction preserves registry ports such as
  `localhost:5000/team/api:latest`.

### 4C — Realtime observation

- Local Docker and Agent daemon-event streams feed independent one-item
  coalescing triggers. Bursts request one authoritative refresh and one source
  cannot hide a refresh requested by the other.
- A skipped-tick 30-minute reconciliation remains the bounded repair path for
  missed events and reconnects.
- Container statistics are streamed, persisted independently of notification,
  and retained for seven days. Realtime payloads are not allocated or
  serialized when there are no subscribers.
- The versioned WebSocket authenticates ordinary Citadel User or Service
  Account bearer tokens, authorizes the requested Platform from persisted ACLs,
  sends an authoritative bounded snapshot, filters events by Platform, reports
  lag as a resync condition, and periodically rechecks authorization.

### 4D — Swarm reads

- Nodes, Services, Tasks, Networks, Configs, and Secrets have bounded list and
  detail routes backed by persisted projections.
- Task service filtering and limits execute in PostgreSQL rather than loading
  and filtering an unbounded collection in memory.
- Service task counts are recomputed from the current task replacement so a
  disappeared task cannot leave a stale healthy count.
- Citadel ownership labels are normalized conservatively. Missing or
  conflicting ownership is not reported as managed.

### 4E — Transport and stability parity

- Local and Agent share the same inventory/stats ports, normalized errors,
  cancellation behavior, deadlines, bounded streams, and projection store.
- Agent requests retain the current signed nonce/timestamp protocol and retry
  only transient transport failures with a fresh signature.
- Reconciliation distinguishes Standalone and Swarm Platforms and does not call
  Swarm endpoints for Standalone Docker.
- Queues, snapshots, request payloads, task results, realtime clients, and
  retained statistics all have explicit bounds.

## Test mapping from the .NET reference

| .NET reference area | Rust proof |
| --- | --- |
| `GetContainerTests` and Platform/Network/Volume endpoint tests | `server/tests/platforms_http.rs` exercises the real Axum routes, PostgreSQL projections, ACLs, capabilities, detail inspect, invalid filters, 401/403/404/409 behavior, and realtime Platform authorization. |
| `ContainerSyncJobTests`, `ImageSyncJobTests`, and `PlatformSyncJobTests` | `adapters/tests/platform_inventory_persistence.rs`, Docker mapping tests, and worker unit tests prove transactional replacement, removal, restart persistence, ownership, Standalone/Swarm dispatch, and failed-refresh preservation. Mutation- and alert-specific scenarios remain with their later owning phases. |
| `ContainerStatsWriterJobTests`, `ContainerStatsStreamerJobTests`, and `ContainerStatsMemoryTests` | Container-stat store tests, Agent transport tests, realtime subscription tests, and no-subscriber tests cover bounded persistence, retention, cancellation, streaming, lag/resync, and skipped notification allocation. |
| `SwarmEndpointTests` and `SwarmPersistenceTests` | The real Platform HTTP and inventory persistence suites cover all six Swarm collections, details, task filtering, transactional replacement, ownership, counts, and restart reads. Mutation, rollout, and adoption scenarios remain Phase 6. |
| `SwarmReconciliationTests` | Worker tests verify no Swarm calls for Standalone, exactly one bounded fetch of each Swarm inventory set, event coalescing, and repair reconciliation. Managed rollout recovery remains Phase 6. |
| Agent transport tests | `adapters/tests/agent_transport.rs` runs a real tonic fixture for retry, cancellation, signed daemon events, and normalized inventory/stat streams. |
| Edge Agent tests | Stable disconnected/capability behavior is covered now. Enrollment, binding, inbound session routing, reconnect, and full cross-language transport equivalence remain the explicit Edge migration gate. |

## Verification

The following pass in the pinned Linux Rust container:

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- real PostgreSQL projection and Axum Platform suites
- real tonic Agent transport suite
- `cargo xtask openapi --check`
- `cargo xtask docker --check`
- `cargo xtask database verify`

## Remaining exit evidence

Phase 4 behavior is implemented, but its final exit statement still requires a
repeatable cgroup soak/differential run and the active Edge Agent command-session
adapter. Those are measurable gates, not hidden TODO behavior. Phase 5 can be
developed without treating either gate as complete.
