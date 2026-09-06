# Runtime statistics parity — 2026-09-05

Status: history slice implemented; Phase 7 is **not complete**.

## Implemented

The existing frontend now has backend history routes for Containers, Platforms,
Deployments, Stacks, Swarm Services, and Tasks. Operation IDs, paths, response
properties, 24/48/72-hour selection, and 60/300-second buckets follow .NET.
No existing React frontend source files or database migrations changed; generated
Rust contract artifacts were refreshed.

- Container history accepts a projection UUID or an unambiguous Docker ID prefix.
  A prefix matching more than one projection is rejected, not arbitrarily routed.
- Platform Read protects direct Container/Platform/Swarm history. Deployment and
  Stack history require their own resource Read permission; that grant does not
  grant access to direct Platform history endpoints.
- Task history checks the live Task through Local or signed regular-Agent
  transport, then resolves its exact platform/node/container projection. Moved,
  stopped, stale, unavailable, or unsynchronized targets fail closed.
- Service samples are attributed and committed in the same transaction as the
  Container samples. The existing `swarmservicestats` table retains managed
  Service/Stack-member identity independently of runtime projection deletion.
  Logical replica-slot averages precede Service sums to avoid double-counting
  task replacement within a bucket. Historical samples from before this change
  are not backfilled.
- Existing connected-manager projections keep their stable Container UUIDs.
  When their node column is absent, Local/regular-Agent history resolution uses
  the manager Node ID reported by Docker and refreshed on every inventory commit.
  Missing identity clears the previous value; names and labels are not used to
  infer a node. Explicit node identities never fall back to a different node.
- Coverage selects only current, non-stale, desired-running Tasks; latest raw
  sample timestamps determine freshness. Freshness allows three configured
  monitoring intervals, with a 30-second minimum. Projection IDs are returned
  even before the first sample so the existing frontend can subscribe.
- Retention deletes at most 5,000 expired rows per table per write transaction.
  Stack histories have a 500-Container and 65,536-sample response bound; requests
  beyond the bound fail explicitly rather than returning incomplete history.
  Responses serialize typed samples directly, without a second JSON value tree.
  These are code-level bounds, not a measured RSS guarantee.

The Docker TaskInspect operation was added through `xtask docker` generation.
OpenAPI metadata is shared with route registration and regenerated normally.
No handwritten generated SQL or compatibility migration was introduced.

## .NET reference and test mapping

| .NET reference | Rust coverage |
| --- | --- |
| `PlatformsStatsWriterJobTests.GetStatsAggregatedAsync_ShouldRespectRequestedWindow` | `history_endpoints_authorize_validate_windows_and_read_persisted_samples` exercises HTTP + PostgreSQL windows, auth failures, validation and chronological results |
| `ContainerStatsWriterJobTests.GetStatsAggregatedAsync_ShouldLoadMultipleContainersInOneQuery`; Container/Platform stat repositories | Batched fixed SQL, bucket mapping in `history_buckets_samples_and_resolves_legacy_docker_ids_without_ambiguity`; resource grouping and scoped permissions in `workload_history_requires_its_own_permission_and_preserves_stack_grouping` |
| `ContainerStatsWriterJobTests.GetStatsAggregatedAsync_ShouldKeepManagedHistoryAcrossTaskReplacementWithoutDoubleCountingSlot` | `service_history_survives_task_replacement_without_double_counting_or_crossing_nodes` covers retries, slot aggregation, wrong-node exclusion, deleted projections, managed Service replacement and Stack-member history |
| `GetSwarmServiceStatsHandler` current-task/freshness behavior | `service_stats_reports_missing_stale_and_fresh_node_coverage` through HTTP + PostgreSQL |
| `SwarmTaskRuntimeQuery.LoadRunningTargetAsync` / `GetSwarmTaskStatsHandler` | `task_history_checks_live_docker_identity_and_node_before_returning_samples` uses a Docker Unix-socket fixture + PostgreSQL; unit tests cover stopped/stale/moved/empty runtime identities |
| Connector cancellation requirements | Local and regular-Agent transport tests cancel before a Task inspection request is issued |
| Connected-manager node routing | `connected_manager_history_uses_reported_node_identity_without_replacing_container_ids` exercises inventory collection, sample persistence and both history endpoints without manually populating the node column |
| Statistics persistence failure and bounded retention | `statistics_retention_is_batched_and_failed_writes_roll_back_all_sample_tables` checks rollback and incremental expiry against PostgreSQL |

The original .NET tests were inspected, not modified. These are selected behavior
ports, not a claim that the whole unit/integration/acceptance suite is ported.
The socket fixtures are integration tests, not real multi-node Docker acceptance.

## Verification

Executed in the Linux devcontainer against the disposable database
`citadel_phase7_completion_20260905`, never the application database:

```sh
cargo test -p citadel-server --test platforms_http -- --ignored
cargo test -p citadel-platforms -p citadel-adapters -p citadel-server --lib
cargo test -p citadel-adapters --test docker_transport --test agent_transport
cargo test -p xtask
cargo clippy -p citadel-platforms -p citadel-adapters -p citadel-server --lib -- -D warnings
```

Results: 18 Platform HTTP/PostgreSQL tests passed (including eight new history
scenarios); 126 selected library unit tests, seven Local/Agent transport tests,
seven generator tests, and the existing PostgreSQL inventory/store-recreation
integration test passed. Targeted library Clippy, workspace formatting,
Docker generation checks and frontend HTTP-contract subset checks passed.
Full frontend tests were not rerun in this backend-only slice; the generated
contract subset test checks compatibility with the existing frontend schema.
The disposable test database was removed after verification. The application
database and existing Docker workloads were not reset or mutated.

## Remaining work found by comparison with .NET

1. **Edge enrollment and inbound session lifecycle are absent**, not merely a
   missing executor switch. Platform creation still explicitly rejects Edge.
   Port enrollment/revocation, signed challenge authentication, bounded pending
   commands, session replacement, disconnect cleanup and exact-node identity
   tests before enabling Edge Build/Backup execution.
2. Multi-node backup execution still supports only the configured regular Agent,
   with live Node-ID verification. The inbound node-session registry, node
   onboarding and interrupted-operation/restore matrix remain open.
3. Live log/terminal execution methods, specific-permission tests, incremental
   `BuildRunLogsAppended`, and selected-recipient `AlertEventReceived` remain
   unimplemented. History routes do not complete those realtime features.
4. Git/image update alert producers still need their authoritative check/update
   execution paths connected; the existing alert persistence pipeline alone
   does not provide these product events.
5. Real Docker/registry/Deno/Shoutrrr/Restic acceptance across Local, Agent and
   Edge has not been implemented or executed by this slice. Phase 7's exit gate
   and Phase 8 candidate/soak work remain open.
