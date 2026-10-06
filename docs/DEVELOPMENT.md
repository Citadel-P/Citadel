# Development

Run commands from the repository root unless a section says otherwise.
See [Architecture](ARCHITECTURE.md) for code ownership and
[Agent protocol](agent-protocol.md) for transport behavior.

## Prerequisites

For Windows development, use VS Code in WSL Ubuntu and keep the checkout on the
Linux filesystem, for example `~/projects/Citadel`.

Install:

- Python 3.10+ and Git for shared build/release tooling.
- Rust through rustup; `rust-toolchain.toml` selects the required toolchain.
- Linux Node.js and npm. CI uses Node.js 24.
- Docker with Compose. On WSL, enable Docker Desktop integration for Ubuntu.
- Linux build dependencies: `build-essential`, `pkg-config`, `libssl-dev` and `ripgrep`.
- VS Code's WSL, rust-analyzer and CodeLLDB extensions for the WSL workflow.

Check that `cargo`, `node` and `npm` resolve to Linux executables. On WSL,
`docker info --format '{{.OperatingSystem}}'` should report `Docker Desktop`.
Use the same Docker socket for Core and the Docker CLI.

Install dependencies once, and again after lockfile changes:

```bash
cargo fetch --locked
npm ci --prefix src/frontend
```

The VS Code task **Citadel: Initialize development dependencies** does this too.

## Run Citadel

Open the checkout with `code .` from WSL, then press **Ctrl+Shift+B**.
The default **Citadel: Run application (API + UI)** task:

1. Creates `deploy/.env.development` if it does not exist.
2. Builds Core locally and regenerates the API contracts and frontend client.
3. Starts Core and PostgreSQL in Compose project `citadel-wsl`.
4. Starts Vite after Core becomes healthy.

| Service | Address |
| --- | --- |
| Frontend | <http://localhost:5173> |
| Core API | <http://localhost:8000> |
| Core liveness | <http://localhost:8000/health> |
| Edge gRPC | `localhost:8001` |
| Development PostgreSQL | `localhost:15432` |

Compose uses a local **release** binary by default. Set
`CITADEL_COMPOSE_PROFILE=dev` for faster, unoptimized builds. The runtime image
provides Docker CLI, Compose, Deno and backup tools; the executable is staged
outside Cargo's disposable build directory.

View logs with `docker logs -f citadel-wsl-core-1` or Docker Desktop.
To stop Core and PostgreSQL without deleting data:

```bash
bash src/tools/dev/dev.sh compose-stop
```

Terminate the Vite task separately through **Tasks: Terminate Task**. Closing
VS Code does not stop detached Compose services.

## Debug Rust

Stop the Compose services and Vite task, select **Citadel: Debug application**,
and press **F5**. This runs Core natively under CodeLLDB and starts Vite.
Use **Shift+F5** to stop both.

Do not run this alongside Ctrl+Shift+B: they share ports and Core allows only one
background-job owner per database. Native execution also needs any external tools
used by the feature being debugged, such as Docker CLI, Deno or Restic.

F5 compiles Core; it does not regenerate API contracts. After an API change, run
the [OpenAPI generator](#api-changes) before debugging the frontend.

## Run individual processes

Prepare the environment:

```bash
bash src/tools/dev/dev.sh prepare
```

Run Core:

```bash
bash src/tools/dev/dev.sh exec cargo run --locked -p citadel-server -- serve
```

Run the frontend in a second terminal:

```bash
cd src/frontend
bash ../tools/dev/dev.sh exec npm run dev -- --host 0.0.0.0 --port 5173 --strictPort
```

To start only the development Compose services, use
`bash src/tools/dev/dev.sh compose-up` from the repository root.

## Configuration and persistent data

| File or location | Purpose |
| --- | --- |
| `deploy/.env.development` | Local run/debug settings; created once and never overwritten by preparation |
| `deploy/.env` | Separate release-style Compose settings |
| `deploy/.env.example` | Core configuration template |
| `deploy/.env.*.example` | Direct and Edge Agent templates |
| `~/.local/share/citadel-wsl` | Default WSL runtime data, signing keys and staged Core executable |
| `citadel-wsl_postgres-data` | Development database volume |
| `target/` | Disposable Rust build output |

Environment files use plain `KEY=value` entries without shell expansion.
Local settings are ignored by Git. Core and Agent read process environment;
use the development launcher or Docker's `--env-file` to load a file.

The development Compose task requires its own PostgreSQL database. To use a custom
`DATABASE_URL`, run Core natively or through the debugger instead. Preserve signing
and encryption keys when reusing data or restoring a backup.

Useful settings:

| Setting | Use |
| --- | --- |
| `RUST_LOG` | Defaults to `info`; add `citadel_server=debug` for targeted diagnostics |
| `LogFormat` | `text` by default; `json` for structured collection |
| `CITADEL_DATA_ROOT` | Persistent runtime directory |
| `CITADEL_RUST_DOCKER_SOCKET`, `DOCKER_HOST` | Must select the same Docker daemon in development |
| `Git__KnownHostsPath` | Verified SSH host keys; defaults to `<CITADEL_DATA_ROOT>/git-known-hosts` |
| `Passwords__MinimumLength` | Defaults to 15; accepts 8–128 for newly set passwords |

Inspect non-secret effective configuration without connecting to PostgreSQL:

```bash
bash src/tools/dev/dev.sh exec cargo run --locked -p citadel-server -- print-effective-config
```

For deployment settings, use the [configuration guide](content/docs/getting-started/configuration.md).

## Checks

Run the checks relevant to your change:

```bash
cargo fmt --all -- --check
SQLX_OFFLINE=true cargo check --locked --workspace
SQLX_OFFLINE=true cargo clippy --locked --workspace --all-targets -- -D warnings
SQLX_OFFLINE=true cargo test --locked --workspace
npm run lint --prefix src/frontend
npm run test:run --prefix src/frontend
npm exec --prefix src/frontend -- tsc -b src/frontend/tsconfig.json
bash test/scripts/test-dev.sh
bash test/scripts/check-source-ownership.sh
```

`src/.sqlx/` holds offline query metadata; `.cargo/config.toml` supplies its path.
The default test run does not execute ignored tests that require external services.
Use the runners in [test/README.md](../test/README.md) for PostgreSQL, Docker, Agent
and browser coverage. Their fixtures must be disposable, not your working database
or application containers.

Build and export contracts without starting services:

```bash
bash src/tools/build/build.sh
```

## API changes

Change request/view/schema definitions in `src/server/`, then run:

```bash
cargo run --locked -p xtask -- openapi
```

This generates `schema/v1.json` and `schema/public-v1.json`, copies the full schema
to `src/frontend/src/api/schema/swagger.json`, and runs the frontend's
`api:generate` command. Install frontend dependencies first. The public schema
excludes internal endpoints; the frontend consumes the full schema.

Verify committed artifacts with:

```bash
cargo run --locked -p xtask -- openapi --check
npm run api:verify --prefix src/frontend
```

Do not hand-edit generated schemas or TypeScript models. Documentation-site
commands are in [docs/README.md](README.md).

## Database schema changes

Edit `src/infrastructure/database/src/schema/schema.sql`, then run:

```bash
cargo run --locked -p xtask -- database refresh-baseline
cargo run --locked -p xtask -- database verify
```

Until the first release, Citadel keeps a single generated `0001_initial.sql`
baseline. The generator also updates its manifest and embedded catalog.

Regenerating the baseline does **not** upgrade an existing database. An
`AppliedChecksum` failure means the applied baseline differs. Back up the data,
compare against a disposable database using the new baseline, and reconcile the
schema before updating its journal. Do not bypass validation or change a checksum
without verifying schema equivalence.

## Agent development

Core and all Agent profiles use the same Cargo workspace. Build the Agent image:

```bash
docker build -f Dockerfile.agent -t citadel-agent:local .
```

Use the installation command from the Platform or Build Pool page, replacing the
image with `citadel-agent:local`. Templates and profile requirements are listed in
[Agent protocol](agent-protocol.md#deployment-profiles). Persist Edge key and identity
files across restarts. The Agent does not use PostgreSQL.

`CITADEL_CORE_URL` must reach Core's **Edge gRPC port**, not the frontend or REST API.
For an Agent container on Docker Desktop, use `http://host.docker.internal:8001`.
For another machine, use Core's reachable IP/hostname and publish the Edge port.
Development Compose binds it to loopback by default; set
`CITADEL_DEV_EDGE_BIND_ADDRESS=0.0.0.0` in `deploy/.env.development` and recreate Core
when remote access is needed. `EdgeAgent__PublicGrpcUrl` configures Core's advertised
address. Use TLS outside local testing.

Agent checks, from least to most involved:

```bash
bash test/scripts/test-agent-image.sh citadel-agent:local
bash test/scripts/test-agent-compatibility.sh citadel-agent:local
docker build -f Dockerfile -t citadel-core:acceptance .
bash test/scripts/test-agent-acceptance.sh citadel-core:acceptance citadel-agent:local
```

These use disposable fixtures. See [test/README.md](../test/README.md) for prerequisites
and the optional released-Agent baseline used for mixed-version verification.

## Dev Container fallback

Use **Dev Containers: Reopen in Container** with Docker Desktop running. The
container installs Rust, Node.js and frontend dependencies. Run/debug Core natively
inside it; the default Ctrl+Shift+B Compose task expects the WSL host checkout.

The container uses `/var/run/docker-host.sock` and named volumes for dependencies,
PostgreSQL and runtime data. Rebuild after changing `.devcontainer/`. The cache
cleanup hook applies only to its dedicated Rust target volume and skips active
builds; it does not remove database or runtime data.

## Release-style local run

Copy `deploy/.env.example` to `deploy/.env` if needed, configure it, then run:

```bash
bash src/tools/build/with-version.sh docker compose --project-name citadel --env-file deploy/.env -f deploy/docker-compose.yml up -d --build --wait
```

The default application address is <http://localhost:18000>. This project has
separate volumes from `citadel-wsl`. The equivalent VS Code task is
**Citadel: Run release (Compose)**.

`version.json` supplies the product version. CI validates release tags against it;
`CITADEL_VERSION` and `CITADEL_INFORMATIONAL_VERSION` can override build metadata.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Ctrl+Shift+B never starts the UI | Inspect the Core task and container logs; Vite waits for Core health |
| Port 8000 or 5173 is busy | Stop the other Compose, debugger or Vite instance |
| Core rejects its job lease | Another Core process owns the same database |
| Docker is unavailable | Verify WSL integration, socket permissions and matching socket settings; do not make the socket world-writable |
| Edge connection times out | Check Core's reachable gRPC address, published port and firewall; container `localhost` is not the host |
| SQLx cannot find offline metadata | Confirm `src/.sqlx/` exists and run Cargo from the checkout so `.cargo/config.toml` is loaded |
| Cargo cannot open its build lock | Check target-directory ownership and active builds; avoid running Cargo with `sudo` |
| SSH Git clone rejects a host key | Provision a verified host key in `Git__KnownHostsPath`; do not disable verification |
| Host disk usage is unavailable | Mount the actual daemon host at `/host:ro`; the Docker socket alone cannot provide host filesystem usage |

Use `/health` for process liveness and `/ready` for dependency readiness. Agent
`/health` does not prove it is connected to Core or can reach Docker.

## Build version metadata

Local builds use the pinned native Rust Nerdbank.GitVersioning (NBGV) CLI.
The development launcher and build wrappers install it when needed. To install
and inspect it directly:

```bash
python3 src/tools/build/version.py install
python3 src/tools/build/version.py resolve
```

The installer uses the source revision and dependency lock checksum in
`src/tools/build/nbgv-pin.json`. It requires Git, Python, the pinned Rust toolchain
and the Linux build prerequisites listed above; no .NET SDK is needed. Its verified
binary is cached outside the checkout under `~/.cache/citadel-tools` by default.

`version.json` contains the three-part product version. Local builds add the Git
height and abbreviated commit, for example `0.1.0-dev.24.g0123456789ab`. Tracked
changes and non-ignored untracked files add `.dirty`; ignored build outputs do
not. These values identify the source used by a development build.

Use complete Git history. For a shallow checkout, run
`git fetch --unshallow --tags`. Commit changes to the product-version value before
resolving identity; configuration-only edits keep the committed product version
and mark the build dirty. Ordinary feature changes do not require a version bump.

Wrap commands that need resolved version metadata:

```bash
bash src/tools/build/with-version.sh cargo build --locked -p citadel-agent
```

The wrapper supplies `CITADEL_VERSION`, `CITADEL_INFORMATIONAL_VERSION`,
`CITADEL_SOURCE_REVISION` and `CITADEL_PRODUCT_VERSION`. Direct Docker builds accept
the corresponding build arguments `VERSION`, `INFORMATIONAL_VERSION`,
`SOURCE_REVISION` and `PRODUCT_VERSION`. The development and local Compose build
wrappers pass them through the existing Compose configuration.

Direct Cargo builds without supplied metadata deliberately display
`<product>-dev.unknown`. For a source archive or an intentional build without Git
identity, use the explicit fallback:

```bash
python3 src/tools/build/version.py exec --fallback -- cargo build --locked -p citadel-server
```

To check changes to version resolution or embedding, install the pinned tool and
run `python3 -m unittest discover -s test/release -p test_version.py -v`.
