# Phase 0B effective Agent configuration

The Agent adapter is disabled unless both its address and private-key path are
configured. Secret key material is never included in effective-configuration
output.

| Setting | Effective value | Source |
|---|---:|---|
| Agent address | Required; no default | `CITADEL_RUST_AGENT_ADDRESS` |
| Raw 32-byte private-key path | Required; redacted | `CITADEL_RUST_AGENT_PRIVATE_KEY_PATH` |
| Unary/connection deadline | Required and greater than zero; no default | `CITADEL_RUST_AGENT_TIMEOUT_SECONDS` |
| Insecure h2c allowed | `true` by existing Citadel default | `AgentTransport__AllowInsecure` |
| Statistics interval | `10 seconds` by existing Citadel default | `JobConfiguration__MonitoringInterval` |
| Stream reconnect delay | `10 seconds` | Existing `PlatformStatsStreamerJob` behavior |
| Unary retry | At most 3 attempts; `Unavailable` only; 200/400 ms waits | Existing `GrpcClientFactory` policy |
| Maximum encoded/decoded gRPC message | `16 MiB` | Existing gRPC client configuration |
| Generated protobuf subset | shared models, Platform, Container | Phase 0B authorized subset |

Direct Agent TLS is deliberately outside this prototype slice. When h2c is
used, `AgentTransport__AllowInsecure=true` must be explicit in the deployed
environment.
