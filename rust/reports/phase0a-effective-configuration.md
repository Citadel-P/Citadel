# Phase 0A effective configuration

This is the non-secret configuration used by the isolated Rust viability
prototype. The running binary also emits the same values with
`citadel-server print-effective-config`; it never prints a database password.

| Setting | Effective value | Source |
|---|---:|---|
| Rust toolchain | `1.97.1` | `rust/rust-toolchain.toml` |
| Docker Engine API range | `1.41`-`1.49` | Existing `DockerApiVersion.cs` |
| Docker schema | `v1.49`, SHA-256 `1d54f6c4...94c` | Existing checked-in `v1.49.yaml` plus `schema/source.json` |
| API listen port | `8000` | Existing `Transport__ApiPort` Compose default |
| PostgreSQL host | `pg_db` when `PG_HOST` is absent | Existing `.env.example`/Compose contract |
| PostgreSQL port | `5432` | PostgreSQL protocol default |
| PostgreSQL credentials/database | Required; redacted | `DATABASE_URL`, `ConnectionStrings__Postgres`, or existing `PG_*` contract |
| PostgreSQL maximum connections | `5` | Phase 0 memory-control setting `CITADEL_RUST_DB_MAX_CONNECTIONS` |
| Docker socket | `/var/run/docker.sock` | Existing local connector contract |
| Monitoring/readiness interval | `10 seconds` | `src/Citadel.WebApi/appsettings.json` |
| Docker request timeout | `10 seconds` | Phase 0 bounded-I/O setting |
| Event queue capacity | `256`, wait/backpressure | Existing `SwarmReconciliationJob.EventQueueCapacity` |
| Docker JSON response limit | `16 MiB` | Phase 0 bounded-I/O setting |
| Docker stream item limit | `1 MiB` | Phase 0 bounded-I/O setting |
| Graceful shutdown timeout | `10 seconds` | Phase 0 cancellation gate |
| Container memory limit | `100 MiB`, swap disabled | Migration specification hard gate |
| Container PID limit | `128` | Phase 0 containment setting |

The prototype does not load the broader .NET hosted-service graph. It runs one
readiness probe, one Docker event source, and one bounded event consumer. The
configuration report is not a claim that these Phase 0-only controls are final
product defaults.
