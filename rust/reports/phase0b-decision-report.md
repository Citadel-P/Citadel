# Phase 0B Agent viability report

Status: **implemented and verified against the active .NET Agent**.

This report closes only the Phase 0B work package. It did not itself authorize
Phase 0C, customer mutations, schema changes, frontend migration, production
routing, or removal of the .NET implementation. Phase 0C was subsequently
authorized and is reported separately.

## Implemented evidence

- Checksum-gated generation of Citadel's current shared, Platform, and Container
  protobuf contracts with vendored `protoc`.
- Exact compatibility with Citadel's existing Ed25519 request signature:
  little-endian timestamp, 16-byte nonce, full gRPC method name, protobuf body
  SHA-256, and the four existing binary metadata headers.
- A single `PlatformRuntimePort` implemented by both the local Docker adapter and
  the Agent adapter.
- Signed `GetPlatformInfo` handshake and signed container-list unary read.
- Signed server-streaming Platform statistics with bounded 16 MiB messages.
- Explicit unary deadlines, local cancellation, normalized transport errors,
  and the existing retry rule: at most three attempts, only for `Unavailable`,
  with fresh signatures/nonces on every attempt.
- An owned Agent stats worker using Citadel's existing 10-second reconnect delay
  and counters for samples, reconnects, and handshake failures.
- An isolated PowerShell harness that builds and starts the active .NET Agent,
  creates an ephemeral signing key, runs Local/Agent equivalence and stream
  checks, then removes the container, network, and private key.

## Verification results

| Check | Result |
|---|---|
| Checksum-gated protobuf generation | Pass |
| Rust workspace tests | Pass |
| Clippy, all targets, warnings denied | Pass |
| Bounded retry with fresh nonces | Pass |
| Unary deadline normalization | Pass |
| Open-stream cancellation within 2 seconds | Pass |
| Active .NET Agent signed handshake | Pass |
| Representative Agent container read | Pass |
| Local/Agent daemon/API/container-ID equivalence | Pass; 19 containers in the measured run |
| Active .NET Agent stats stream | Pass |
| Combined Phase 0A + Agent workload | Pass; 4 owned tasks, Agent sample received, no OOM, clean exit 0 |
| Agent harness cleanup | Pass; no retained container, network, or key |

The measured active-Agent run negotiated Docker API `1.49` with minimum daemon
API `1.40`, matched the same daemon identifier and all container identifiers,
received a live CPU/memory/network sample, and observed stream cancellation.
The short combined workload then ran the HTTP/readiness, PostgreSQL, Docker
event, and Agent workers under the 100 MiB cgroup and shut down cleanly.

## Defects found by the equivalence gate

1. The Local adapter initially exposed Docker's raw maximum API rather than
   Citadel's negotiated API. It now reports the negotiated version, matching the
   Agent and the existing .NET behavior.
2. The Agent stats adapter initially read unused duplicate top-level container
   counters. Citadel's active Agent populates the counters inside `stat`, as the
   existing .NET connector expects. The Rust mapping now does the same and has a
   regression test.
3. The Agent repository still targeted .NET 10 while its shared contracts had
   moved to .NET 11. Its target/base images were aligned, and the redundant
   `PublishSingleFile` setting was removed because it produced a broken apphost
   with Native AOT on the current .NET 11 preview.

## Deliberate limits

- Phase 0B exercises the current regular-Agent h2c mode only, and requires
  `AgentTransport__AllowInsecure=true`. Direct TLS belongs to the existing
  transport-security phase and was not reimplemented here.
- Only the protobuf packages and operations required by Phase 0B are generated.
- The long combined decision soak remains deferred until Phase 0C adds the
  realtime workload. A 24-hour wait is not required to validate this isolated
  transport slice.

Phase 0C is documented in `phase0c-decision-report.md`.
