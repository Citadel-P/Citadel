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
2. Builds Core locally and regenerates the API contracts, script SDK and frontend client.
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

Script execution uses the checked-in `src/infrastructure/scripts/generated/citadel.js`
SDK in native Cargo runs. Development startup regenerates it with the API contracts;
`openapi --check` checks it alongside
the API artifacts. Container runtime stages package the SDK at `/app/scripts/citadel.js`.
Standalone binary distributions must keep `scripts/citadel.js` beside the executable.
Each execution still uses its own private source, credentials and Deno cache.

F5 regenerates API contracts, the script SDK and frontend models before compiling
Core. `npm run dev` also regenerates them before starting Vite, including when run
outside VS Code. Frontend development therefore needs the Rust toolchain and the
frontend dependencies installed. Generation failures stop startup.

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

This development Compose setup mounts a locally built Core executable and is not
registered for in-app Core or PostgreSQL updates. Sharing a Compose project does
not enable updater registration. To test replacement in the browser, use the
[isolated updater fixture](../test/README.md#browser-and-full-lifecycle-postgresql-upgrades).

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
update-check choice in **Administration → Updates → Config** and the MFA policy in
**Administration → Instance**.

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

On Linux, inspect the descriptor from a verified published target with:

```bash
citadel-server inspect-update-release --version 1.2.3 --architecture amd64
```

Use an actual completed release version and `amd64` or `arm64`. This operator
diagnostic requires Cosign, a Docker CLI supporting `image inspect --platform`
and a daemon supporting API 1.49 or newer, plus access to `/var/run/docker.sock`;
`CITADEL_UPDATE_CHECKS_ENABLED=false` also prohibits it. It verifies the signature
before pulling the immutable image and runs a disposable descriptor-only container
without installation mounts or network access. It returns JSON containing the
target identity and embedded descriptor. It does not start Core services, migrate
a database, register an installation, save a review or install the release.
The target architecture must be executable by the daemon; emulation is not configured.
Both pull and image inspection explicitly select that architecture, including when
multiple platforms of the same release are cached.

Cancellation waits for bounded probe cleanup. Failures return no successful JSON;
a cleanup failure names the probe requiring inspection. A process/host crash or a
late Docker create after a timeout can leave an unstarted probe labelled
`com.citadel.update-inspection`; inspect that exact probe, rather than removing
installation containers. Pulled images remain cached. Successful inspection proves
target metadata, not a qualified source-to-target transition or recovery readiness.

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
Compose runs, except `!override` on `services.server.volumes`: the complete resolved
mount list must still match the supported profile. External databases, remote Core Agents, additional services and
storage-root overrides are not supported. It resolves captured bytes in a private
temporary directory, rejects input changes during the check, and prints only
the project, configuration fingerprint and desired image/volume/network names.
Errors omit Compose output because it may contain secrets. It does not save a
registration or prove that running containers match these settings.

The supplied Compose installation starts with `docker compose up -d`. It sets the
internal update-state mount and registers automatically after administrator setup.
Core discovers the host directory and ordered files from Docker Compose labels,
then uses the existing restricted host helper. A pending registration is atomically
replaced with verified registration; no containers are recreated and `.env` is
not modified. See [the product setup guide](content/docs/operations/upgrade-and-rollback.mdx#enable-in-app-updates).
The optional `enable-updates.py` utility remains for supported custom deployments
without automatic setup. Lower-level commands remain available for diagnostics
and operator recovery.

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
the Docker socket. Installer and state mounts keep their original host paths; the
socket is always mounted at `/var/run/docker.sock` inside the executor. For rootless
Docker its host source is `/run/user/<uid>/docker.sock`. Core must use that same
socket mapping and mount the daemon’s `DockerRootDir` read-only at `/host` plus its
absolute path, instead of mounting the whole host root. Only these mounts and the
Core data volume are supported. Run as the intended updater
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

For UI installation, use the root executor (`0:0`) **inside a container**. On
rootless Docker this maps to the daemon’s unprivileged host user; installer inputs
and state must be owned by that user on the host. Do not use its host UID as the
container UID or chown the files to host root. The registered host socket source,
daemon identity and data-root mount are rechecked before replacement and recovery.
First register the healthy
installation, then add `CITADEL_UPDATE_STATE_DIRECTORY=/opt/citadel-update-state`
to its existing `.env` and recreate Core with the original Compose invocation.
After Core is healthy, repeat `register-update-installation` with `--refresh` and
the same paths. This explicitly accepts the changed Core container/configuration
while preserving the instance, daemon, database, volumes, network and registration
ID. Refresh requires an idle, healthy installation with a verified catalog; active
or orphaned recovery evidence blocks it. It is never an escape from maintenance.
Do not enable the state-directory setting before the initial registration exists.

To revalidate an existing registration without changing it, use:

```bash
citadel-server check-update-installation --state-directory /opt/citadel-update-state
```

Use the same executor UID/GID and mounts as registration. The command derives the
Compose invocation from protected registration, takes the existing nonblocking
installation lock, and compares fresh configuration, daemon, container, storage,
network and instance observations with the saved values. It returns JSON with
`registration`, `database`, `recoveryKeys` and `checkpointStorage`; no registration or
lock file is created or repaired. Edited
inputs, changed resources, lock contention or recovery evidence cause refusal.
The lock is released on success, failure or cancellation. Release-network policy
does not disable this local check. It invokes
`instance-identity-json --verify-migrations` inside the source Core to read initialized
identity, update ownership, the migration journal and database size. Missing,
incomplete, failed, unknown or checksum-mismatched migrations fail closed without
changing migration history. A retained update owner also blocks the check.

It also invokes `check-update-recovery-keys` inside Core as its packaged service
user (65532:0). This read-only diagnostic uses existing key files or explicitly
configured JWT/encryption keys; it does not initialize Core or create replacements.
It checks availability, encoding and length, with bounded private file reads and
no symlink traversal. Its JSON reports only `DataVolume` or `Environment`
for JWT, secret encryption and Agent signing. External keys must be represented
in the protected installer inputs and match the running container environment.
Neither values nor key fingerprints are returned. Preserve the original keys;
passing this check is not proof of successful decryption or an independent backup.

Checkpoint storage requires available space of twice the current database size
plus 64 MiB in the registered state directory. A private 4 KiB file is written,
synchronized and removed to test executor access. A crash can leave a
`.checkpoint-probe-*` file; preserve the state directory and inspect that exact
probe before retrying. This budget does not reserve space or guarantee that a
future dump fits.

This supplies database and storage evidence for preflight; it does not qualify a
target transition, create or restore-test a checkpoint, save an update review or
permit replacement. Its result is a
point-in-time observation; the executor must revalidate under ownership before
any future replacement.

The initial profile requires the Compose network and Citadel data volumes to be
used only by these two services; additional consumers are rejected as ambiguous.
Registration remains an explicit operator step. Registered Linux Compose
installations now have review/approval endpoints and a supervised execution path.
Only a signed target descriptor that explicitly qualifies the source version,
catalog, architecture and Agent versions permits a review. Packaged transition
qualification remains required before publishing an eligible transition.

Live update admission has a shared process-local gate wired into HTTP and worker
dispatch. `UpdateDrainService` validates a committed claim with the PostgreSQL
reader, closes the gate through the Server runtime port, and observes local tasks
and durable operations/reservations within a deadline. It does not mutate journal
history, release ownership or create checkpoints. Run it under a separate supervised
owner, not inside the `DynamicTasks` tracker it observes. The installation worker invokes this step after the driver's acknowledged
journal has been imported. All writers must be quiescent before checkpoint creation. See the admission and
owned-drain regression commands in [`test/README.md`](../test/README.md).

`prepare_update_checkpoint` is an internal helper building block, not a CLI or HTTP
action. It requires the held installation lock and matching backup-phase journal,
preserves installer inputs, reuses the system recovery builder and durably publishes
a preparation receipt after integrity checks. It never initializes database
identity, advances the journal or resumes Core. Failed/interrupted attempt
directories are evidence, not disposable cache. The caller must finish draining
and stop source writers first; the detached helper driver enforces those steps
before calling this primitive. Run the checkpoint regressions with dedicated disposable source
and restore databases as described in [`test/README.md`](../test/README.md).

`verify_checkpoint_integrity` rechecks the operation, registration, saved installer
inputs and recovery-bundle checksums immediately before replacement. It performs no
Docker mutations and does not restore the archive. Runtime updates need no restore
sandbox, restore receipt or sandbox resource budget. Validate actual restoration
with the shared recovery tests and the disposable checkpoint round trip in
[`test/README.md`](../test/README.md); never describe integrity checks as restore tests.
Target startup still validates keys, migrations and stored credential decryption.

The detached updater has three entry points: `launch_update_helper` in the
registered executor namespace, `run_update_helper` in the pinned source helper,
and `TargetUpdateSession` in replacement Core startup. Individual execution phases
are private to the adapter. Core invokes the launcher through a short-lived `update-host` utility from its
actual installed image. Only installer inputs, private state and the local socket
are bound into the utility; temporary files use a separate tmpfs. Its exact ID,
image, labels, command, mounts and privilege profile are checked before execution
and cleanup. No Python installation or embedded Compose template is required.

The helper driver owns its lock and journal for preparation, authorized shutdown,
checkpoint creation and integrity verification, Core-only replacement and completion.
`UpdateInstallService`
waits for prepared-helper readiness, then drains before granting shutdown. Preparation
must have published its fresh target/Agent/configuration evidence before a grant can succeed.
Core and target both use the same bounded `update-helper-handoff` Docker-exec
command, with typed actions and exact operation/helper identity. Only the driver
writes the complete execution journal; command handlers publish grants or completion
mailboxes. Read-only pending responses share one bounded, cancellation-aware wait.
No request executes replacement, retries a stop or releases maintenance.

Post-publication validation is shared by replacement observation, target startup
and completion. A replacing operation may return pending while Compose starts the
target before the helper binds it. Bound targets require unchanged configuration,
checkpoint, PostgreSQL/storage/network and container/start identity. Readiness can
still fail in maintenance; Docker health is not update success.

Target startup validates the source catalog and recovery keys, runs supported
migrations, verifies the target catalog and credential decryption, and commits
completion under the monitored single-Core lease. The helper asks the target's
read-only `update-target-receipt` command to verify that committed receipt before
acknowledging completion and exiting. No source-schema queries run after replacement.
Host reconciliation and final admission are connected to target acceptance.
A durable finalization record permits interrupted cleanup and archive moves to finish
after validating the committed target again; replacement is never replayed. A complete
archive can finish an interrupted database release. Failures before committed
completion retain maintenance for explicit operator recovery. Only qualified release transitions can be approved.
Helper acknowledgement, preparation, stop,
replacement and target binding now live with the phase history in one atomic journal.
The launcher still owns launch intent/start binding, and other processes publish only
the shutdown grant/completion mailbox. Standalone checkpoint journals, database
claims and migration history retain their formats. Existing fragmented operation
state is retained and blocks automatic relaunch; it is not silently migrated or replayed.

For a failed update with a completed checkpoint, `restore-update-checkpoint`
restores the source into a **new empty database**. Stop the original Core and helper,
preserve their volumes and journals, and use the exact source image recorded in
`checkpoint.json`. Supply the original encryption key and configure the new database.
Mount the private checkpoint at its original absolute path, then run:

```bash
citadel-server restore-update-checkpoint \
  --checkpoint-directory /opt/citadel-update-state/checkpoint-OPERATION_UUID \
  --confirm-instance-replacement
```

The command refuses a nonempty destination or a live Core database lease. It checks
artifact integrity and source build/catalog before restoring, then validates instance,
claim identity, applied catalog and stored credential decryption. The restored
attempt is recorded as failed and its claim is released atomically with its Activity.
The original database and host journal are untouched. If post-restore verification
fails, the restored claim remains closed; preserve that database and retry recovery
into another empty database after correcting the cause. Restore the original JWT,
Agent signing and encryption material and the required Core data before starting the
source image against the recovered database with a **new** update-state directory.
Register that recovered installation separately; do not erase the original evidence.

For the execution invariants and module responsibilities, see
[instance updates](ARCHITECTURE.md#instance-update-work-in-progress). Do not expose
internal commands as operator shortcuts around the full update/recovery flow.

Validate bootstrap changes with the isolated Docker acceptance command in
[`test/README.md`](../test/README.md). It exercises the shipped layout and a compatible customized Edge-overlay installation
and containerized registration with completed HTTP setup. This is a registration
gate; it does not qualify a source-to-target upgrade or recovery path.

The local lifecycle runner in [`test/README.md`](../test/README.md) exercises normal
replacement, interrupted final release and checkpoint restore without publishing
anything. It uses a test-only release dependency; shipped binaries always use the
official verifier and cannot enable the fixture through configuration. Candidate
acceptance runs this regression on both supported architectures.

The local Agent updater runner in [`test/README.md`](../test/README.md) uses the same
test-only Core catalog boundary with real Direct/Edge Agents, separate disposable
Docker engines and source/target images built from the checkout. Its browser mode
leaves approval to the user; its automated mode verifies replacement and
authenticated reconnect. It does not bypass production artifact verification.

The opt-in published-transition smoke test in [`test/README.md`](../test/README.md)
uses the real update APIs and signature checks on a disposable Docker daemon, then
restores the resulting checkpoint into a fresh database. It requires a source that
already contains the updater and an eligible published target; it cannot qualify
the first transition while the compiled source list is empty. Do not add a source
entry merely to make this test reach approval.

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

Change request/view/schema definitions in `src/server/`. The normal build script,
Compose development startup, F5 debugger and `npm run dev` regenerate contracts
automatically. Restart the development session after endpoint or SDK helper
changes; this is startup generation, not a background watcher. Review and commit
the generated changes with the source changes.

To regenerate explicitly without starting the application, run:

```bash
bash src/tools/build/generate-api.sh
```

This generates `schema/v1.json` and `schema/public-v1.json`, copies the full schema
to `src/frontend/src/api/schema/swagger.json`, and runs the frontend's
`api:generate` command. It also generates `src/infrastructure/scripts/generated/citadel.js`
from the SDK template and handler-derived endpoint catalog. Install frontend dependencies first. The public schema
excludes internal endpoints; the frontend consumes the full schema.

The wrapper serializes development generation in this checkout, including concurrent
debugger and UI startup. It calls `cargo run --locked -p xtask -- openapi` with
offline SQLx metadata and needs no running database. Plain `cargo build`/`cargo run`
and direct `vite` invocation do not run this wrapper. CI still rejects stale
artifacts; it does not regenerate and commit changes for developers.

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
findings globally or replace official tools with untested forks. Rust's audit policy
is in `.cargo/audit.toml`; its SQLite advisory exception covers an inactive SQLx
dependency. Remove it before enabling SQLite or when SQLx permits the patched version.

## Database schema changes

The migration catalog starts with `0001_initial.sql` and appends numbered upgrade
migrations. The initial migration includes workload
statistics, alert observation ordering, secret-use grants, and build, backup,
restore and script execution ownership/capacity changes.

Migration `0004_query_indexes.sql` consolidates the former `0004`–`0010` SQL in
its original order, including statistics storage and Instance settings/restart.
It upgrades the unchanged `0001`–`0003` catalog. Databases that already recorded
any migration from the former `0004`–`0010` sequence cannot use this consolidated
catalog directly; their checksums/history remain protected by the migration
runner. Any data-preserving conversion is a separate operator task.

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
