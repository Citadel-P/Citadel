# Phase 0C effective realtime configuration

The Phase 0 realtime route is disabled unless its Actor ID, Platform ID, and
token are all configured. The token is reduced to a SHA-256 comparison value at
startup and is never emitted by effective-configuration or metrics output.

| Setting | Default | Source |
|---|---:|---|
| Actor identity | Required; no default | `CITADEL_RUST_REALTIME_ACTOR_ID` |
| Local Platform identity | Required; no default | `CITADEL_RUST_REALTIME_PLATFORM_ID` |
| Prototype bearer token | Required, at least 32 characters; redacted | `CITADEL_RUST_REALTIME_TOKEN` |
| Shared event-ring capacity | `64` | `CITADEL_RUST_REALTIME_QUEUE_CAPACITY` |
| Concurrent connection limit | `8` | `CITADEL_RUST_REALTIME_MAX_CONNECTIONS` |
| Initial subscription deadline | `5 seconds` | `CITADEL_RUST_REALTIME_SUBSCRIBE_TIMEOUT_SECONDS` |
| Client write deadline | `2 seconds` | `CITADEL_RUST_REALTIME_SEND_TIMEOUT_SECONDS` |
| Authorization recheck interval | `30 seconds` | `CITADEL_RUST_REALTIME_AUTH_RECHECK_SECONDS` |
| Snapshot container limit | `1,024` | `CITADEL_RUST_REALTIME_SNAPSHOT_LIMIT` |
| Client message limit | `16 KiB` | Phase 0C protocol constant |
| WebSocket write-buffer limit | `128 KiB` | Phase 0C transport constant |
| Protocol version | `1` | Phase 0C envelope contract |
| Payload schema version | `1` | Phase 0C Platform snapshot/runtime payload contract |

The queue is a shared bounded Tokio broadcast ring. A lagging receiver does not
retain overwritten events: the server emits `resyncRequired`, rechecks the
Actor's Platform access, and follows it with an authoritative bounded snapshot.
Every reconnect also begins with authorization and a snapshot. Connection
sequence numbers are scoped to a generated connection ID; resource revisions
remain monotonic for the lifetime of the server process.

The configured token is an isolated Phase 0 authentication fixture, not the
final Citadel session or service-account design. Production authentication is
owned by the identity/access migration phase. The route is not mounted when the
fixture is absent.

The short production-shaped verification overrides the event-ring and
connection capacities to `4` and the authorization recheck interval to `2
seconds` so overflow, reconnect, and revocation paths are exercised quickly.
