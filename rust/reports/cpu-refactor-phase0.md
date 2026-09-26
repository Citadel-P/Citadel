# CPU architecture refactor — Phase 0

This is Phase 0 of the September 2026 Architecture & Performance Refactor
Specification. It is separate from the older v13 Phase 5 report. Runtime behavior,
event policy, SQL, scheduling, and concurrency remain unchanged. Phase 1 is not
part of this change.

## Source and responsibility map

Baseline: `4f7cb7b2` (`feat--migrate-to-rust`, including the latest container command
and realtime batching fixes). Both Rust and local .NET `main` were verified against
GitHub's current branch heads. .NET reference: `main` at
`fb7fa7c4db00ebdaf2fd31bc1230102476a76178`; Citadel.Contracts `main` at
`21461a1f774f4b9cce35c1cd3abfd0faf7ccf751`.
[MonitorEventsService](https://github.com/Citadel-P/Citadel.Contracts/blob/21461a1f774f4b9cce35c1cd3abfd0faf7ccf751/src/Citadel.Hosting.DockerClient/Services/MonitorEventsService.cs)
and [ContainerService](https://github.com/Citadel-P/Citadel.Contracts/blob/21461a1f774f4b9cce35c1cd3abfd0faf7ccf751/src/Citadel.Hosting.DockerClient/Services/ContainerService.cs)
were fetched through GitHub. They confirm ignored lifecycle noise, ID-filtered
container reads, Swarm-scoped network signals, and bounded parallel commands.

Paths below are relative to `rust/crates` unless stated otherwise. The map covers
production callers; test callers are listed by suite underneath.

| Entry point | Current callers and cost ownership | Reference responsibility / later phase |
| --- | --- | --- |
| `InventoryEvent` | `server/src/workers/platforms/events.rs` produces Local raw and Direct Agent events, including synthetic reconnects; `platforms/agents.rs` owns Agent subscriptions; `platforms.rs` wires the bounded queue | Shared normalizer, then typed deltas (1–2) |
| `apply_container_event` | `events.rs::event_consumer` invokes it once per queued event. Local non-destroy events inspect the container, including `kill`, `die`, and `stop`. `exec_*`, `attach`, `top` return early. Direct Agent uses its bound platform and supplied observation | .NET `DockerDaemonEventJob` dispatches normalized resource work items (1–2) |
| `queue_inventory_reconciliation` | Event consumer fallback after unhandled events/errors; reconnect bypasses policy. Local capacity-one trigger; bounded Agent scopes upgrade to all Agents on overflow | Resource dirty signals and recovery orchestration (2–4) |
| `triggers_inventory_reconciliation` | Only production caller is the queue helper. Rejects `builder` and `plugin`; network disconnect, unknown kinds and missing payloads can request full inventory | Dedicated Swarm coordinator and scoped recovery (2–4, 12) |
| `collect_inventory` | Platform inventory worker; Edge worker; node-Agent setup; platform HTTP registration/synchronization paths. `collect_inventory_from_info` is also used by feature registration | Calls info, containers, images, networks, volumes sequentially, plus Swarm sets when applicable. Split into resource owners (3–4) |
| Docker `container_observation` | Local event handler maps targeted inspect JSON to `RuntimeContainerSummary` | Targeted container read; independent of full sync |
| PostgreSQL `container_observation` | Local/Direct event handler; metadata upsert then `container_event_in` in one transaction, then deployment reconciliation | .NET created/updated work items; semantic projection writer (5) |
| PostgreSQL `container_event` / `container_event_in` | Event handler for delete/state-only payloads; Edge store calls transactional helper directly and optionally metadata helper | Currently returns matched/updated state, not semantic equality. Do not call `false` a semantic no-op (5) |
| `publish_container_observation` | Local/Direct event handler and Edge worker after successful persistence | Existing 100 ms bounded coalescing must remain. Claim/completion batches bypass its delay (15) |
| `ContainerMutationService` | Router constructs it; composition supplies process task owner/notifier; platform/deployment HTTP handlers, pruning worker and container recovery worker invoke it | .NET processing service: claim → notify → runtime mutation → per-target verify/persist → finish → notify. Four admitted operations, up to 100 selectors; Local mutation loop remains sequential (7) |
| `ContainerStatsStore` | Local/Direct samplers call `persist_stats_retry`; Edge uses its session-owned transaction path | .NET streamer/writer separation; retain current set-based SQL, later batch across ticks (9) |
| Health / recovery | Health probes every 5 s and also reconciles deployments; container stale claims every 10 s; other recovery remains feature-owned | .NET health transition broadcaster and five-minute `ReconcilableResourceJob` (8, 13) |
| Identity / realtime | Snapshot and resource permissions are SQL-backed; realtime rechecks authorization; existing container batches share reads | .NET scope/permission/role caches and after-commit affected-principal eviction (14–15). No cache changes in Phase 0 |

Tests calling these boundaries: `platforms/tests/container_commands.rs`,
`adapters/tests/{platform_inventory_persistence,stack_persistence,stack_runtime_local,edge_transport}.rs`,
`server/src/workers/{runtime_persistence_tests,pruning/tests}.rs`, platform worker
unit tests, and `server/tests/{platforms_http,stacks_http,external_integrations}`.

Additional .NET responsibilities inspected on local main: container identity cache
and mutation-version fencing; bounded DB/notification channels and commit ordering;
sync barrier and tracked tasks; two-second Swarm debounce; stats count/time flush;
update-check/Git/image-scan single-flight; mutation-driven Team/Role/User permission
eviction. These are responsibility references, not a plan to recreate .NET DI or a
global Rust database writer.

## Counter definitions

All additions extend `citadel_runtime_*{family="..."}`. Families are closed enum
variants; resource IDs, paths, actions, addresses and actor IDs are never labels.
`iterations_total` counts started timed operations (including failure/cancellation),
`duration_microseconds_total` includes awaited I/O, and `units_total` has the
explicit meaning below. Nested timers overlap; do not sum them to estimate CPU.

| Family | Meaning |
| --- | --- |
| `DockerRawEvent`, `AgentDaemonEvent` | Units: received Local raw messages / Direct Agent semantic messages, excluding synthetic reconnects |
| `ContainerEventIgnored` | Units: existing `exec_*`, `attach`, `top` early returns. `kill`/`stop` are still processed |
| `ContainerEventInspect` | Iterations: targeted Local event inspection attempts, including mapping |
| `ContainerEventApply` | Iterations: valid-identity container event handling; failures: errors returned to event consumer |
| `ContainerEventProjection` | Iterations: targeted Local/Direct persistence calls; units: calls returning `updated=true`. This is **not** a semantic-change/no-op counter; includes post-commit deployment reconciliation |
| `InventoryRequestContainer`, `InventoryRequestNetwork`, `InventoryRequestOther`, `InventoryRequestReconnect` | Units: event-policy requests before queue coalescing, split by fixed reason |
| `InventoryRequestCoalesced` | Units: Local event request rejected because its trigger is already full; failures: closed trigger. Does not count Agent overflow or debounce-drained requests |
| `InventoryLocalEvent`, `InventoryAgentEvent`, `InventoryRecovery` | Iterations: target passes admitted after the inventory budget. Local/Agent trigger scopes also include health transition requests; Recovery includes startup, target refresh and timer passes |
| `DockerContainerList`, `DockerFilteredContainerList`, `DockerImageList`, `DockerNetworkList`, `DockerVolumeList` | Iterations: shared Docker client list entry points (including stats/API/reconciliation); filtered container lists are separate. Not purpose-specific reconciliation counts; the request proxy is authoritative for total HTTP counts |
| `ContainerMutation` | Iterations: command router batch attempts; units: supplied target count, not successfully mutated count |
| `ContainerVerification` | Iterations: router observation attempts from commands **or** stale-claim recovery, including target resolution |
| `ContainerRecoveryQuery` | Iterations: stale-claim query passes; units: claims returned |
| `HealthDeploymentRecovery` | Iterations: deployment reconciliation performed by health worker |
| `StatsCommit` | Units: committed transactions in the three public container stats store methods. Edge's externally owned transaction path is excluded |
| `StatsCommittedSamples` | Units: container rows inserted/upserted by those committed transactions, not platform samples |
| `StatsRetry` | Failures: failed Local/Direct persistence attempts; units: completed retry delays. Cancellation before the next attempt can leave one scheduled retry unused |
| `RealtimeRuntimeInvalidation` | Units: actual hub `runtimeChanged` publications with subscribers; batching/no-subscriber behavior unchanged |

Existing `StatsPersistence`, resource recovery families, `Inventory`, budget wait /
saturation, event queue depth/lag, realtime messages and reconnect counters remain.
The capture also records PostgreSQL statement-level calls/rows/time to distinguish
stats SQL, committed transactions, stale scans and repeated projection updates.

## Measurement method

`scripts/measure-cpu-refactor.py` creates a fresh database per capture and runs a
release Core against a Unix socket proxy connected exclusively to a disposable
Docker-in-Docker daemon. It registers one Local standalone platform. The fixture
has no Direct/Edge Agent or realtime subscribers. No existing host workloads are
used as command targets.

Defaults: pool 5, event queue 256, monitoring interval 10 s, unchanged production
release profile and Tokio sizing. After setup and a 30-second settle, measure:

1. Idle for 300 seconds.
2. Create/start one container, settle 15 seconds, stop through Citadel's HTTP API;
   observe the command and its events over a 20-second window.
3. Create/start 20 containers, settle 15 seconds, sample stats for 60 seconds.
4. Stop all 20 in one Citadel HTTP request; observe for 30 seconds.
5. Create/delete 20 containers in the nested daemon; observe for at least 30 seconds.
6. Observe idle recovery for another 60 seconds.

CPU uses `/proc/PID/stat` user+system ticks; 100% means one logical CPU. Average
uses window boundary ticks; p95/max use one-second samples wholly inside the
window. Docker counts include all Core requests in the window (including unrelated
health/stats ticks); raw paths preserve query parameters. PostgreSQL statement
deltas include transaction statements, nested statements (`track=all`), and the
benchmark's single post-command verification query, but exclude
`pg_stat_statements` observer queries. Boundary snapshots are asynchronous; small
background-call differences can fall on either side of a window. These are
attempt/statement counts, not successful semantic mutation counts. Short command
windows should not be compared as if they were sustained CPU throughput tests.

The instrumented capture also records per-second RSS/PSS and thread count, final
metric gauges, and RX/TX deltas on `eth0` inside each disposable Docker/PostgreSQL
container. Bridge/veth interfaces are excluded to avoid double-counting packets.
These are fixture network bytes, including protocol overhead and metric-boundary
traffic, not SQL payload bytes. The earlier uninstrumented capture used driver
version 1 and did not record these additional fields; it supplies the CPU and
operation-count comparison. Host disk usage is unavailable because the nested
Docker data filesystem is not exposed under `/host`; this is unchanged in both
runs. Realtime publication counts are zero without subscribers.

The driver stores per-window metrics and SQL deltas, raw request/CPU samples,
binary hash, source SHA/dirty flag, image IDs, machine details, and server logs.
Session tokens, authorization headers, and HTTP bodies are not recorded. Raw
captures live under `/tmp/citadel-cpu-phase0-*`; retain them separately when moving
this report to another machine. Do not compile during accepted captures.

## Reproduce

From the repository root, start dedicated fixtures (these names and ports must be
unused):

```sh
docker run -d --rm --name citadel-cpu-phase0-postgres -p 127.0.0.1:55449:5432 \
  -e POSTGRES_USER=citadel_cpu -e POSTGRES_PASSWORD=citadel_cpu_fixture \
  -e POSTGRES_DB=citadel_cpu \
  postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44 \
  -c shared_preload_libraries=pg_stat_statements -c pg_stat_statements.track=all
docker run -d --rm --privileged --name citadel-cpu-phase0-docker \
  -p 127.0.0.1:52389:2375 -e DOCKER_TLS_CERTDIR= \
  docker@sha256:3ef33f2e220b79ed3ef3b99d81746f06f306cd6340e2cb7331d17ae996e74cb6 --tls=false
docker exec citadel-cpu-phase0-docker docker pull \
  alpine@sha256:85fe1e81d6758c208f3e1eed4338a1997e19d4be002d4dd32d3100c9a8c010a0
(cd rust && cargo build --locked --release -p citadel-server)
python3 rust/scripts/measure-cpu-refactor.py /tmp/citadel-cpu-phase0-capture
docker stop citadel-cpu-phase0-docker citadel-cpu-phase0-postgres
```

Core's fixture API uses port 58049 and its current default all-interface bind;
the fixture uses disposable credentials. Docker and PostgreSQL are localhost-only.

## Results and validation

The uninstrumented release was rebuilt from
`4f7cb7b28ee87acb0d31324b16942b559b9bd44e` before Rust source edits. Its SHA-256 is
`6d5dd3f53c206695778f8022b4024b069b004a23cd02cc48e298ce785b17f8dd`.
The capture's dirty flag reflects source edits prepared **after** that binary was
built; no compilation ran during the accepted capture. The short initial smoke
ran concurrently with compilation and is excluded from these results.

Machine: WSL2 Linux 5.15.153.1, x86-64, Intel i7-9750H, 12 logical CPUs. Shared
host; single captures, not statistical claims. Immutable image digests appear in
the reproduction commands. The normal release profile, allocator and Tokio
thread sizing are unchanged.

| Scenario | Seconds | CPU avg / p95 / max (% of one CPU) | Docker requests | PostgreSQL statements | Full inventory passes |
| --- | ---: | ---: | ---: | ---: | ---: |
| Idle | 300 | 0.320 / 1.996 / 4.987 | 140 | 2,771 | 0 |
| Single stop | 20 | 0.550 / 2.996 / 2.996 | 24 | 378 | 1 |
| Stats, 20 running | 60 | 0.550 / 1.997 / 3.989 | 149 | 856 | 0 |
| 20-stop | 30 | 1.567 / 5.986 / 6.995 | 198 | 2,309 | 11 |
| 20 create/delete pairs | 51.33 | 0.526 / 0.999 / 3.997 | 48 | 1,013 | 0 |
| Recovery idle | 60 | 0.383 / 1.998 / 3.987 | 29 | 572 | 0 |

Baseline findings:

- One stop: one Docker stop, four inspections, three event state UPDATEs plus
  one command state UPDATE, and one full inventory pass. A slow stats metadata
  refresh also fell in this window, so info/image/network/volume totals are two
  each, not proof of two inventory passes. Command plus verification took 1.557 s.
- Twenty stops: 20 stop calls, 80 inspections, 60 event state UPDATEs plus 20
  command state UPDATEs, and 11 full inventory passes. Info/image/network/volume
  lists each total 12 including one slow stats refresh. Command plus verification
  took 10.766 s. Debounce timing makes the full-pass count variable across runs.
- Twenty running containers: 120 Docker stats calls and six stats persistence
  statements in 60 s. Writes follow the ten-second sampling cadence.
- Idle: 30 container discovery lists, four slow metadata refreshes, and 90 pings
  over five minutes. PostgreSQL averages 9.24 statements/s and Docker 0.467
  requests/s. There are 60 health-owned deployment scans and 30 stale container
  claim scans (none returned work). Alert-delivery claims (300), Git claim and
  scheduling reads (151 each), and transaction statements dominate SQL frequency.

Instrumented release SHA-256:
`129216f85c33adf2e10902888af1d0a2cc0309beea36d8a208af3b04c5a038b3`.
This is the Phase 0 source change contained in the commit that adds this report,
based on `4f7cb7b28ee87acb0d31324b16942b559b9bd44e`. No behavior optimization is
claimed by this comparison. Raw captures: `/tmp/citadel-cpu-phase0-before` and
`/tmp/citadel-cpu-phase0-instrumented`.

| Instrumented scenario | Seconds | CPU avg / p95 / max | Docker requests | PostgreSQL statements | Full inventory passes |
| --- | ---: | ---: | ---: | ---: | ---: |
| idle | 299.98 | 0.350 / 0.998 / 4.984 | 140 | 2,763 | 0 |
| single-stop | 20.00 | 0.500 / 1.993 / 1.993 | 24 | 378 | 1 |
| stats-20-running | 60.00 | 0.500 / 0.998 / 2.990 | 144 | 843 | 0 |
| 20-stop | 30.00 | 1.367 / 4.980 / 6.990 | 189 | 2,195 | 10 |
| event-burst | 40.99 | 0.512 / 0.998 / 2.994 | 41 | 855 | 0 |
| recovery-idle | 60.00 | 0.267 / 1.995 / 2.996 | 24 | 569 | 0 |

Idle Docker calls are identical (140), and SQL differs by eight background
statements over five minutes (2,771 → 2,763). Average CPU is 0.320% → 0.350%
of one CPU; this single shared-host comparison does not establish a statistically
significant overhead. Single-stop Docker and SQL totals match exactly. Bulk stop
still performs 80 inspections and 60 event projection updates; the existing
debounce produced ten full passes versus eleven before instrumentation. Timing
also moves slow stats metadata refreshes between windows. These differences are
not evidence of an optimization. Event-burst elapsed time includes the nested
Docker CLI creating/removing all 20 pairs and varies with host scheduling.

Counter cross-checks:

| Counter | Single stop | 20-stop |
| --- | ---: | ---: |
| `DockerRawEvent` (units) | 4 | 80 |
| `ContainerEventInspect` (iterations) | 3 | 60 |
| `ContainerEventProjection` (units) | 3 | 60 |
| `ContainerMutation` (units) | 1 | 20 |
| `ContainerVerification` (iterations) | 1 | 20 |
| `InventoryRequestNetwork` (units) | 1 | 20 |
| `InventoryLocalEvent` (iterations) | 1 | 10 |

The stats window reports `StatsCommit=6`, `StatsCommittedSamples=120`, and
120 Docker stats requests. Idle reports 30 committed stats transactions, 30
stale-container query passes with zero claims, and 60 health-owned deployment
reconciliation passes. These agree with the SQL capture. Realtime invalidations
remain zero in this unsubscribed fixture.

Additional instrumented baseline (not collected by the first driver version):

| Scenario | RSS / PSS mean (MiB) | Threads mean | PostgreSQL RX / TX (KiB) | Docker RX / TX (KiB) |
| --- | ---: | ---: | ---: | ---: |
| idle | 51.89 / 49.58 | 13.13 | 675.3 / 456.7 | 29.0 / 102.9 |
| single-stop | 52.48 / 50.18 | 13.47 | 92.2 / 60.8 | 5.7 / 61.1 |
| stats-20-running | 53.57 / 51.27 | 13.12 | 186.8 / 117.6 | 43.5 / 509.1 |
| 20-stop | 53.95 / 51.65 | 13.32 | 452.0 / 192.6 | 54.6 / 1418.8 |
| event-burst | 53.99 / 51.69 | 13.10 | 182.2 / 120.7 | 16.9 / 263.2 |
| recovery-idle | 54.03 / 51.73 | 13.03 | 128.8 / 90.1 | 11.9 / 214.3 |

## Regression validation

- `cargo test --locked -p citadel-runtime -p citadel-platforms -p citadel-server --lib`:
  passed (including both new counter tests; external-service tests remain opt-in).
- `citadel-platforms --test container_commands`: 4 passed.
- `citadel-adapters --test docker_transport --test local_runtime`: 29 + 7 passed.
- `citadel-adapters --test platform_inventory_persistence -- --ignored --test-threads=1`:
  all 21 PostgreSQL tests passed.
- `citadel-server --lib workers:: -- --ignored --test-threads=1`: all 16 database
  worker tests passed across the initial run and configured rerun. The three build
  tests additionally require `CITADEL_PHASE7_DATABASE_URL`; the initial invocation
  omitted it. No source fix was needed.
- `citadel-server --test platforms_http container_mutations -- --ignored --test-threads=1`:
  6 passed (claims, lock ordering, parent ownership, Edge routing and batch verification).
- `citadel-server --test platforms_http realtime_groups -- --ignored --test-threads=1`:
  11 passed in a **fresh** database. Reusing the worker-test database initially
  exposed its intentionally incomplete Deployment specs through the global admin
  reader. Keep this suite's database separate from those low-level fixtures.
- Rust formatting, `git diff --check`, source ownership guard, and Python parser /
  metrics / memory / network sampler checks passed.

Both accepted captures verify final `Exited`/`Idle` persistence for all stopped
containers. Full Agent/Edge/Swarm performance and subscribed realtime CPU baselines
are not claimed by this Local-only fixture.

## Next phase

Phase 1 should extract shared Local/Agent normalization and add parity tests.
Phase 0 intentionally retains the broad network/event fallback, sequential Local
commands, direct stats persistence, and existing recovery cadence. New normalizer,
scoped-recovery, semantic-no-op, queue, cache and writer metrics belong alongside
those implementations; zero placeholders would falsely imply coverage today.
