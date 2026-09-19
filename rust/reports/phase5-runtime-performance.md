# v13 Phase 5 — runtime work and lifecycle

This report covers the runtime phase of the v13 architecture specification, not the earlier API-porting phase with the same number. Phase 6 resource refactoring is outside this change.

## Implementation

- Stats persistence no longer performs historical deletion. Maintenance owns bounded historical cleanup, with independent recovery (60 seconds), lease expiry (30 seconds), and configurable retention (`CITADEL_RUST_RETENTION_INTERVAL_SECONDS`, default 900 seconds). Each retention statement commits separately; a drain is limited to 128 passes or ten seconds.
- Threshold alerts flush on their configured interval, with at most ten batches or five seconds per wake. There is no per-second pending-count query. A transaction advisory lock serializes flushers; captured tuple revisions prevent a concurrent stats upsert from being acknowledged accidentally. Alert I/O holds no stats-row lock. Pending alert samples survive retention.
- Local health uses Docker ping; Direct Agent health uses `CheckHealth` on its reused channel; Edge health uses the existing heartbeat. Debounced health owns background availability transitions. Inventory refresh failures no longer independently mark a reachable platform offline.
- One process-owned registry caches only persisted connector targets and reusable Agent clients. Transactional notifications refresh it promptly; a 60-second database refresh recovers missed notifications. Address/connector changes replace clients, deletion removes them, and no ACL result is cached.
- Independent external-work limits are health 4, inventory 2, local stats 8, and slow metadata 1. Inventory requests within each target are sequential, including Edge, so nested fan-out cannot multiply its limit. Permit wait/saturation and fixed-cardinality operation counters/timers are exposed in metrics; IDs remain in logs.
- A local stats cycle discovers containers once. Slow image/network/volume/storage/disk metadata uses a 60-second refresh threshold checked on stats ticks (up to one monitoring interval later), expires at 180 seconds, and invalidates on daemon generation changes. Static version data is shared with API negotiation. Unchanged platform metadata does not rewrite its database tuple.
- Standalone container events inspect/persist the affected container, including discovery of new containers. Full inventory and single observations share the ownership-aware upsert and transactional creation notifications; a single observation never deletes absent siblings. Swarm relationship changes, unknown events, reconnects, and periodic reconciliation retain complete inventory fallback. Bounded Agent trigger overflow upgrades the queued pass to all Agents; it cannot silently discard a platform scope.
- Measured idle claimers for Automation, Builds, Backups, Restores, and build completion now drain persisted work and wait for notification, cancellation, or a 30-second minimum fallback. One multi-channel listener fans out capacity-one watch signals. Reconnect signals all consumers. Schedulers retain their time semantics.
- Equivalent Container, Stack, and Swarm runtime routers share handles. Distinct scanner/cache semantics remain separate. Platform workers are split into health, inventory, stats, events, and Agent ownership modules.
- `DynamicTasks` establishes a cloneable process task owner, error reporting, active-task metric, root cancellation, and an explicit admission gate. Shutdown stops producers, drains fixed workers, closes/drains tracked tasks within the remaining budget, then closes PostgreSQL. Feature-specific dynamic spawn migrations remain in Phases 6–9. The Core lease watcher owns its connection by value and returns it for retention through shutdown cleanup.

Implementation entry points:

| Responsibility | Rust modules |
| --- | --- |
| Metrics, bounded I/O, tracked tasks, signal names | `application/src/{runtime_metrics,io_budget,dynamic_tasks,runtime_signal}.rs` |
| Persisted targets and reusable clients | `server/src/runtime_targets.rs` |
| Platform worker split | `server/src/workers/platforms/{health,inventory,stats,events,agents}.rs` |
| Shared claim notifications and delayed scheduling | `server/src/workers/{notifications,schedule}.rs` |
| Consolidated local sampling | `adapters/src/docker/local_sampler.rs` |
| Retention ownership and alert concurrency | `adapters/src/{container_stats_store,maintenance_store}.rs`, `server/src/workers/{maintenance,stats_alerts}.rs` |
| Shared full/targeted container projection | `adapters/src/{inventory_projection_store,resource_status_store}.rs` |
| Composition and shutdown | `server/src/{state,startup,app}.rs` |

Paths above are relative to `rust/crates/`. Shared runtime-router handles are constructed explicitly in `state.rs`; there is no dynamic service-locator registry.

## Startup and recurring work classification

| Worker family | First work and reason |
| --- | --- |
| Core recovery, lease, readiness | Immediate: exclusive ownership and readiness correctness |
| Target registry, cheap health, inventory/event subscriptions, Edge sessions | Immediate/near-immediate: discover persisted targets and restore observation |
| Automation/Build/Backup/Restore/build completion claims | Immediate durable drain, then notification plus timer fallback |
| Container/volume/deployment/service/stack operation recovery | Immediate: recover persisted in-flight operations |
| Unmanaged-container, drift, pruning, job-alert consumers | Subscribe before producers; retain event-driven behavior and recovery fallback |
| Automation/Backup due-time schedulers | Immediate, then configured time cadence |
| Alert delivery claims, Build pool health | Immediate: resume persisted delivery work and establish executor availability; existing bounded fallback retained |
| License transition monitor | Immediate deadline/state evaluation, retaining authorization-related timing |
| Service-account last-used buffer | Existing bounded channel/timer flush; empty ticks perform no database write |
| Git execution claims | Immediate, existing bounded idle backoff; due-time scheduling starts after its first 60-second period |
| Local stats | First monitoring period plus deterministic worker-name offset |
| Threshold alert flush | First configured flush interval |
| Stack webhook/update/drift polls | First normal period plus deterministic worker-name offset |
| Image scan | First complete pass after 5.733 seconds, then the existing long interval |
| Deployment/Swarm Service image-update sweeps | Existing delayed first two-hour interval retained |
| Maintenance recovery/expiry/retention | Independent first normal periods plus deterministic offsets |

The deterministic scheduler test checks that expensive first iterations differ. Correctness-driven immediate work is intentionally retained.

## Measurement method

The accepted Phase 4 release was measured for 60 seconds of startup and 900 seconds of idle before optimization. A statement snapshot at 611 seconds preceded the first optimization. An instrumented pre-optimization release then ran the complete 1,800-second Phase 0 workload. Instrumentation and observer overhead are included; CPU uses process user+system ticks, with 100% equal to one logical CPU.

The fixture uses disposable, pinned PostgreSQL and Docker-in-Docker containers, one persisted Local platform, a ten-second monitoring interval, database pool five, event queue 256, and the existing allocator. It includes startup, idle, ordinary API reads, OpenAPI/docs access, realtime reconnect/slow-consumer traffic, one running container, ten create/remove bursts, log streaming/disconnect, and a final soak. PostgreSQL statement rates include transaction statements but exclude the observer query. Docker rates come from a socket proxy recording request paths without credentials.

The host is WSL2 Linux 5.15.153.1, x86-64, Intel i7-9750H with 12 online logical CPUs. PostgreSQL is pinned to `postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44`; Docker-in-Docker to `docker@sha256:3ef33f2e220b79ed3ef3b99d81746f06f306cd6340e2cb7331d17ae996e74cb6`; the preloaded workload image to `alpine@sha256:85fe1e81d6758c208f3e1eed4338a1997e19d4be002d4dd32d3100c9a8c010a0`. CPU/residency samples are taken once per second and database/metrics observations every five seconds. Percentiles use nearest rank. The host is shared, so small absolute differences and isolated one-second peaks must be interpreted cautiously.

Raw captures and measurement drivers are outside Git under `~/.codex/citadel-refactor-audit-20260919/refactor-v13/`. Only this result report is added for the audit. The failed initial capture before Docker readiness and the preliminary `phase5-after-initial` diagnostic run are excluded from the final comparison. That preliminary run exposed the missing targeted discovery upsert; the live burst smoke then verified zero additional inventory passes after the fix.

The live Direct Agent image registry returned unauthorized. Protocol fixtures exercise signed `CheckHealth`, channel reuse, target replacement/removal, stream reconnect/cancellation, and bounded responses; they do not substitute for a measured production Agent workload. The live Agent performance row is unavailable.

Release captures run sequentially without concurrent compilation. The profile experiment changes only optimization level; thin LTO, one codegen unit, `panic=abort`, symbol stripping, allocator, and runtime sizing remain fixed. The thread experiment uses the same final `z` binary with `TOKIO_WORKER_THREADS=4`.

| Release | Bytes | SHA-256 |
| --- | ---: | --- |
| Accepted Phase 4 | 40,731,584 | `b9ddfb3a3dd9c4f35f9924cd822d4940e93463f5a04c35c6dc7408b7d3a7d88a` |
| Instrumented before | 40,735,952 | `a2c893e5ad42b637e41d11ca6e0736af025494c6deeba7a36cc18991f1ed75da` |
| Final Phase 5, `z` | 40,784,136 | `31c94e9b1a631e5f5d34ada893788ca816ea740b07ccd7e5eb2924f27b639222` |
| Final Phase 5, `3` | 55,492,944 | `3a6f4e0c47adb80128f8d8cfdf46e9be2a5cebbcd85fe08b95eaa1c484863f1c` |

## Results

Values below are **before → after**, comparing the instrumented accepted Phase 4 release with the final Phase 5 `z` release. CPU is percent of one logical CPU. Rates use the same windows; proxy wall-clock timestamps are aligned to the monotonic scenario markers to avoid assigning boundary requests to the preceding scenario. Inventory counts use timestamped iteration records and their durations to locate starts exactly; five-second metric snapshots alone can miss a pass at a window boundary.

| Scenario | CPU avg | CPU p95 | CPU max | Docker req/s | DB statements/s | Full inventory counter |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Startup, first 60 s | 1.676 → 1.245 | 4.000 → 2.618 | 35.961 → 55.028 | 1.433 → 0.783 | 16.182 → 7.836 | 2* → 2 |
| Idle 5 min | 0.518 → 0.274 | 2.000 → 1.001 | 3.001 → 3.578 | 1.313 → 0.467 | 16.258 → 8.617 | 0 → 0 |
| Idle 15 min | 0.548 → 0.283 | 2.000 → 1.001 | 7.473 → 3.578 | 1.314 → 0.472 | 16.352 → 8.869 | 0 → 0 |
| One running container | 0.532 → 0.296 | 1.536 → 1.035 | 3.000 → 3.823 | 1.365 → 0.573 | 16.441 → 8.541 | 0 → 1† |
| Ten create/remove pairs | 0.790 → 0.406 | 2.002 → 1.296 | 2.998 → 3.584 | 2.750 → 0.750 | 23.837 → 11.291 | 10 → 0 |
| Maintenance boundary | 0.552 → 0.315 | 2.000 → 1.088 | 2.997 → 2.774 | 1.160 → 0.360 | 19.350 → 13.150 | 0 → 0 |
| Final soak, 1442–1800 s | 0.471 → 0.281 | 1.998 → 1.861 | 2.438 → 2.987 | 1.313 → 0.480 | 16.237 → 8.657 | 1 → 1† |
| One live Direct Agent | Unavailable | Unavailable | Unavailable | Unavailable | Unavailable | Unavailable |

*The pre-change startup counter includes the initial worker iteration before targets exist; the final counter measures actual target passes. Synchronous registration enumeration is included in Docker totals, not in this background counter. Idle and event-burst counts are directly comparable for the fixture's one persisted target.

†Starting/removing the running container also changes its network attachment, which correctly retains full reconciliation. The start-related pass falls just before the fixed running window in the baseline and just inside it after the change; both removal-related passes fall at the beginning of the final soak. This boundary difference is separate from the ten create/remove pairs, which do not start containers and trigger zero full passes after the change.

Over fifteen idle minutes, average CPU fell about 48%, Docker traffic 64%, and database statement traffic 46%. The event burst eliminated all ten full scans and reduced Docker requests about 73%. Individual one-second maxima did not uniformly decrease: startup initialization/setup remains the largest CPU sample, and first documentation access remains a distinct CPU/memory warm-up step. These are single captures with low absolute CPU usage, not statistical claims about all installations.

| Docker operation during the 15-minute idle window | Before calls | After calls |
| --- | ---: | ---: |
| Container list | 360 | 90 |
| Version | 180 | 0 |
| Info | 270 | 13 |
| Image list | 90 | 13 |
| Network list | 90 | 13 |
| Volume list | 90 | 13 |
| Storage usage | 13 | 13 |
| Ping | 90 | 270 |

Ping increases because it replaces the expensive health enumeration while retaining readiness checks. There is exactly one discovery list per ten-second stats cycle in this idle window. Storage usage was already cached before this phase; the gain is consolidating the other slow metadata calls and disk sampling around that cadence.

Across the complete 30-minute capture, the former pending-alert count query ran 1,772 times and is absent after the change. The platform metadata UPDATE still executes once per persisted sample, but affected rows fell from 179 to four; unchanged samples no longer rewrite the tuple. Retention iterations fell from 31 to one, while stale-operation recovery retained its one-minute cadence. The largest remaining recurring application SQL total is alert-delivery claiming: 1,795 calls, 69.7 ms total execution time. Git claims also retain their existing cadence. These are named remaining costs, not evidence for speculative cleanup indexes. The observer's own statement aggregation cost is excluded from application statement rates.

| Worker, complete capture | Before mean / p95 / max (ms) | After mean / p95 / max (ms) |
| --- | ---: | ---: |
| Health | 34.607 / 64.677 / 82.135 | 1.477 / 2.129 / 11.186 |
| Local stats | 286.508 / 1156.003 / 1259.825 | 209.272 / 1042.856 / 1234.565 |
| Stats persistence | 4.830 / 12.590 / 32.776 | 2.297 / 5.726 / 15.055 |
| Recovery | 29.481 / 68.848 / 142.893 | 13.751 / 30.726 / 38.044 |
| Threshold alert flush | 41.299 / 61.070 / 93.109 | 33.724 / 44.285 / 69.942 |

Worker times are elapsed durations, including awaited I/O. In particular, one-container Docker stats calls take about one second; their latency is not one second of CPU consumption. Realtime traffic completed 72 reconnects and 72 slow-consumer connections; log streaming delivered 130 messages in 30 seconds. Shutdown completed cleanly.

### Secondary memory baseline

Values are window means, in MiB. The comparison keeps the allocator and production release settings fixed.

| Window | RSS before → after | RSS delta | PSS before → after | PSS delta |
| --- | ---: | ---: | ---: | ---: |
| Startup | 108.25 → 105.13 | −3.12 | 106.02 → 103.11 | −2.91 |
| Idle 5 min | 109.77 → 107.44 | −2.33 | 107.43 → 105.39 | −2.04 |
| Idle 15 min | 110.12 → 107.64 | −2.48 | 107.79 → 105.58 | −2.21 |
| After first docs access | 162.53 → 160.03 | −2.50 | 160.19 → 157.98 | −2.21 |
| One running container | 163.82 → 161.01 | −2.81 | 161.48 → 158.95 | −2.53 |
| Final soak | 163.34 → 161.27 | −2.07 | 161.00 → 159.21 | −1.79 |

First documentation access still adds about 52 MiB RSS and is responsible for the visible memory step; no claim is made that this phase removes that cache/initialization cost. RSS/PSS are process residency measures, not live Rust heap. These results are a Phase 5 attribution point only: **Phase 13 must measure a fresh baseline after Phase 12**, before allocator, ownership, dependency, or small-object optimization.

### Release-profile experiment

Both captures use default Tokio sizing. CPU values are average / p95 / maximum percentages; residency is mean RSS / PSS in MiB. All other release settings are fixed as described above.

| Measurement | `opt-level="z"` | `opt-level=3` |
| --- | ---: | ---: |
| Binary bytes | 40,784,136 | 55,492,944 |
| Startup CPU, first 60 s | 1.245 / 2.618 / 55.028 | 0.942 / 1.001 / 23.991 |
| First healthy response, seconds | 2.132 | 1.628 |
| Idle 15 min CPU | 0.283 / 1.001 / 3.578 | 0.304 / 1.233 / 3.108 |
| Idle RSS / PSS | 107.64 / 105.58 | 117.72 / 115.35 |
| One-container CPU | 0.296 / 1.035 / 3.823 | 0.316 / 1.647 / 2.353 |
| Event-burst CPU | 0.406 / 1.296 / 3.584 | 0.379 / 1.154 / 2.843 |
| Realtime CPU | 1.000 / 2.023 / 3.959 | 1.022 / 2.458 / 3.000 |
| Log-stream CPU | 0.350 / 1.001 / 3.003 | 0.354 / 1.737 / 2.000 |
| Final-soak CPU | 0.281 / 1.861 / 2.987 | 0.293 / 1.312 / 2.683 |
| Final-soak RSS / PSS | 161.27 / 159.21 | 171.96 / 169.59 |
| Platform reads, mean / p95 ms | 8.959 / 15.627 | 10.886 / 23.445 |
| Deployment reads, mean / p95 ms | 8.492 / 24.967 | 10.975 / 22.323 |
| First OpenAPI request, ms | 188.136 | 172.710 |
| Health worker, mean / p95 ms | 1.477 / 2.129 | 2.814 / 5.749 |
| Local stats, mean / p95 ms | 209.272 / 1042.856 | 214.407 / 1063.200 |
| Stats persistence, mean / p95 ms | 2.297 / 5.726 | 2.977 / 8.026 |

Ordinary API latencies use 30 reads per endpoint in the API window and exclude initialization/registration. These are client-observed elapsed times, not isolated server CPU measurements. Startup includes a fresh database and readiness polling at 500 ms intervals. Both profiles completed 72 reconnects, 72 slow-consumer connections, and 130 log messages in 30 seconds; both performed zero full inventory passes during the burst. Idle Docker traffic was identical (0.472 requests/s), with database traffic 8.869 versus 8.822 statements/s.

**Decision: retain `z`.** The candidate has a 36.1% larger binary, about 10 MiB higher steady residency, and no consistent steady-state CPU or request/worker latency benefit. Faster observed startup and somewhat lower individual CPU peaks are useful evidence, but do not establish a clearly better total trade-off on this single, I/O-heavy fixture.

### Tokio thread-count experiment

Both captures use the identical final `z` binary. The candidate changes only `TOKIO_WORKER_THREADS=4`. CPU values again mean average / p95 / maximum; latency and residency use the same definitions as above.

| Measurement | Default sizing | Four workers |
| --- | ---: | ---: |
| Mean process thread count while idle | 13.144 | 5.144 |
| Startup CPU, first 60 s | 1.245 / 2.618 / 55.028 | 1.284 / 2.672 / 35.202 |
| First healthy response, seconds | 2.132 | 1.656 |
| Idle 15 min CPU | 0.283 / 1.001 / 3.578 | 0.311 / 1.260 / 3.564 |
| Idle RSS / PSS, MiB | 107.64 / 105.58 | 107.12 / 104.95 |
| One-container CPU | 0.296 / 1.035 / 3.823 | 0.366 / 1.316 / 4.033 |
| Event-burst CPU | 0.406 / 1.296 / 3.584 | 0.388 / 1.740 / 3.161 |
| Realtime CPU | 1.000 / 2.023 / 3.959 | 1.223 / 2.002 / 3.459 |
| Log-stream CPU | 0.350 / 1.001 / 3.003 | 0.414 / 1.732 / 2.504 |
| Final-soak CPU | 0.281 / 1.861 / 2.987 | 0.320 / 1.408 / 4.285 |
| Final-soak RSS / PSS, MiB | 161.27 / 159.21 | 160.01 / 157.83 |
| Platform reads, mean / p95 ms | 8.959 / 15.627 | 12.773 / 36.572 |
| Deployment reads, mean / p95 ms | 8.492 / 24.967 | 11.029 / 23.460 |
| First OpenAPI request, ms | 188.136 | 217.927 |
| Health worker, mean / p95 ms | 1.477 / 2.129 | 1.762 / 3.071 |
| Local stats, mean / p95 ms | 209.272 / 1042.856 | 216.272 / 1069.238 |
| Stats persistence, mean / p95 ms | 2.297 / 5.726 | 2.616 / 7.499 |

The four-worker run completed the same 72 reconnects, 72 slow-consumer connections, and 130 log messages, with zero full inventory passes during the event burst. Idle Docker traffic remained 0.472 requests/s; database traffic was 8.821 statements/s. All three final release captures ran for 1,800 seconds, completed successfully, and recorded no server error-level events.

**Decision: retain default Tokio sizing.** The candidate saved only 0.52 MiB idle RSS and 1.26 MiB in the final soak, while this capture had higher CPU averages and mean API/worker latency. Realtime completion counts were preserved, but there is no measured overall benefit. This fixture contains neither a live Direct Agent nor an active build, so their responsiveness under a smaller runtime is not claimed. Workload I/O limits remain explicit and independent of CPU/thread count regardless of this decision.

### Original architecture baseline

The original Phase 0 raw capture, rebinned into the same fixed windows, provides the architectural baseline. The direct before/after comparison above uses the accepted Phase 4 implementation so earlier architecture changes are not attributed to Phase 5.

| Phase 0 window | CPU avg / p95 / max (%) | Docker req/s | DB statements/s | RSS / PSS (MiB) |
| --- | ---: | ---: | ---: | ---: |
| Startup, 0–60 s | 1.444 / 2.273 / 35.983 | 1.433 | 16.091 | 105.14 / 102.90 |
| Idle 5 min, 60–360 s | 0.491 / 2.000 / 4.000 | 1.313 | 16.173 | 106.67 / 104.30 |
| Idle 15 min, 60–960 s | 0.504 / 2.000 / 4.146 | 1.314 | 16.297 | 106.88 / 104.50 |
| One running container, 1142–1320 s | 0.461 / 1.999 / 2.406 | 1.365 | 16.282 | 160.20 / 157.82 |
| Event burst, 1320–1380 s | 0.773 / 2.001 / 3.933 | 2.750 | 23.945 | 160.20 / 157.83 |
| Maintenance boundary, 895–920 s | 0.395 / 1.063 / 2.002 | 1.160 | 18.900 | 107.09 / 104.71 |

Phase 0 did not expose the new inventory counter. The live Direct Agent row is unavailable in both the original and current fixture; no Agent CPU comparison is claimed.

## Deferred changes and limits

- No allocator, small-string/container, or speculative dependency pruning: those belong to Phase 13, after a fresh Phase 12 baseline.
- No speculative cleanup indexes or relational rewrite of build-reference JSON. Retention frequency is corrected first; the fixture does not contain a representative historical build backlog to justify schema changes.
- Existing dedicated notification consumers remain separate because their payload/recovery ownership differs. The new claim channels share one bounded hub.
- Git execution claims and alert delivery polling remain at their existing bounded cadence; expanding notification ownership requires separate evidence and recovery tests.
- This workload has one local container and no live Direct Agent or active build process. It can reject obvious profile regressions and verify request reductions, but cannot justify universal thread-count tuning for large installations.
- File splitting is an ownership/readability change; no CPU or memory improvement is attributed to it alone.

## Validation

Commands run from `rust/`, with Rust 1.97.1:

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed |
| `cargo test --locked --workspace` | 638 passed, 238 environment-dependent tests ignored, zero failures |
| Server library, `--include-ignored --test-threads=1`, isolated PostgreSQL database | 118 passed, zero ignored/failures |
| Platform HTTP, `--include-ignored --test-threads=1` | 62 passed |
| Real-process bootstrap/lifecycle, `--include-ignored --test-threads=1` | 8 passed |
| Docker socket transport | 18 passed, including endpoint counts, metadata freshness, and daemon invalidation |
| Direct Agent transport | 8 passed, including signed health without inventory, rebinding, deadlines, and cancellation |
| Live disposable Docker discovery smoke | Ten create/remove pairs caused zero full inventory passes after startup; development build, request-count evidence only |
| Final release performance fixture | Three sequential 30-minute captures: `z`/default, `3`/default, `z`/four workers; all completed, no server errors |
| `cargo run --locked -p xtask -- openapi --check` | 404 full and 305 public operations verified |
| `cargo run --locked -p xtask -- docker-api --check` | Docker API 1.49: 249 generated files verified with generator 7.25.0 |
| `git diff --check` | Passed |

`docker-api --check` is the accepted Phase 4 replacement for the specification's obsolete `docker --check` spelling. PostgreSQL tests run against disposable databases separate from the measurement database; queue-claim tests require an otherwise isolated database. Early reruns against a shared dirty fixture failed and are excluded from the passing acceptance run.

Regression coverage includes explicit maximum in-flight sampling, independent inventory/health budgets, cancellation while waiting for capacity, sequential nested inventory I/O, no DELETE on stats persistence, unchanged metadata tuple preservation, alert failure/concurrent-upsert safety, pending-alert retention, target channel reuse/address generation/removal, transaction-bound notifications and real listener termination/reconnect, event burst coalescing/overflow/follow-up, deterministic first ticks, tracked cleanup/admission deadlines, and real listener/worker shutdown. Two old HTTP expectations were updated for the deliberate contract changes: one version lookup per inventory pass, and retention owned by maintenance.
