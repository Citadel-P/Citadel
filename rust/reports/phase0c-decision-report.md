# Phase 0C realtime viability report

Status: **implemented and verified with a short production-shaped workload**.

This closes the Phase 0C implementation slice. It does not authorize product
routing, replacement of the existing SignalR path, customer/data mutations,
Phase 1, or a final Rust rewrite decision. The specification's full combined
decision soak has not been run.

## Implemented evidence

- One versioned WebSocket subscription for the configured local Platform.
- A prototype token that authenticates one configured Actor, followed by the
  existing PostgreSQL Actor/Team/Role/resource-ACL authorization query.
- Initial, periodic, overflow, manual-resync, and reconnect authorization
  checks.
- A versioned envelope containing connection ID, connection sequence, resource
  type/ID/revision, event kind, payload schema version, and payload.
- A shared bounded broadcast ring. Tokio's explicit lag signal produces
  `resyncRequired` plus an authorized bounded snapshot rather than silent loss.
- Bounded client message size, write buffer, write time, snapshot size,
  subscription time, and total concurrent connections.
- Docker events and one local container statistics stream feeding Platform
  runtime revisions, alongside the Phase 0B active-Agent statistics stream.
- A dependency-free Node WebSocket client proving the language-neutral wire
  contract, sequence checks, manual resync, reconnect snapshots, and bounded
  client buffering.
- A repeatable PowerShell harness with real PostgreSQL ACL state, the active
  .NET Agent, 20 deterministic containers, event bursts, slow/reconnecting
  clients, cgroup memory sampling, metrics assertions, and cleanup.

## Verification results

| Check | Result |
|---|---|
| Rust workspace and WebSocket integration tests | Pass |
| Invalid token rejected before snapshot | Pass |
| PostgreSQL resource authorization before snapshot | Pass |
| Connection sequence and resource revision | Pass |
| Manual resync and reconnect snapshot | Pass |
| Bounded queue lag detection | Pass |
| Total connection limit | Pass |
| Language-neutral Node client | Pass |
| Active .NET Agent stream in combined process | Pass |
| Local container statistics stream | Pass |
| Deterministic reference containers | Pass; 10 running and 10 stopped |
| Slow/reconnecting-client cleanup | Pass; zero active clients after workload |
| Combined short cgroup run | Pass; no OOM, no swap use, clean exit `0` |

The measured 10-second workload observed:

- `5` owned background tasks;
- `364` published runtime events;
- `136` delivered realtime messages;
- `3` queue overflows followed by resynchronization;
- `24` authorized snapshots;
- `38` local statistics samples and an active-Agent sample;
- `0` retained connections and `0` send timeouts after the client workload;
- `4,947,968` bytes maximum `memory.current` under the `100 MiB` hard limit.

The short measurement proves bounds and cleanup, not a statistically meaningful
retained-memory slope.

## Defects found by the viability gate

1. A client that attached one-shot message listeners could miss a message
   between reads. The language-neutral client now installs one persistent,
   bounded inbox before subscribing and treats sequence gaps as resync signals.
2. Per-client queues and deadlines did not by themselves cap the number of
   client tasks. A semaphore now rejects connections above the configured
   global limit before allocating subscription state.
3. The first harness assumed Linux cgroup v2. The memory probe now supports both
   cgroup v2 and the cgroup v1 layout used by some Docker Desktop environments.

## Deliberate limits

- This is a Platform runtime subscription only. Product resource events, logs,
  terminal streams, and statistics fan-out remain outside Phase 0.
- The fixture token is not the final JWT/service-account implementation.
- Resource revisions are process-lifetime observation revisions, not a durable
  replay log. Reconnect always snapshots and never promises replay.
- The current React frontend and .NET Core continue using SignalR. The Node
  client proves protocol viability without creating split production ownership.
- Run `Test-Phase0CRealtime.ps1 -DurationSeconds 86400` before requesting the
  formal Phase 0 rewrite decision if the full specification gate is retained.

Phase 1 has not been started.
