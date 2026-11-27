# Citadel development

This guide covers setup, daily commands and generation. Design rules belong in
[Architecture](ARCHITECTURE.md), fixture requirements in [Tests](../test/README.md),
and production procedures in the [operator guide](content/docs/operations/upgrade-and-rollback.mdx).
Run commands from the repository root unless stated otherwise.

## Prerequisites

Use Linux or WSL Ubuntu with the checkout on the Linux filesystem. Install Git,
Python 3.10+, rustup with the pinned `rust-toolchain.toml`, Linux Node.js 24/npm,
Docker with Compose, and native build dependencies (`build-essential`,
`pkg-config`, `libssl-dev`, `ripgrep` on Ubuntu). Core and the Docker CLI must use
the same daemon/socket. On WSL, enable Docker Desktop integration for the distro.

```bash
cargo fetch --locked
npm ci --prefix src/frontend
```

Repeat after lockfile changes. For VS Code, use the WSL extension, rust-analyzer
and CodeLLDB; open the checkout with `code .` from WSL.

## Run Citadel

Press **Ctrl+Shift+B** for **Citadel: Run application (API + UI)**. It creates
`deploy/.env.development` when absent, builds Core, regenerates API/client/SDK
contracts, starts Compose project `citadel-wsl`, then starts Vite after health.

| Service | Local address |
| --- | --- |
| Frontend | <http://localhost:5173> |
| Core API / liveness | <http://localhost:8000> / <http://localhost:8000/health> |
| Edge gRPC | `localhost:8001` |
| PostgreSQL | `localhost:15432` |

Compose stages a local release binary; set `CITADEL_COMPOSE_PROFILE=dev` for faster,
unoptimized builds. The runtime image supplies Docker CLI, Compose, Deno and
backup tools. View logs with `docker logs -f citadel-wsl-core-1`.

```bash
bash src/tools/dev/dev.sh compose-stop
```

This preserves data. Terminate Vite separately through **Tasks: Terminate Task**;
closing VS Code does not stop detached containers.

## Debug Rust

Stop Compose Core first, then use **F5** for native CodeLLDB debugging. Native and
Compose Core share ports and the database background-job lease, so do not run
both. Native execution needs the external CLIs that the runtime image supplies.
Generated contracts refresh at startup, not continuously; restart after API changes.

## Run individual processes

```bash
bash src/tools/dev/dev.sh prepare
bash src/tools/dev/dev.sh exec cargo run --locked -p citadel-server -- serve
```

In another terminal:

```bash
cd src/frontend
bash ../tools/dev/dev.sh exec npm run dev -- --host 0.0.0.0 --port 5173 --strictPort
```

For Compose Core alone, run `bash src/tools/dev/dev.sh compose-up`. Plain Cargo or
Vite commands do not regenerate contracts automatically. Development Compose uses
a mounted executable and is not an in-app updater installation; use the
[isolated updater fixture](../test/README.md#browser-and-full-lifecycle-postgresql-upgrades).

## Configuration and persistent data

| Location | Purpose |
| --- | --- |
| `deploy/.env.development` | Ignored development configuration, created once from the example |
| `deploy/.env` | Separate release-style Compose configuration |
| `~/.local/share/citadel-wsl/` | Default development runtime state, keys and staged executable |
| `citadel-wsl_postgres-data` | Development PostgreSQL 19 volume |
| `target/` | Disposable Cargo output, not application state |

Environment files contain literal `KEY=value` entries, without shell expansion.
Use `dev.sh exec` to load development settings. Compose selects its own PostgreSQL
service; use native/debug mode for a custom database URL. Preserve encryption and
signing keys alongside stored data. Never reset volumes to fix a build or migration
failure.

```bash
bash src/tools/dev/dev.sh exec cargo run --locked -p citadel-server -- print-effective-config
```

For setting meanings and restart requirements, see the
[environment reference](content/docs/reference/environment-variables.mdx).
`CITADEL_DOCKER_SOCKET` and CLI `DOCKER_HOST` must select the same daemon.
Git SSH requires verified known hosts; do not disable verification.

## Checks

Choose focused checks from [test/README.md](../test/README.md). For broader changes:

```bash
cargo fmt --all -- --check
SQLX_OFFLINE=true cargo check --locked --workspace
SQLX_OFFLINE=true cargo clippy --locked --workspace --all-targets -- -D warnings
SQLX_OFFLINE=true cargo test --locked --workspace
npm run lint --prefix src/frontend
npm run test:run --prefix src/frontend
npm exec --prefix src/frontend -- tsc -b src/frontend/tsconfig.json
```

`SQLX_OFFLINE=true` uses checked-in query metadata; it does not isolate runtime
tests from databases. Ignored integration tests require separate disposable
fixtures. Use `bash src/tools/build/build.sh` for the local build workflow.

## API changes

Rust Server's `utoipa` definitions own the API. Regenerate OpenAPI, the frontend
client and script SDK together:

```bash
bash src/tools/build/generate-api.sh
cargo run --locked -p xtask -- openapi --check
npm run api:verify --prefix src/frontend
```

The wrapper runs the generator sequence and serializes checkout writes. Outputs
include `schema/v1.json`, `schema/public-v1.json`, frontend Swagger/resources and
`src/infrastructure/scripts/generated/citadel.js`. Review and commit generated
changes; never edit them by hand.

Run generation and verification sequentially and finish before frontend tests,
lint or type checks: `api:verify` temporarily regenerates/restores files. The
`xtask openapi` command owns Rust contract generation; the wrapper also handles
client/SDK generation. A running frontend task is not a substitute for verification.

## Frontend contributor path

Use the Registry feature's queries, list, actions and form as a reference. Register
screens through the resource factories, reuse FormShell/FormBuilder and shared
selectors, and follow existing React Query/realtime invalidation. Add forms use
the resource type’s Write grant from the current profile. Docker and Swarm
inventory forms use the selected platform’s Write grant. Declare an override at
form registration only when the create endpoint uses a different permission type
or requires an administrator.

API calls use named argument objects; path/query/data parameters remain distinct.
Transport tokens and abort signals are not cache-key data. Preserve required-input
gates, cancellation, actor isolation, draft identity/version and secret exclusions.
Mutation responses may update caches while disconnected, but must cancel stale
reads first. Reconnect reconciliation belongs to the shared realtime flow.

See [Architecture](ARCHITECTURE.md#frontend-and-generated-contracts) for boundaries;
run affected component tests and the shared architecture test when dependencies
change. Avoid adding separate polling or refresh-all behavior.

## Database schema changes

Preserve applied migrations. Add the next numbered SQL migration under
`src/infrastructure/database/generated/` and append the same SQL to
`src/infrastructure/database/src/schema/schema.sql`; verification compares the
schema with the ordered migration SQL.

```bash
cargo run --locked -p xtask -- database refresh-catalog
cargo run --locked -p xtask -- database verify
```

Test fresh installation and populated upgrades on separate disposable databases.
`refresh-baseline` is only for a single unpublished initial migration, not an
upgrade path. Never rewrite migration journals or checksums to suppress an
`AppliedChecksum` error. An older pre-release database with a replaced baseline
requires an explicit, rehearsed data-preserving conversion; normal development or
deployment does not authorize resetting it. Preserve its keys and captured grants.

## Agent development

```bash
docker build -f Dockerfile.agent -t citadel-agent:local .
```

Use the Platform/Build Pool installation command with this image. Follow
[Agent deployment profiles](agent-protocol.md#deployment-profiles); persist Edge
keys/identity across restarts. Agents do not use PostgreSQL. Core, Agent and
frontend must use the same current build.

`CITADEL_CORE_URL` must reach **Edge gRPC**, not the frontend or REST API. For an
Agent container on Docker Desktop, use `http://host.docker.internal:8001`.
Development Compose binds Edge to loopback; for remote tests, set
`CITADEL_DEV_EDGE_BIND_ADDRESS=0.0.0.0` and recreate Core.
`CITADEL_EDGE_PUBLIC_GRPC_URL` controls the advertised address. Use TLS outside
isolated local testing. Image, compatibility and acceptance commands are in
[Tests](../test/README.md#docker-and-agent-tests).

## Dev Container fallback

The Dev Container supports native execution with its provided dependencies,
PostgreSQL and runtime volumes; the WSL Compose launch task is not its entry point.
Its Docker socket is `/var/run/docker-host.sock`. Stop builds before cleaning
build caches; remove only dedicated cache volumes, never database/runtime state.

## Release-style local run

Create `deploy/.env` from its example if absent, then:

```bash
bash src/tools/build/with-version.sh docker compose --project-name citadel --env-file deploy/.env -f deploy/docker-compose.yml up -d --build --wait
```

This uses <http://localhost:18000> and separate volumes from development. The
**Citadel: Run release** VS Code task provides the same workflow. In-app update
setup/recovery belongs in the [operator guide](content/docs/operations/upgrade-and-rollback.mdx),
not the development environment.

## Build version metadata

`version.json` owns product version identity. The Python resolver downloads the
checksum-pinned native NBGV tool without requiring .NET:

```bash
python3 src/tools/build/version.py install
python3 src/tools/build/version.py resolve
```

Use `with-version.sh` around builds that need resolved metadata. It supplies the
version arguments used by Docker/native builds. Plain Cargo otherwise uses the
fallback development identity. Use complete Git history/tags; dirty state includes
untracked, nonignored files. Ordinary feature changes do not bump product versions.
Commit intentional version changes before resolving publication identity.

For publishing, signed artifacts, retries and environment deployment, follow
[release publication](release-publication.md) and the
[release workflow](../.agents/skills/citadel-release-workflow/SKILL.md).

## Dependency security updates

Use `cargo audit --file Cargo.lock` and `npm audit` in the affected frontend, docs
or E2E package. Scan built Core/Agent images with Trivy when changing their layers.
Keep paired image pins aligned; do not change PostgreSQL major versions incidentally.
Document narrowly scoped exceptions rather than suppressing advisories globally.

Monaco's embedded DOMPurify uses a patched isolated dependency; run the frontend
`test:monaco` script when updating it. Audit exceptions in `.cargo/audit.toml` must
remain valid for the enabled dependency graph.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Port conflict or background lease held | Stop the other native/Compose Core; do not clear the database lease blindly |
| Docker works in CLI but not Core | Compare socket, daemon identity and permissions |
| Edge Agent cannot connect | Check gRPC port, bind/advertised address, TLS and persistent identity |
| SQLx compile error | Use checked-in offline metadata or regenerate through the database workflow |
| Migration checksum mismatch | Preserve storage/history and reconcile the catalog; do not reset volumes |
| Build lock or stale generated client | Stop competing builds/generation; rerun the sequential API workflow |
| Git SSH failure | Configure verified known hosts and private credential permissions |
| Container bind mount missing | Paths must exist on the Docker daemon host, not only inside Core |

`/health` is liveness; `/ready` checks dependency readiness. Agent process health
alone does not prove Core connectivity. Terminal/expired execution ownership does
not prove helpers stopped: investigate the original attempt and authority before
using explicit recovery. See [execution invariants](ARCHITECTURE.md#execution-and-recovery)
and the [troubleshooting guide](content/docs/operations/troubleshooting.mdx).
