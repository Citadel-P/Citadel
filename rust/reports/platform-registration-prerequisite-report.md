# Platform registration prerequisite

Status: accepted on 2026-09-04 for Local Docker and for the single configured
signed Agent transport.

## Outcome

The existing React Platform form can now call the Rust Core's compatible
`POST /api/v1/platforms` operation. Registration:

1. authenticates the Actor and requires global Platform Write permission;
2. validates the request before opening a Docker connection;
3. forces Local Docker to `http://localhost.docker`;
4. discovers Docker identity and a bounded initial inventory;
5. rejects a Standalone/Swarm type mismatch, a Swarm worker, a missing Swarm
   cluster identity, and duplicate name, address, daemon, or cluster identity;
6. atomically persists the Platform, initial Image and Container projections,
   tags, and typed `PlatformCreated` Activity;
7. returns the normal authorized `PlatformView` and publishes a metadata-free
   realtime invalidation.

A cheap name/address preflight avoids unnecessary remote work. The definitive
duplicate check runs again under a PostgreSQL transaction advisory lock, so
concurrent registrations cannot create two Platform rows for one Docker
daemon or Swarm cluster. No retry is performed after an ambiguous runtime
failure.

The normal Platform workers discover the committed row on their next bounded
iteration; registration does not create an additional per-Platform task.

## Contract and UI compatibility

- Operation ID, method, path, Actor authentication, request schema, and
  response schema match the generated .NET frontend contract.
- Platform and connector enums retain the existing JSON values, including the
  unsupported `Kubernetes` and `Unknown` values so they receive a normal 400
  validation response. The implemented Rust endpoint currently returns an
  explicit unavailable error for Edge enrollment instead of partially
  creating an unusable Platform.
- The current Rust Agent router supports the one Agent address configured for
  Core. Arbitrary per-Platform Agent dialing and inbound Edge enrollment remain
  migration exit work.

## Test mapping

| Existing .NET scenario | Rust evidence |
| --- | --- |
| Local create persists Platform, Images, Containers, and Activity | `platform_creation_http::create_platform_enforces_authorization_and_atomically_persists_initial_inventory` |
| Agent create | `platform_creation_http::agent_platform_creation_persists_tags_transport_and_realtime_change`; signed transport remains covered by `agent_transport` |
| duplicate name/address | PostgreSQL-backed HTTP tests; also prove a known preflight conflict does not contact Docker or Agent |
| Swarm manager persists cluster identity and current inventory | `platform_creation_http::swarm_manager_creation_persists_cluster_inventory_and_prunes_historical_tasks` |
| Standalone rejects active Swarm | HTTP parity test plus `registration::tests::validates_type_against_the_discovered_daemon` |
| Swarm rejects non-member/worker/missing cluster ID | HTTP parity test plus `registration::tests::swarm_registration_requires_an_active_manager_with_cluster_identity` |
| duplicate Docker daemon/Swarm cluster | PostgreSQL-backed Local and Agent daemon tests, cluster test, and simultaneous-create serialization test |
| missing daemon ID | HTTP parity test plus `registration::tests::missing_daemon_identity_is_a_remote_runtime_failure` |
| invalid payload | HTTP persistence test and typed request validation |
| forbidden/unauthorized | HTTP persistence test; both cases prove zero runtime calls |
| Agent unauthenticated failure | HTTP parity test preserves 401 and the Agent detail; signed transport normalization has separate coverage |
| unsupported Kubernetes type | HTTP parity test plus `registration::tests::unsupported_platform_types_are_validation_failures` |

Additional Rust coverage proves valid Tag persistence, invalid-Tag rejection,
full transaction rollback after a late persistence failure, realtime
publication, pruning enabled/disabled semantics, bounded inventory call counts,
and exactly one winner for simultaneous duplicate registrations.

The disposable Linux gate is `./rust/scripts/Test-Phase4Reads.ps1`. It covers
the new PostgreSQL-backed Axum test alongside existing Docker, Agent,
projection, Platform read, and realtime regression tests.

## Remaining boundaries

- Edge Agent Platform enrollment/session routing is not implemented in Rust.
- Multiple independently addressed regular Agents are not yet resolved by a
  per-Platform connector registry.
- A differential registration test against a real Docker Engine should be
  added before the .NET Platform write path is removed; transport and
  concurrency behavior are otherwise covered independently.
