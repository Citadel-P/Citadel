# Phase 0A provisional decision report

Status: **implementation and short baseline complete; full 24-hour measurement pending**.

Phase 0A is isolated from production routing and owns no customer mutation or
database schema. Passing this report cannot authorize Phase 0B, Phase 0C, or a
rewrite decision.

## Implemented evidence

- Pinned Rust 1.97.1 workspace with exact dependency versions and lock file.
- Axum liveness/readiness/OpenMetrics endpoints and bounded graceful shutdown.
- SQLx pool plus Citadel's existing Actor/direct/team-authorized Platform read.
- SHA-pinned Docker v1.49 subset generator for Ping, Version, Info, container
  list/inspect, events, and stats.
- Unix-socket transport with Citadel's existing API range negotiation.
- Bounded one-megabyte stream records, bounded response bodies, a 256-item
  backpressured event queue, capped reconnect delay, and an owned task tree.
- Pinned release image, 100 MiB cgroup limit, swap disabled, non-root runtime,
  read-only filesystem, dropped capabilities, and a 128 PID limit.
- Automated Unix-socket fixtures, authorization fixture, formatting/lint/tests,
  short measurement, and 24-hour soak commands.

## Results

| Measure | Result | Gate |
|---|---:|---:|
| Unit/Unix-socket compatibility tests | Pass | Pass |
| Actor-authorized PostgreSQL fixture | Pass | Pass |
| First cold optimized release build | 133.766 seconds | Report |
| Current cached optimized release build | 36.103 seconds | Report |
| Stripped binary size | 3,430,392 bytes (3.27 MiB) | Report |
| Runtime image size | 29,755,903 bytes (28.38 MiB) | Report |
| Short-run p95/p99 `memory.current` | 7,340,032 bytes (7.00 MiB) | Provisional only |
| Short-run maximum `memory.peak` | 12,791,808 bytes (12.20 MiB) | `< 100 MiB`, no OOM/swap |
| 24-hour p95 `memory.current` | Pending | `< 60 MiB` representative |
| 24-hour p99 burst `memory.current` | Pending | `< 85 MiB` |
| Retained-memory slope | Pending 24-hour run | No statistically meaningful growth |
| Short-run CPU p95/max | 0.12% / 0.12% during Docker smoke bursts | Provisional only |
| Short-run health p95 latency | 150.706 ms via Windows PowerShell | Provisional only |
| Short-run Tokio tasks | 3 min / 3 max | Stable |
| Short-run processes / FDs / Unix sockets | 16 / 14 / 6 maximum | Stable |
| Short-run PostgreSQL pool | 1 max / 1 minimum idle | `<= 5` and returns idle |
| Short-run event queue / lag / reconnects | 0 max / 5 ms max / 0 | Bounded/no sustained lag |
| Short-run graceful shutdown | 634.267 ms, exit 0, no OOM | `< 10 seconds`, no leaked process |

## Required decision step

The measured baseline above is a 72-second implementation smoke and cannot be
used to evaluate retained-memory slope. Run
`./rust/scripts/Measure-Phase0A.ps1 -DurationHours 24`, attach the generated
CSV/JSON metrics, and replace the pending rows only with measured results. Do
not authorize later phases from a short smoke run. A Windows-native developer
build also requires the Visual Studio Build Tools C++ workload; the release and
test scripts use the pinned Linux builder and are already reproducible without
that host linker.
