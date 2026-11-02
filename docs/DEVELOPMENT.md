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
`CITADEL_DATABASE_URL`, run Core natively or through the debugger instead. Preserve signing
and encryption keys when reusing data or restoring a backup.

Useful settings:

| Setting | Use |
| --- | --- |
| `RUST_LOG` | Defaults to `info`; add `citadel_server=debug` for targeted diagnostics |
| `CITADEL_LOG_FORMAT` | `text` by default; `json` for structured collection |
| `CITADEL_DATA_ROOT` | Persistent runtime directory |
| `CITADEL_DOCKER_SOCKET`, `DOCKER_HOST` | Must select the same Docker daemon in development |
| `CITADEL_GIT_KNOWN_HOSTS_PATH` | Verified SSH host keys; defaults to `<CITADEL_DATA_ROOT>/git-known-hosts` |
| `CITADEL_PASSWORDS_MINIMUM_LENGTH` | Defaults to 15; accepts 8–128 for newly set passwords |

Inspect non-secret effective configuration without connecting to PostgreSQL:

```bash
bash src/tools/dev/dev.sh exec cargo run --locked -p citadel-server -- print-effective-config
```

This command reports deployment configuration. Administrators manage the persisted
update-check choice in **Settings → Updates → Config** and the MFA policy in
**Profile → Instance settings**.

For updater development, `citadel-server verify-update-release --version VERSION`
checks a completed official release manifest and its exact Core image signature.
The packaged Core includes Cosign; native runs require `cosign` on PATH. This
explicit diagnostic does not need a database or start Core. It honours
`CITADEL_UPDATE_CHECKS_ENABLED=false`, sends no instance credentials, and prints
only the verified version and digest reference. It does not establish upgrade
eligibility or perform installation. Automatic checks keep their existing setup
and policy gates.

`citadel-server upgrade-descriptor-json` prints the local build's version,
embedded migration-catalog SHA-256, installer/update protocol versions and
qualified source transitions. It needs neither configuration, network access nor
a database connection. The source list is currently empty: no automatic upgrade
path is qualified. Release acceptance compares this output from both Core image
architectures with the tagged checkout; it does not enable update execution.

On Linux, `citadel-server check-update-compose` checks the configuration portion
of updater registration using the Compose CLI packaged in Core:

```bash
citadel-server check-update-compose \
  --directory /opt/citadel --project citadel \
  --compose-file /opt/citadel/docker-compose.yml
```

Supply the actual original project name and canonical host paths. If the original
invocation used the Edge overlay, append
`--compose-file /opt/citadel/compose.edge.yml`. A container running this diagnostic
needs the installation directory mounted read-only at that same path and a user
that can read its private files. It needs no Docker socket or database access.
Do not relax file permissions to make the check pass.

The Docker Compose adapter accepts one to four distinct `.yaml`/`.yml` files
directly inside the installation directory, in their original invocation order.
It validates configuration rather than comparing files with embedded templates.
Comments, filenames, labels, published Core ports and explicit volume/network names
can differ from the examples. The current profile still requires `server` and
`pg_db`, the `citadel_data`/`postgres_data` volume roles, the default bridge network,
and the supported persistent mount paths. An installation kind identifies this
profile as Docker Compose; Podman and Kubernetes are not implemented.

Settings use a private `.env` containing unique `NAME=value` assignments without
variable interpolation. Service `env_file` references must point to that same
`.env` with `format: raw`. Includes, extends, builds, external configuration/secret
files, YAML merge/tag extensions and unrecognized fields are rejected before
Compose runs. External databases, remote Core Agents, additional services and
storage-root overrides are not supported. It resolves captured bytes in a private
temporary directory, rejects input changes during the check, and prints only
the project, configuration fingerprint and desired image/volume/network names.
Errors omit Compose output because it may contain secrets. It does not save a
registration or prove that running containers match these settings.

The next step for a current-build development installation is the Linux bootstrap
command below. The state directory must already exist, be mode `0700`, and belong
to the user intended to execute updates. Use a dedicated host path outside Docker's
data directory and the Citadel volumes. The command requires local Docker socket
access, a healthy single-Core Compose project and completed instance setup:

```bash
citadel-server register-update-installation \
  --directory /opt/citadel --project citadel \
  --compose-file /opt/citadel/docker-compose.yml \
  --state-directory /opt/citadel-update-state
```

Supply the original ordered Edge overlay here too, when used. This Rust command
is also included in the Core image; no host Python installation is required.
For containerized execution, use Docker's default hostname and mount exactly the
installer directory read-only, the private state directory read-write and
`/var/run/docker.sock`, each at its original host path. Run as the intended updater
user with socket access and permission to read the protected installer files.
Use `--entrypoint /app/citadel-server` to invoke the diagnostic directly: the normal
Core entrypoint switches root to the Core service UID. Set `--user UID:GID` to the
owner of the private installer inputs and state; add the socket's group for a
non-root executor when needed. Do not change those files' permissions to bypass
an ownership mismatch.
The command verifies these mounts against its own container on that daemon.
The registration container needs no network attachment: the database identity diagnostic executes in the
existing Core container, using Core's existing database configuration.

The bootstrap compares running services and storage with the supplied inputs,
reads the initialized database identity without migrations or writes, and creates
`installation.json` with mode `0600`. The file records the installation kind, identities and fingerprints;
it contains no environment values. Unknown kinds or malformed registration are
refused without replacing existing evidence. Identical retries preserve its ID. Drift,
conflicting registration, corrupt files and orphaned recovery evidence cause a
refusal without overwriting state. It does not stop services, approve an update,
establish source-to-target compatibility or enable automatic installation.
The initial profile requires the Compose network and Citadel data volumes to be
used only by these two services; additional consumers are rejected as ambiguous.
Automatic installer integration, registration reconciliation, startup maintenance
and detached execution are still pending.

Validate bootstrap changes with the isolated Docker acceptance command in
[`test/README.md`](../test/README.md). It exercises the shipped layout and a compatible customized Edge-overlay installation
and containerized registration with completed HTTP setup. This is a registration
gate; it does not qualify a source-to-target upgrade or recovery path.

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

Run generation and both verification commands sequentially. Finish them before
frontend tests, lint, or TypeScript checks: `api:verify` temporarily regenerates
outputs and restores the original files, so concurrent checks can observe missing
or intermediate generated files.

Do not hand-edit generated schemas or TypeScript models. Documentation-site
commands are in [docs/README.md](README.md).

## Frontend contributor path

Use [Registries queries](../src/frontend/src/features/registries/queries.ts) as a bounded
example of typed query keys/options and explicit mutation effects. Its
[list](../src/frontend/src/features/registries/index.tsx),
[actions](../src/frontend/src/features/registries/actions.tsx), and
[form](../src/frontend/src/features/registries/form/form.tsx) consume that contract.
Generic reads and mutations accept one named argument object: path parameters,
`query`, `data`, and transport `params` remain distinct. Generated metadata follows
the actual client signatures; the invocation adapter alone orders arguments.
Transport options, tokens and signals never identify cached data. Reads wait for
required route/query arguments and retain React Query cancellation.

Registry updates explicitly invalidate filtered lists and the affected configuration
key. Creation refreshes lists; deletion also invalidates affected configuration keys.
Keep related-resource effects in the feature: OpenAPI tags cannot define a universal
invalidation policy. Preserve actor cache isolation on account changes. Capabilities
from authorized responses control action/form availability; server authorization
remains authoritative. Shared resource page templates remain under `pages/`.

Forms use the existing `FormShell`, field definitions and `useSaveResource` lifecycle.
Keep draft keys/versioning and actor scoping, sensitive-field exclusions, dirty state,
validation, successful-save cleanup and failure recovery. The Registry form's
`registrySettings`/`registryUpdate` mappings preserve credential patch semantics.
Use [the Registry query regression](../src/frontend/src/features/registries/queries.test.tsx)
for real-client/MSW coverage of requests, filtered keys, scoped invalidation and
success callback timing; retain form/draft tests when changing form behavior.

Realtime supports the advertised `WebSocketV1` contract. Disabled, unresolved and
unknown capabilities do not connect. Tests may inject a `RealtimeConnection` for
that supported transport; native-adapter tests supply only a `webSocketFactory`.
HTTP queries own ordinary reads; live subscription snapshots own their existing
live-backed caches. Keep explicit reconnect reconciliation, group ownership and
stale callback rejection instead of refetching every query. Swarm HTTP and live
snapshots use the same server overview projection. Complete live overviews cancel
older HTTP reads; partial node-local events retain omitted fields and allow the
initial complete read to finish. Aggregate reads may be shared per event, while
capabilities remain actor-specific.

`npm run api:generate` synchronizes `schema/v1.json` into the frontend schema,
generates the client and resource metadata, and owns `src/api/generated/` except
`foundation-api.ts`. That file belongs to Rust `xtask openapi` and its `--check`.
`npm run api:verify` regenerates all npm-owned outputs, checks file membership and
bytes, and restores the original working-tree contents even on failure. It detects
new, missing, edited and unexpected outputs without staging changes. After an
intentional generation, run verification again to check deterministic output.
Generated schema/client files are excluded from Prettier; their generators verify
them. Authored components are included in ESLint and the frontend formatting check.

Run from `src/frontend` (install with `npm ci` first):

```bash
npm run api:generate
npm run api:verify
npm run test:run -- src/features/registries/queries.test.tsx src/lib/hooks.test.tsx
npm run test:run -- src/test/architecture.test.ts
npx tsc -b
npm run lint
npm run prettier:verify
npm run test:run
npm run build:image
```

CI runs generation verification, TypeScript, lint, formatting and tests. The image
build also runs lint. For Swarm projection changes, run the focused server tests
with a disposable PostgreSQL fixture as described in [test/README.md](../test/README.md).

### Dependency security updates

Audit both lockfiles and packaged runtime tools when updating dependencies:

```bash
cargo audit --file Cargo.lock
npm audit --prefix src/frontend
npm audit --prefix docs
npm audit --prefix test/e2e
npm run test:monaco --prefix src/frontend
trivy image --scanners vuln citadel-core:acceptance
trivy image --scanners vuln citadel-agent:local
```

Monaco embeds its own DOMPurify source. The frontend's Vite alias replaces that
import with an isolated instance of the patched npm dependency, preserving
Monaco's hook lifecycle. The production builds run `test:monaco` to check the
actual module graph and sanitized editor markup; updating the npm override alone
does not replace Monaco's embedded copy.

Keep Core and Agent image pins aligned, and update PostgreSQL pins in deployment
and disposable test fixtures together while retaining the database major version.
The Core installer upgrades inherited Alpine packages as well as installing its
tools. Run the runtime and Agent image checks after changing these dependencies.
Scanning an upstream image can still report dependencies with no patched release
or unreachable functions. Record those separately from fixes; do not suppress
findings globally or replace official tools with untested forks. Rust's audit policy is in `.cargo/audit.toml`; no vulnerability exceptions are
currently configured.

## Database schema changes

The migration catalog starts with `0001_initial.sql` and appends numbered upgrade
migrations. The initial migration includes workload
statistics, alert observation ordering, secret-use grants, and build, backup,
restore and script execution ownership/capacity changes.

This consolidated baseline requires a fresh database. It cannot upgrade a database
that recorded an earlier baseline checksum or the former `0002`–`0011` sequence.
Recreating a database is a separate, explicit operator action; development tasks
never reset storage.

The script-execution resource is now `Script`, with `/api/v1/scripts` routes,
`scripts` and `scriptruns` tables, and `CITADEL_SCRIPTS_*` Core settings. Update
Core, Agents, and the frontend together; the previous Automation API, Agent
protocol names, and configuration keys are not aliases. Existing pre-release
data needs an explicit conversion when restoring into the new baseline.

For an explicitly authorized pre-release baseline reset, restore captured product
records into a fresh database while retaining its new migration journal and the
original encryption/signing keys. If the capture predates `secretusegrants`, run
the baseline's marked `RESTORE SECRET USE GRANTS` block after loading records;
otherwise retain captured grants exactly, including revocations. Rehearse the
import before cutover. Normal deployments must continue to preserve applied
migration checksums.

For the current catalog, preserve applied migrations and add each schema change
as the next numbered SQL migration under `src/infrastructure/database/generated/`.
Append the same SQL to `src/infrastructure/database/src/schema/schema.sql`; the
verifier checks it against the ordered migration SQL. Then regenerate the catalog:

```bash
cargo run --locked -p xtask -- database refresh-catalog
cargo run --locked -p xtask -- database verify
```

Test fresh installation and upgrades against separate disposable databases.
`refresh-baseline` only supports a single unpublished initial migration; it is not
an upgrade path for this catalog. Never rewrite applied migration journals
or checksums to suppress an `AppliedChecksum` error.

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
when remote access is needed. `CITADEL_EDGE_PUBLIC_GRPC_URL` configures Core's advertised
address. Use TLS outside local testing.

Agent checks, from least to most involved:

```bash
bash test/scripts/test-agent-image.sh citadel-agent:local
bash test/scripts/test-agent-compatibility.sh citadel-agent:local
docker build -f Dockerfile -t citadel-core:acceptance .
bash test/scripts/test-agent-acceptance.sh citadel-core:acceptance citadel-agent:local
```

These use disposable fixtures. See [test/README.md](../test/README.md) for prerequisites
and current-build acceptance coverage. Core, Agent and frontend must use the same
current build; no older-version image baseline is required.

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
| SSH Git clone rejects a host key | Provision a verified host key in `CITADEL_GIT_KNOWN_HOSTS_PATH`; do not disable verification |
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

### Execution capacity and retained ownership

The initial schema includes build dispatch ownership, renewable backup
attempt/source/repository ownership, shared daemon capacity for data helpers,
restore admission backoff, and scripts executor reservations. A terminal run
or expired lease does not prove remote work stopped. Cleanup receipts retain
same-authority evidence; uncertain work remains blocked until verified.
Let active builds, backups, restores and repository operations finish before
an authorized database rebuild. Preserve unresolved ownership records during
restoration; never invent a live attempt or cleanup proof.

Backup renewal runs every 30 seconds; configured repository/source lease durations
have a 90-second minimum. Expiration permits interruption and cleanup investigation,
not replacement ownership. Never clear leases as a general startup cleanup.
For uncertain data work, inspect `dataexecutorreservations` together with its
`ownerid`/`attemptid`, the run logs, `backuprepositoryleases` and
`backupsourceleases`. Local Docker reservations use the current Core process
as their authority; Direct/Edge reservations use the Agent process authority.

The automatic data sweeper only accepts cleanup from the original authority on
the original daemon and node. After an authority restart, manual recovery requires
stopping dispatch, proving the old helper/Restic work stopped on that exact host,
reviewing any partial restore, and recording the investigation before a targeted
transaction releases that owner's host/repository/source records. Never bulk-delete
expired leases, release a live owner's records, fabricate Agent receipts or
rerun a restore because its connection disappeared. There is no backup cleanup
release button in this implementation. Keep uncertain holds if proof is unavailable.
