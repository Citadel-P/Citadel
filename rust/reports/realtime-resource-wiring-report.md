# Realtime migration: preserve the existing frontend contract

Updated: 2026-09-05. This replaces the earlier event-to-query invalidation approach,
not a claim that all .NET runtime streaming has been ported.

## Implementation

Removed the frontend `realtime-queries.ts` policy, extra Platform statistics
subscription manager, and Rust-specific feature-hook branches. The migration
boundary is now `createWebSocketConnection.ts` plus provider transport selection.
The existing provider owns group reference counts, token replacement, reconnect,
and disposal for both transports. Existing feature hooks remain unchanged.

Rust's `realtime_groups` reads authorized application/store projections and emits
existing named events rather than browser instructions to refetch queries:

| Subscription | Existing event contract |
| --- | --- |
| Platforms | PlatformUpdated, PlatformsDeleted, PlatformStatsUpdated |
| Containers / Images | ContainersInfoUpdated, ContainersStatsUpdated, ImagesInfoUpdated |
| Docker daemon | Container/Image/Volume/Network events and SwarmInventoryUpdated with nested items wrappers |
| Deployment / Stack / managed Service | Corresponding InfoUpdated view and action arguments |
| Container / Stack summaries | ReceiveContainerInfo, ReceiveStackContainersInfo |
| Git / Automation / Builds / Backups | Named resource and run-view events |
| Resource Activities | Typed, resource-authorized ActivityEventReceived |
| Alerts | Read-authorized AlertEventsUpdated and UnresolvedAlertCount |
| License | LicenseStateChanged |

No frontend resource catalog, database schema change, or new binary is required.
Existing mutation and committed-worker notifications remain the trigger. Existing
reconnect recovery remains; no per-event browser-wide refetch policy is introduced.
Volume/Network cached-remount fixes in the working tree are independent fixes,
not requirements of the Rust transport.

Joins and delivery check current identity/resource permissions. Duplicate joins
are idempotent but reauthorize. Service groups require parent Platform visibility
too. Administrative activity groups explicitly reject non-administrators.
Private/untyped groups and unimplemented log/terminal methods fail closed.

The hub has bounded queues and connections. Each connection permits at most 64
groups, with snapshot-limited retained IDs/routing tombstones, not full resource
configuration or secret payloads. Messages are limited to 4 MiB and writes/reads
have deadlines. Overflow closes the connection for authoritative reconnect/rejoin.
Samples do not trigger unrelated daemon, image, Activity, or Service-list reads.
The adapter bounds pending invocations to 128 with 15-second deadlines and cancels
them on disposal. Obsolete sockets cannot deliver events.

## .NET test mapping

.NET paths are relative to `test/`.

| Source scenario | Rust equivalent / scope |
| --- | --- |
| Unit Application/Services/SignalR/BaseStreamManagerTests: subscription cleanup | Group snapshot/tombstone unit tests; real socket duplicate join, leave, disconnect/shutdown; existing bounded/no-subscriber hub behavior |
| Integration WebApi/Hubs/ApplicationHubTests: join/leave/disconnect | platforms_http/realtime_groups.rs::group_wire_acceptance_joins_once_delivers_committed_rows_and_leaves_without_refetch |
| ApplicationHubAuthorizationTests: permission matrix and private/untyped groups | dotnet_group_permission_matrix_uses_current_database_permissions and group-parser unit test |
| Same suite: delivery only to an authorized recipient | Real TCP test with two DB-authenticated users, denied join, ACL grant, committed update, ACL revocation, duplicate-join denial, closure before another sample |
| Same suite: Service and parent Platform visibility | managed_service_group_requires_parent_platform_and_preserves_persisted_spec |
| SignalRDeploymentSerializationTests: external image discriminator and applied digest | dotnet_deployment_and_service_discriminators_survive_named_event_serialization plus persisted Service spec/read-back test |
| Container/Platform statistics persistence behavior | realtime_subscription.rs PostgreSQL-to-WebSocket test; group wire test checks Docker ID to Citadel projection ID mapping |

Rust preserves JSON `$type`, not MessagePack's union encoding or the .NET stream
manager class hierarchy. The TCP/PostgreSQL scenario is boundary acceptance
coverage, not packaged multi-node release acceptance. No direct HubConnection,
JoinGroup, or named-stat-event test was found in the .NET acceptance suite; do not
claim the whole acceptance suite was ported from this evidence.

Frontend tests run unchanged Deployment, Volume, Network, and Platform hooks via
the adapter, verifying updates without refetch. Transport tests cover arguments,
discriminators, invocation correlation, authorization rejection, fresh-token
reconnect, stale sockets, disposal, and bounded outstanding requests. Existing
.NET-provider tests remain enabled.

## Explicit remaining parity gates

- Live logs and terminal/exec methods and their specific-permission tests. Rust
  rejects these methods; an adapter cannot implement missing backend execution.
- Historical Container/Service/Task statistics endpoints and complete node-agent
  coverage. Live delivery alone does not complete dependent screens.
- Incremental Build log tailing (BuildRunLogsAppended), beyond lifecycle snapshots.
- Selected-user alert push (AlertEventReceived) and its .NET recipient-isolation
  scenario. Current alert snapshots obey HTTP read authorization, not selected
  recipient notification semantics.
- Remaining activity subtype serialization, full permission matrix, and packaged
  Agent/Edge/multi-node acceptance cases. Representative tests are not full parity.

## Verification

Linux devcontainer, 2026-09-05:

- Full frontend unit suite: **368 passed in 105 files**.
- TypeScript and targeted ESLint: passed.
- Rust group unit tests: **5 passed**.
- PostgreSQL/group-wire tests: **3 passed**, none ignored.
- Existing realtime tests, including PostgreSQL persistence: **9 passed**, none ignored.
- Rust formatting and targeted server/library/binary/test Clippy: passed.
- All-target Clippy additionally reports an unrelated existing large_enum_variant
  warning in server/tests/platform_creation_http.rs::InfoOutcome. No suppression
  or production optimization was added to hide it.

Database tests used CITADEL_PHASE4_DATABASE_URL with a disposable PostgreSQL
database and --include-ignored. No main application database reset, Docker image
build, staging, or commit is performed.
