# Citadel Rust workspace

Core, Agent and the volume helper share this Cargo workspace. See
[DEVELOPMENT.md](DEVELOPMENT.md) for local setup and [ARCHITECTURE.md](ARCHITECTURE.md)
for ownership and dependency rules.

## OpenAPI generation

Run `bash rust/scripts/build.sh` from the repository root to build the API and
export `schema/v1.json` and `schema/public-v1.json`. The VS Code build/run and
debug workflows do this automatically. Generation also copies the full schema to
`src/Citadel.FrontEnd/src/api/schema/swagger.json` and regenerates the TypeScript
client with `npm run api:generate`; install frontend dependencies with `npm ci` first.
Plain `cargo build` only compiles.

The API uses Utoipa and utoipa-axum to share route registration and documentation.
Schemas derive from Rust DTOs; the former central HTTP catalog and handwritten
schema generator have been removed. See [OpenAPI ownership](crates/server/src/openapi/README.md).

## Server startup ownership

The executable entry point in `crates/server/src/main.rs` calls `app::run()`.
The executable modules own composition; feature handlers remain in the server
library and receive their existing, narrowly scoped HTTP state.

- `app.rs`: tracing, application lifecycle, HTTP and edge gRPC serving, signals,
  worker supervision, and bounded shutdown of listeners, workers, and the pool.
- `cli.rs`: command parsing and the existing migration, recovery, diagnostics,
  healthcheck, and smoke commands. These do not build the full application.
- `state.rs`: explicit database/client/repository/service construction. Shared
  `AppState` is cloneable; owned worker inputs are returned separately because
  the service-account usage queue has a single receiver.
- `startup.rs`: schema migration, unattended bootstrap, and persisted readiness.
- `api/mod.rs`: feature routes, realtime and Swagger, edge gRPC, and middleware.
- `jobs/mod.rs`: registration of all jobs with the existing `TaskSupervisor`.
  Implementations remain in `workers/` and their owning crates.

Serving initializes tracing and loads configuration, migrates the schema,
builds dependencies, completes bootstrap/readiness, starts supervised jobs,
composes both routers, and starts both listeners. Migrations precede dependency
construction; bootstrap completes before jobs or requests can observe setup.
SIGTERM, Ctrl+C, server failure, or worker exit trigger the existing shared
cancellation and bounded shutdown. Auxiliary commands retain their individual
configuration requirements.

Run `cargo check --locked` and `cargo test --locked` from `rust/`. To exercise
actual startup, all documented route registrations, workers, restart, and Unix
signal shutdown, use a **disposable** PostgreSQL instance with CREATE DATABASE
permission:

```bash
CITADEL_TEST_DATABASE_URL=postgres://user:password@localhost:port/test_database \
  cargo test --locked -p citadel-server --test bootstrap_process -- --ignored --test-threads=1
```

These process tests use isolated databases, temporary data directories, ephemeral
HTTP/gRPC ports, and an absent Docker socket; they do not use development workloads.

## Unattended first run and recovery

For unattended Rust Core setup, supply all three settings:
`Bootstrap__AdminName`, `Bootstrap__AdminEmail`, and
`Bootstrap__AdminPasswordFile`. Mount the password file read-only and use its
absolute path inside Core. It must be a regular UTF-8 file of at most 1 KiB;
one trailing newline is allowed. Do not put the password itself in environment
variables. With no bootstrap settings, use the normal browser setup.

Partial settings fail startup without completing setup. Successful setup creates
one administrator and the two disabled example automations in one transaction.
It does not issue a browser session. After setup, these settings are ignored and
the password mount can be removed before restarting.

Offline `restore-system` requires the original `Jwt__Key` and
`Secrets__EncryptionKey` from the backed-up installation. Preserve these outside
the database backup. A missing/malformed encryption key is rejected before target
database work; a correctly sized **wrong** key is not detected by this preflight
and cannot decrypt restored secrets. Restore into a disposable environment and
verify the retained keys before replacing an installation.

## Interactive development

On Windows, use VS Code in WSL with the repository on the Linux filesystem,
running the Rust API and frontend directly and PostgreSQL in Docker Desktop.
The committed Dev Container remains available as a fallback. The complete setup,
run, debugging, test, shutdown, and troubleshooting commands are documented in
[DEVELOPMENT.md](DEVELOPMENT.md).

## Verification

Run workspace checks and tests from `rust/`:

```bash
cargo check --locked --workspace
cargo test --locked --workspace
cargo run --locked -p xtask -- openapi --check
cargo run --locked -p xtask -- database verify
```

HTTP routes are tested through Axum, persistence through disposable PostgreSQL,
and Docker/Agent/Edge behavior through the integration and acceptance harnesses.
See [scripts/README.md](scripts/README.md) for fixture requirements and commands.
[Agent protocol documentation](docs/agent-protocol.md) describes the wire contract,
deployment profiles and Swarm-node restrictions.

## Container build and Compose

Build the Rust server and frontend image from the repository root:

```bash
docker build -f rust/Dockerfile -t citadel-rust:local .
```

For a persistent local Compose installation:

```bash
cd rust
cp -n .env.example .env
# Fill the blank credentials and DOCKER_GID in .env (instructions are in the file).
docker compose config --quiet
docker compose up -d --build
```

The ignored `.env` holds database credentials, the JWT key, the persistent secret
encryption key, Docker socket group, image/version, and transport settings.
Keep the encryption key with your backups. The defaults bind HTTP to
`127.0.0.1:18000` and Edge gRPC to `127.0.0.1:18001`, leaving the WSL development
ports available. The image serves the frontend and API from the same HTTP port.

The server runs as UID 65532 with the Docker socket's supplementary group.
`citadel_data` persists `/app/data`; `postgres_data` persists PostgreSQL data.
These belong to the separate `citadel-rust` Compose project. Existing WSL and
measurement databases are not automatically reused or migrated. Use
`docker compose down` to stop it while retaining volumes.

`compose.measurement.yml` is an isolated measurement
fixture, including its 100 MiB memory cap and disposable database. Measurement
scripts use that file, not the persistent Compose installation.
