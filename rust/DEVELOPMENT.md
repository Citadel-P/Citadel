# Rust development environment

On Windows, prefer **VS Code in WSL Ubuntu**, with the repository on the Linux
filesystem. Ctrl+Shift+B runs Core and PostgreSQL together in Docker Compose;
Cargo builds and Vite run directly in Ubuntu. F5 runs Core natively for debugging.
The Dev Container remains a supported fallback for native run/debug tasks.

This removes the workspace container and Windows bind-mount file watching from
the normal development loop. It does not remove WSL's VM or guarantee a memory
ceiling: Rust compilation, rust-analyzer, Docker workloads and filesystem cache
still consume memory. Cargo builds use two parallel jobs by default, and the
editor runs `cargo check` instead of Clippy on save. Full Clippy checks remain
available in the check task. Development breakpoints and variable inspection
are preserved.


Run `python3 rust/scripts/measure-cpu-refactor.py /tmp/citadel-cpu-capture` from
repository root after preparing that fixture. Do not build or run tests during
accepted CPU captures; short `--idle-seconds`/`--stats-seconds` runs are fixture
smokes only. The report distinguishes attempted work, committed writes, and
semantic changes so raw counters are not mistaken for avoided work.

## Direct WSL setup (recommended)

1. Install the VS Code **WSL** extension. In Docker Desktop, enable
   **Settings > Resources > WSL Integration > Ubuntu**.
2. From Ubuntu, verify `docker info --format '{{.OperatingSystem}}'` reports
   `Docker Desktop`, and that `/var/run/docker.sock` is accessible to your user.
   Compare `docker ps` with Docker Desktop before proceeding.
   If Ubuntu already runs an independent Docker Engine, stop here and choose
   which engine to use. Do not remove it or its data blindly. The preparation
   task rejects an unexpected native engine; intentionally opting into it with
   `CITADEL_DEV_ALLOW_NATIVE_DOCKER=true` in `rust/.env.development` uses that
   engine's **different containers and volumes**, not Desktop's.
3. Install Linux prerequisites inside Ubuntu:

   ```bash
   sudo apt-get update
   sudo apt-get install -y build-essential pkg-config libssl-dev git curl rsync
   ```

   Install Rust using [rustup](https://rustup.rs/) and Linux Node.js 22 using
   the [Node.js installation instructions](https://nodejs.org/en/download).
   The repository pins the Rust toolchain in `rust-toolchain.toml`. Docker
   Desktop integration must also provide `docker compose`. Verify `cargo`,
   `node`, and `npm` resolve to Linux installations, not `/mnt/c/...`.
   Native debugging of features that execute external tools also needs their
   Linux executables (for example Deno for Automations and Restic for backups).
   The Compose Core image includes these tools.
4. Put the checkout under `~/projects/Citadel`, **not `/mnt/d`**. For a clean
   checkout, clone recursively from your remote. For an existing dirty checkout,
   copy the repository **including `.git` and submodule metadata** into a new,
   empty directory; preserve staged, unstaged and untracked source files. Skip
   disposable `node_modules`, Rust `target`, and `bin`/`obj` artifacts.
   Compare Git status and diffs before using the copy. Keep the original until
   verified; do not replace it with a remote clone that loses local work.
5. In Ubuntu, open the Linux checkout:

   ```bash
   cd ~/projects/Citadel
   code .
   ```

   The status bar must say **WSL: Ubuntu**, not Dev Container. Install the
   recommended Rust/CodeLLDB and frontend extensions in WSL when prompted.
6. Run **Citadel: Initialize development dependencies** once. Then press
   `Ctrl+Shift+B` to run, or choose **Citadel: Debug application** and press
   `F5` for breakpoints. Do not run both at once.

Both workflows build the Rust API and regenerate `schema/v1.json` and
`schema/public-v1.json` at the repository root before launching it. To build
and export without starting the application or PostgreSQL, run the
**Citadel: Build Rust API** task, or `bash rust/scripts/build.sh` from the
repository root. Plain `cargo build` only compiles. The standalone
**Citadel: Generate OpenAPI spec** and **Citadel: Verify OpenAPI spec** tasks
remain available for export and freshness checks.

The preparation task creates an ignored, private `rust/.env.development` file
once. Both the normal tasks and debugger use it. Entries are plain `KEY=value`
(no shell expansion or surrounding quotes); edit it to override defaults.
If you change the Docker socket, update both `CITADEL_RUST_DOCKER_SOCKET` and
`DOCKER_HOST` so API calls and external Docker CLI operations use the same engine.
Default keys are **development-only**, never suitable for production.

In WSL, preparation starts PostgreSQL, with project name `citadel-wsl`,
exposing it on `127.0.0.1:15432`. The normal run task adds Core to the same project
and preserves the existing `citadel-wsl_postgres-data` volume. This database is
separate from the devcontainer's database. Runtime data lives under
`~/.local/share/citadel-wsl`, mounted at the same path inside Core to preserve
signing keys and Docker bind-mount paths. Compiler artifacts remain under
`rust/target` (or the configured Cargo target directory).
Nothing deletes or migrates your old database, volumes, or checkout. To retain
an existing Citadel instance, use its backup/restore procedure and preserve its
encryption/signing keys and runtime data; don't merely point at an old database
with newly generated keys. The Compose task uses this development database and
connects to it as `postgres:5432` inside the project. A custom `DATABASE_URL` is
supported by native run/debug tasks, which skip starting PostgreSQL; the Compose
task rejects it to avoid accidentally switching your database.

Stop the old devcontainer API/UI before running WSL to avoid port conflicts.
To stop Core and the WSL development database without deleting data:

```bash
bash rust/scripts/dev.sh compose-stop
```

Docker socket access does **not** require a container bind mount when the API
runs directly in Ubuntu: it opens `/var/run/docker.sock` as a normal Unix socket.
Do not expose the daemon on an unauthenticated TCP port or make its socket
world-writable. Docker access is privileged; use trusted repositories only.
See [Docker's WSL guidance](https://docs.docker.com/desktop/features/wsl/) and
[VS Code's WSL guide](https://code.visualstudio.com/docs/remote/wsl).

## Dev Container fallback

### Prerequisites

- Docker Desktop with Linux containers enabled
- Visual Studio Code
- The **Dev Containers** VS Code extension

Rust, Node.js, CodeLLDB, rust-analyzer, and the frontend dependencies are
installed inside the development container.

### Open the development container

1. Open the Citadel repository in VS Code.
2. Press `Ctrl+Shift+P`.
3. Run **Dev Containers: Reopen in Container**.
4. Wait until the lower-left status area shows **Dev Container: Citadel Rust**.

Use **Dev Containers: Rebuild Container** after changing `.devcontainer/`.
Ordinary source changes do not require a rebuild. PostgreSQL, Cargo, npm, and
frontend dependency data use named volumes and survive a rebuild.

Inside the Dev Container, use **Citadel: Debug application** (F5), or
**Citadel: Run Rust API** and **Citadel: Run UI only**. The default Compose task
requires the WSL checkout because its binary and runtime bind mounts use host
paths; it rejects execution inside the workspace container.

Both the API and Docker CLI use the feature-mounted `/var/run/docker-host.sock`
directly. The feature's `/var/run/docker.sock` proxy can truncate delayed Docker
exec/run output after a half-close, which breaks long-running builds and backups.
The workspace stays non-root; an additional group grants access to Docker
Desktop's group-0 socket without changing the host socket's permissions. Docker
access is privileged: use this development environment only with trusted code.

Compiler output is disposable. On container start and before the VS Code Rust
run/check/debug tasks, Citadel checks the dedicated `rust-target` cache. It runs
`cargo clean` for dev/test, release and documentation output if the cache
reaches 8 GiB or the container filesystem has less
than 5 GiB free, unless a Cargo/rustc build is already active. This is a cleanup
threshold, not a hard disk quota; one build can exceed it. Direct terminal Cargo
commands do not invoke this hook. Run `bash .devcontainer/clean-build-cache.sh`
from the repository root before a manual build when needed.

The volume mountpoint is preserved. Run
`bash .devcontainer/test-clean-build-cache.sh` inside the devcontainer to verify
the thresholds and safety checks without deleting any actual artifacts.

Cleanup affects only `/home/vscode/.cache/citadel-target`, not PostgreSQL,
runtime data, downloaded dependencies, or source files. The next build after
cleanup takes longer. Development builds retain full debug information for
Citadel crates; dependencies omit it, and tests use line-level backtraces without
incremental artifacts. No production build settings are changed.

Git over SSH verifies the server key against an operator-managed OpenSSH
`known_hosts` file. Set `Git__KnownHostsPath` (default
`<CITADEL_DATA_ROOT>/git-known-hosts`) and mount that file read-only or provision it
in the persistent data volume. Verify server fingerprints through your Git
provider or administrator before adding keys. Unknown and changed keys fail;
Citadel never automatically trusts `ssh-keyscan` output or disables verification.
SSH submodules follow the same trust policy. Private client keys remain temporary
and are removed when execution ends.

Runtime files (including Git repository caches and Automation runs) use the
`citadel-data` volume at `/home/vscode/.local/share/citadel`, configured through
`CITADEL_DATA_ROOT`. Container initialization makes this private directory
writable by `vscode`. It survives rebuilds alongside the PostgreSQL volume.

### Automation execution settings

The Rust worker accepts the existing `Automations__...` environment settings.
Restart the API after changing them. These settings do not grant an Action any
additional Citadel permissions: its run-as identity is still authorized at execution.

| Setting | Default | Purpose |
| --- | --- | --- |
| `Automations__Enabled` | `true` | Enable execution and scheduling |
| `Automations__MaxParallelRuns` | `4` | Shared limit for manual, Test and background runs |
| `Automations__DefaultTimeoutSeconds` | `300` | Default for newly saved Actions |
| `Automations__MaxTimeoutSeconds` | `1800` | Maximum allowed Action timeout |
| `Automations__DenoPath` | `deno` | Deno executable |
| `Automations__WorkDir` | `<data root>/automations/runs` | Private per-run files, removed after execution |
| `Automations__DenoCacheDir` | `<data root>/automations/deno-cache` | Persistent Deno dependency cache |
| `Automations__InternalBaseUrl` | `http://127.0.0.1:8000` | API address reachable from Deno |
| `Automations__AllowNet` | Internal API host and port | Comma-separated Deno network allow-list; an explicitly empty value denies network access |
| `Automations__MaxLogBytes` | `1048576` | Captured output limit, bounded between 1 KiB and 16 MiB |

Scripts may read/write their own run directory, not arbitrary host files. Deno
environment access is limited to `NO_COLOR` and `DENO_DIR`; use Citadel bindings
for Action inputs and credentials. Add external API hosts to `AllowNet` only
when the scripts need them.

## Run Citadel

The platform form loads regular Agent installation instructions from
`GET /api/v1/platforms/agent/setup` (Platform Write permission required).
These instructions expose only the public signing key. If configured,
`CITADEL_RUST_AGENT_PRIVATE_KEY_PATH` supplies the same key used by the Agent
transport; otherwise startup creates a persistent key at
`<CITADEL_DATA_ROOT>/agent/signing-key` with owner-only permissions on Unix.
Preserve this private file across restarts and never share it with Agents.
Installation instructions do not install an Agent or configure its connection.

For normal browser testing, press `Ctrl+Shift+B`. This runs the default
**Citadel: Run application (API + UI)** task. It builds Core and exports OpenAPI,
starts the `core` and `postgres` services in Docker Compose project `citadel-wsl`,
and waits for both to be healthy before starting Vite on the host. Core listens
on port 8000 (and reserves 8001 for Agent transport).

The development image uses the production Dockerfile's `runtime-base` stage,
including Docker CLI, Deno and backup tools. It mounts the local release binary
read-only, avoiding a second Core build in Docker. Compose uses `cargo build --release`
by default so browser CPU measurements use optimized code. Set
`CITADEL_COMPOSE_PROFILE=dev` for faster, unoptimized Compose builds; F5 retains
the normal development profile and debugger information. Each run
recreates Core so the container picks up the latest binary and environment,
without restarting PostgreSQL. The first run downloads/builds the runtime image.

Local volume browsing uses the small `citadel-volume-helper` binary bundled in
both development and release images. Core resolves its container using Docker's
default `HOSTNAME` and launches a read-only helper from that container's immutable
image ID; browsing does not pull an Agent image. Keep the default container
hostname when running Core. The helper has no network access, follows no symlink
paths, and is removed when the operation finishes. For Direct and Edge Agents,
Core uses the running image reported by the Agent to launch the helper on its
owning node. A locally built Agent therefore needs no registry image for volume
browsing. The configured helper image is the fallback when the Agent does not
report its running image.

Remote backup, restore and repository operations also use the running Agent
image, which includes Restic. They do not require a separate `restic/restic`
image on the Agent host. Agents that do not report their image use the configured
backup helper image as a fallback.

When running Core directly on the host, `CITADEL_VOLUME_HELPER_IMAGE` can select
an already installed Agent image containing `/app/Citadel.Agent.VolumeHelper`.
Containerized Core requires no helper-image configuration.

Open these addresses from the host:

- UI: <http://localhost:5173> or <http://127.0.0.1:5173>
- API health: <http://localhost:8000/health>

The task creates separate **citadel-api** and **citadel-ui** terminal panels.
The API is ready when its panel reports that the Compose services are running.
View Core logs with `docker logs -f citadel-wsl-core-1` or Docker Desktop.

For Edge Agents in Docker Desktop, the development environment advertises
`EdgeAgent__PublicGrpcUrl=http://host.docker.internal:8001`. Generated installation
commands work on Docker's default bridge network; no `--network` option is needed.
Existing development environments can set this value in `rust/.env.development`
and restart Core. A remote Agent needs a hostname or IP that reaches Core from
that host instead. Set `EdgeAgent__PublicGrpcUrl` accordingly; `localhost` inside
an ordinary Docker container points to that container. For the release Compose
project, the default published Edge port is `18001`.

The enrollment API uses `EdgeAgent__PublicGrpcUrl` as the default `CITADEL_CORE_URL`.
The setup form lets you enter Core's address as reached from the Agent machine
and updates the Docker command immediately. Public addresses are prefilled;
local-only defaults leave the field empty until you supply an address. For an
Agent on the same Docker Desktop host, enter `http://host.docker.internal:8001`.
For a remote Agent, enter Core's reachable IP or hostname and Edge port, not the
Agent's address. This choice applies to the displayed/copied command and does
not require restarting Core or generating a new token. It is not saved as Core
configuration. The browser's `localhost` address cannot identify Core's network
address from another machine.

To test an Agent on another machine against the development Compose project,
set `CITADEL_DEV_EDGE_BIND_ADDRESS=0.0.0.0` in `rust/.env.development` and restart
the Compose task to recreate Core's port mapping. This publishes only the Edge
port (`8001`) on the host's network interfaces; API and PostgreSQL ports remain
local-only. Enter `http://<core-host-lan-ip>:8001` in the enrollment form. Changing
the command's address alone does not change Docker's port binding. If a host
firewall blocks inbound TCP port `8001`, allow it from the Agent's network.
Use TLS for production as described below.

For remote installations, configure `EdgeAgent__PublicGrpcUrl` in Core's deployment
environment (the release Compose project uses `rust/.env`). For example,
`EdgeAgent__PublicGrpcUrl=https://edge.example.com` requires a gRPC-capable reverse
proxy for that hostname forwarding to Core's Edge listener. Configure
`Transport__Mode=ReverseProxy`, the trusted proxy settings and `AllowedHosts`
accordingly, or use `Direct` with Core's TLS certificate settings. Ensure the
listener or proxy is reachable from the Agent host; Compose's default
`127.0.0.1` port binding is for local access. Restart Core and generate a fresh
enrollment command after changing the advertised address. Neither
`host.docker.internal` nor `localhost` identifies a Core running on another host.

For a release build, run **Citadel: Run release (Compose)** with `Ctrl+Shift+R`.
This builds Core with `cargo build --release`, bundles the frontend into the
image, and starts Core and PostgreSQL in Compose project `citadel`. The task uses
`rust/.env` and waits for both services to be healthy. With the default settings,
open <http://localhost:18000> for the application. This project has its own data
volumes, separate from the development project `citadel-wsl`.

VS Code stores custom shortcuts in user settings. On a new installation, open
**Preferences: Open Keyboard Shortcuts (JSON)** and add this entry to the array:

```json
{
  "key": "ctrl+shift+r",
  "command": "workbench.action.tasks.runTask",
  "args": "Citadel: Run release (Compose)",
  "when": "workspaceFolderCount > 0"
}
```

The equivalent command from the repository root is:

```bash
docker compose --project-name citadel --env-file rust/.env -f rust/docker-compose.yml up --detach --build --wait --wait-timeout 120
```

Only the Dev Container enables file-watcher polling at one-second intervals so Vite
detects edits made on the Windows host. Without polling, mounted files can change
while Vite continues serving an older transformed module, even after a browser
refresh.

Run **Citadel: Stop Core + PostgreSQL (Compose)** to stop the containers while
preserving data. Use **Tasks: Terminate Task** to stop the Vite task separately.
Compose services run detached and continue running when VS Code closes.

To start only the Compose services from a WSL terminal:

```bash
bash rust/scripts/dev.sh compose-up
```

## Debug Rust

1. Stop the Compose services and Vite task if running, then open **Run and Debug**
   with `Ctrl+Shift+D`.
2. Select **Citadel: Debug application**.
3. Add breakpoints in the Rust source.
4. Press `F5`.

This launch configuration starts Vite and the unoptimized Rust API under
CodeLLDB. Press `Shift+F5` to stop both. Do not start the normal application
task at the same time because both workflows use ports 5173 and 8000 and Core
requires an exclusive background-job lease on its database. Stop the debugger
before returning to Ctrl+Shift+B.

## Run one process manually

Use separate Linux terminals at the repository root when investigating one
side of the application. Prepare the environment once:

```bash
bash rust/scripts/dev.sh prepare
```

```bash
cd rust
bash scripts/dev.sh exec cargo run --locked -p citadel-server -- serve
```

```bash
cd src/Citadel.FrontEnd
bash ../../rust/scripts/dev.sh exec npm run dev -- --host 0.0.0.0 --port 5173 --strictPort
```

The second command runs only the UI. API-backed pages will report that Citadel
is unreachable unless the first command or the debugger is also running.

## Checks

Run the committed tasks from **Terminal > Run Task**:

- **Citadel: Check Rust workspace** runs formatting, Clippy, and Rust tests.
- **Citadel: Check frontend** runs frontend lint and unit tests.
- **Citadel: Initialize development dependencies** refreshes locked Cargo and
  npm dependencies when their lockfiles change.

Run `bash rust/scripts/test-dev.sh` to check environment generation, run/debug
settings compatibility and startup guards without building Rust or starting
containers. In WSL it additionally exercises the daemon selection and database
startup guards with a mocked Docker command.

## Host disk metrics

Disk usage measures the filesystem containing Docker's data directory, rather
than the sum of Docker image or volume sizes. Rust persists the used bytes, total
bytes, and percentage for dashboard/history queries and built-in disk alerts.

For containerized Core or Agent, expose the daemon host read-only:

```yaml
volumes:
  - /var/run/docker.sock:/var/run/docker.sock
  - /:/host:ro
```

`CITADEL_HOST_ROOT` defaults to `/host`. Core running natively **on the same host
as Docker** can use `CITADEL_HOST_ROOT=/`, with permission to access Docker's
data directory. There is no automatic fallback to the Core filesystem: WSL
Ubuntu connected to Docker Desktop cannot measure Docker Desktop's disk through
the socket. Use a Core or Agent container on that daemon with the host mount.
With Docker Desktop, verify the mount source: binding `/` from Ubuntu WSL can
expose Ubuntu's root rather than the daemon filesystem. A mount alone does not
prove it is the correct disk.

Agent and Edge telemetry retains nullable disk fields; older agents without
these measurements remain supported. Missing mounts, failed probes, invalid or
stale remote measurements remain unavailable, rather than reporting zero.

The seeded critical disk rule triggers at 90% or more after three matching
observations, with a 300-second cooldown. Valid below-threshold readings resolve
the incident; missing telemetry does not falsely resolve it. Notification
channels must be attached to the rule for external delivery.

## Database schema changes

The declarative schema in `crates/infrastructure/database/src/schema/schema.sql` is the Rust
database authority. Until the first Rust release, keep a single generated
baseline and fold schema changes into it; do not hand-write migration SQL:

```bash
cd rust
cargo run --locked -p xtask -- database refresh-baseline
cargo run --locked -p xtask -- database verify
```

`refresh-baseline` regenerates `0001_initial.sql`, its checksum manifest, and the
compile-time migration catalog from the declarative schema. Numbered follow-up
migrations begin only after the first Rust release freezes that baseline.

Refreshing the baseline does not upgrade an existing development database. If
Core exits with `AppliedChecksum` after a rebuild, its database still records an
older baseline. The default Ctrl+Shift+B task waits for Core to become healthy
before starting Vite, so this also prevents the UI task from starting. Back up the
database, compare its schema with a disposable database created from the new
baseline, and apply the required schema changes before aligning the local migration
journal. Preserve existing rows and saved preferences. Do not bypass checksum
validation or replace the recorded checksum without verifying schema equivalence.

## Restore a Citadel system backup

Stop every Citadel Core instance before restoring. Extract the selected Restic
snapshot to a private local directory, configure the target PostgreSQL URL, and
run the offline command from the Rust release image or development container:

```bash
export DATABASE_URL='postgres://citadel:password@postgres:5432/citadel'
cargo run --locked -p citadel-server -- restore-system \
  --bundle /private/recovery-bundle \
  --confirm-instance-replacement
```

The command validates the manifest, its complete checksum inventory, safe file
paths, and the PostgreSQL archive signature before changing the database. It
then performs a single-transaction `pg_restore --clean`; a failed restore rolls
back instead of leaving a partially replaced schema. Keep every Core instance
offline until the command succeeds. The restored database still requires the
original signing and encryption keys. Restore file-backed keys from the bundle's
`recovery/` directory into `CITADEL_DATA_ROOT` before running the command, and
supply any external keys listed in the recovery manifest.

## Application configuration

`rust/.env.example` lists user-facing Core settings. Copy it to the
ignored `rust/.env`, fill in `PG_PASSWORD`, and use
`docker compose --env-file rust/.env -f rust/docker-compose.yml up -d --build`. The Compose service
passes these settings into Core. Entries use plain `KEY=value` without shell
expansion or surrounding quotes. Restart/recreate Core after changing settings.

The container entrypoint detects the mounted Docker socket's group and then
starts Core as user 65532 with access to that group. No socket group setting is
needed, and the host socket's ownership and permissions are left unchanged.
An explicit non-root container user retains its configured groups; the
development launcher supplies the socket group automatically for that workflow.

For the VS Code run/debug workflows, put overrides in `rust/.env.development`;
that file is intentionally separate from the production Compose `.env`, so a
production database or signing key cannot accidentally replace development data.
The same application settings work in both files. Native `cargo run` reads its
process environment; it does not automatically load an env file.

The env template covers installation, transport, authentication, logging,
monitoring, automation, builds, backups and Agent connections. Internal queue
sizes, polling intervals, lease durations, buffers and executable paths use code
defaults and are omitted from the template. Advanced overrides remain supported
for development and troubleshooting; see `crates/server/src/config.rs` and
`config/execution.rs` for their definitions.

`CITADEL_IMAGE` selects the Core image. When building a versioned image locally,
`CITADEL_VERSION` sets build metadata; `CITADEL_IMAGE_TAG` is a fallback for both
when their explicit overrides are unset.

For Direct TLS, add `-f rust/compose.direct-tls.yml` after the main Compose file; it mounts
`CITADEL_TLS_HOST_DIRECTORY` read-only at `/etc/citadel/tls`. Configure HTTPS
public URLs and the mounted certificate/key paths. The built-in health probe
uses the configured API port and, in Direct mode, trusts Core's configured
certificate for the loopback connection.

An unset/blank `Jwt__Key` or `Secrets__EncryptionKey` is generated once in
`CITADEL_DATA_ROOT` (`jwtsecret` and `secret-encryption-key`). Existing explicit
keys always take precedence. Keep this data volume across container recreation.
System backups include file-backed keys from `Backups__CoreDataPath`, including
the Rust Agent signing key at `agent/signing-key`; explicitly configured JWT and
encryption keys remain external requirements in the recovery manifest. Before
offline recovery, restore the bundle's `recovery/` contents to the data root and
retain any external keys. Recovery never generates replacement keys.

`AgentTransport__CaCertificatePath` accepts a mounted PEM CA bundle for HTTPS
connections to direct Agents. HTTPS also uses native roots and validates the
Agent hostname. Setting `AgentTransport__AllowInsecure=false` rejects HTTP Agent
connections. This CA is separate from Core's listener certificate and from the
CA distributed to node agents through `CITADEL_NODE_AGENT_CA_CERTIFICATE_PATH`.

`Backups__AllowedCorePaths__0`, `__1`, etc. restrict Core filesystem repository
locations; the default is `<CITADEL_DATA_ROOT>/backups/repositories`. Add roots
for existing repositories outside that directory. Relative paths use the first
allowed root, so `daily/core` resolves to
`<CITADEL_DATA_ROOT>/backups/repositories/daily/core` with the defaults. Absolute
paths must stay within an allowed root; parent traversal and symlink escapes are
rejected. Paths must be available in the
Core container. `Backups__DefaultTimeoutSeconds` controls repository operations
and restores; backup policies retain their individual execution timeouts.

Core logs use readable, single-line text by default (`LogFormat=text`). Set
`LogFormat=json` for structured log collection. `EnableLogColor` controls only
text color: when unset, color is enabled for an interactive terminal and disabled
for redirected/container output. Explicit `true`/`false` overrides detection;
JSON never includes ANSI colors. `RUST_LOG` defaults to `info`; use
`RUST_LOG=info,citadel_server=debug` when investigating Core behavior such as
realtime subscriptions. The full effective startup configuration is logged at DEBUG.
Existing `.env.development` files retain their settings; change an old DEBUG
filter to `RUST_LOG=info` to reduce routine chatter.

`citadel-server print-effective-config` reports effective non-secret values
without connecting to PostgreSQL. The Agent TLS integration test requires OpenSSL
to create a temporary certificate; no test keys are committed.

## Troubleshooting

### Cannot connect to Citadel

Check <http://localhost:8000/health>. If it is unavailable, inspect the
**citadel-api** terminal and ensure that the API task or debugger is running.
The **Citadel: Run UI only** task intentionally does not start the API.

Both `localhost:5173` and `127.0.0.1:5173` are allowed development origins.
Other ports are rejected, and Vite uses `--strictPort` so it cannot silently
move to an incompatible origin.

### Port 5173 or 8000 is unavailable

Stop an existing Citadel task or debugger before starting another. On Windows,
the listening process can be inspected from PowerShell:

```powershell
Get-NetTCPConnection -State Listen -LocalPort 5173,8000
```

### WSL Wayland mount prevents container creation

Citadel does not use a GUI socket. If the Dev Containers log reports a missing
`distro-services/ubuntu.sock` or `wayland-0` mount, add this application-level
setting to the VS Code **User** settings JSON and rebuild the container:

```json
{
  "dev.containers.mountWaylandSocket": false
}
```

### Docker is unavailable inside the container

Confirm Docker Desktop is running, then rebuild the development container. From
its terminal, `docker version` must report both a client and a server when run
as the normal `vscode` user. A `permission denied` error means the Dev Container
socket proxy was not initialized; use **Dev Containers: Rebuild Container** so
the Docker-outside-of-Docker feature can recreate its non-root socket.

## Runtime behavior

### Container-driven resource status

Known `start`, `die`, `pause` and `unpause` events use a narrow state projection;
Local and updated Agents perform no Docker metadata read for these actions. The
writer validates platform/node/Docker identity, uses the identity index only as a
hint, preserves metadata and operation claims, and suppresses semantic no-op
revisions. Unknown identities request Container reconciliation only. `create`,
`rename` and other supported metadata observations retain targeted hydration;
confirmed deletion remains an ID-only tombstone. Older enriched Agent events
are accepted, but their metadata is not applied on a state-only transition.

Local daemon timestamps, or Agent receive timestamps, feed the existing
Unix-second projection watermark. Within a batch, available millisecond timestamps
choose the latest state; equal timestamps use arrival order. Across batches,
equal-second transitions retain stream order. Resource generations still fence
snapshots captured before a delta. This is not nanosecond ordering or a new timestamp field in the Agent protocol.

Local, Direct and Edge gather a maximum of 256 raw state events over a fixed 75 ms
window. Coalescing never crosses metadata/create/destroy, recovery or scope changes.
Each resulting platform/node batch takes one projection guard and state transaction;
Edge validates the current authenticated session in that same transaction.

State writes return parent/operation IDs. Unmanaged containers skip Stack and
Deployment reconciliation entirely. Operation-owned state changes also skip parent
effects and Stack drift, while preserving their durable claims. External parents
are reconciled once per distinct ID: Stack effects share the state transaction;
Deployment effects retain their post-commit transactions to preserve lock ordering.
One realtime invalidation carries the changed IDs.

Container commands register claimed targets before mutation. Committed state events
confirm the expected result for a bounded 150 ms window; only missing targets use
read-only inspection. Parent status, one transition activity, and claim release then
commit together. The coordinator is a local hint; durable claims and stale recovery
remain authoritative. Inspections capture a projection generation before Docker I/O;
a superseding committed event rejects the write and retries missing observations
up to three times. A standalone claim finishes without reading or locking parent
tables. Parent claims retain the durable platform/parent lock order, including
recovery after their last target disappears.

Lifecycle patches route only to the matching Container, DockerDaemon and detail
subscriptions. Parent status changes publish separately with affected parent and
Platform IDs. Container statistics share a narrow identity read; Platform telemetry
uses the current sample plus a narrow Platform context, without loading inventory
relationships or the last flushed statistics. Detail subscriptions receive partial
`ReceiveContainerInfoPatch` messages and preserve metadata in the browser.

Recovery composes independent resource refreshes after metadata validation at
startup, on reconnect, and on a six-hour safety pass. Swarm projection recovery
has its own coordinator: at most 256 pending/in-flight Platforms per coordinator,
a 1.5-second event debounce, and at most four concurrent refreshes subject to the
shared inventory I/O budget. Duplicate dirty events occupy no additional slots;
events during collection retain one follow-up. The independently configured
`JobConfiguration__SwarmReconciliationIntervalSeconds` (30 minutes by default)
marks only Swarm projections dirty. Local/Direct targets come from the runtime
registry; Edge safety work is session-bound and only enabled for a confirmed
manager. This path reads nodes, services, tasks, networks, configs and secrets;
only Swarm-scoped networks are persisted in the Swarm projection. Local collection
uses network topology without standalone container usage enrichment. Legacy
remote connectors retain their compatible network-list RPC, whose Agent may
enumerate containers to derive usage. Image/volume inventory is not collected.

Platform health is checked every five seconds with a two-second timeout,
three failures before going offline, and two successes before recovery.
The monitor only probes and emits confirmed transitions through bounded ingress.
A separate lifecycle coordinator persists status, requests one metadata-first
recovery plan, evaluates alerts and publishes committed realtime changes.
It retains per-Platform retry progress, so an alert failure cannot repeatedly
queue resource recovery. While a confirmed Platform remains offline, the same
coordinator re-evaluates its alert every 30 seconds so cooldown or quiet-hour
suppression cannot permanently lose an outage. These checks reuse committed
progress and do not repeat status writes, recovery plans or Platform broadcasts.
Recovery or removal stops the offline checks. Alert responses include the latest
responsible actor; automatic resolution is attributed to System.
Direct Agent subscription ownership is woken on Online;
existing streams remain responsible for reconnecting, and Edge sessions own their
own bootstrap. Deployment observation crash catch-up runs separately every five
minutes; stable health probes perform no Deployment reconciliation.
`JobConfiguration__MonitoringInterval` controls statistics sampling. Platform
health has its own five-second probe cadence: an unreachable daemon degrades its
Deployments; recovery refreshes inventory before restoring observed health.
`/ready` checks PostgreSQL and Docker on demand, while `/health` remains a cheap
liveness check. Separate node-Agent
container projections are preserved when the manager disconnects.

Swarm task health remains owned by Swarm reconciliation. Existing container,
Deployment Apply, and Stack operation recovery jobs continue to recover expired
operations; event synchronization does not overwrite active operation claims.

### Recovery worker ownership and wakeups

PostgreSQL operation claims and queue rows are authoritative. One shared LISTEN
connection fans out twelve fixed, capacity-one watch topics; repeated notifications
coalesce, and reconnect wakes every topic. Claim/queue notifications commit with
the state they describe. Workers never replay ambiguous daemon mutations.

| Owner | Normal trigger and bounded work | Missed signal / crash fallback |
| --- | --- | --- |
| Container mutation recovery | Claim notification schedules one pass after 65s; up to 10 claims, observations persisted in batches of 100 | Startup, then every 5 minutes; existing 60s stale fence and 300s abandonment age |
| Deployment recovery | Apply/update-check claims schedule one pass after 65s; 10 Applies and 25 update checks | Startup, then every 5 minutes; Apply fence remains 11 minutes |
| Stack recovery | Claim notification schedules one pass after the 16-minute Apply fence; 25 claims | Startup, then every 5 minutes |
| Swarm active rollout observer | Accepted-operation notification; keyset pages of 25 accepted claims, 5s follow-ups while work exists | Startup and every 5 minutes discover missed accepted operations; no 5s scan when idle |
| Swarm crash janitor | Claim/update-check notification schedules one pass after 65s; 25 claims/checks | Startup, then every 5 minutes; dispatch recovery uses an 11-minute fence |
| Stack webhook dispatcher | Queue/settlement notification; batches of 10, continue while progress is made | Startup, 30s ready/retry fallback; durable available-at timestamps and Apply claims fence execution |
| Legacy resource maintenance / volume helpers / historical Swarm task pruning | Existing startup recovery; pruning also consumes terminal-task and setting-change notifications | Staggered 5-minute safety scans |

Each recovery coordinator runs one feature batch at a time; SQL claims retain
execution ownership and existing mutation admission limits. Swarm observation
keeps only a keyset cursor and a work flag, not an inventory cache or an ID queue.
Timers skip missed ticks. Cancellation ends waits immediately and is checked
between batches; dispatched operations retain their process-owned cancellation
and timeout behavior. Platform health keeps its five-second cadence and
hysteresis; readiness probes run only when `/ready` is requested. Lease expiry
stays at 30 seconds, and Git/update scheduling is handled separately.

### Statistics ingestion and persistence

Local, Direct Agent and Edge collectors enqueue telemetry through two shared
writers. Each writer has an independent bounded queue of
`CITADEL_RUST_EVENT_QUEUE_CAPACITY` rows (default 256), and retains at most
four times `JobConfiguration__BatchSize` rows (default 500 per batch) in its flush/retry buffers.
Collection backpressures when a queue fills; sampling ticks skip missed cycles.
No producer spawns a database retry task. A ten-second sampler does not imply a
ten-second commit: each writer flushes at its row threshold or after
`JobConfiguration__FlashInterval` (default 60 seconds) from the first buffered row.

Container rows retain capture timestamps and the owning Platform/node identity.
Flushes recheck connector/address or the authenticated Edge binding/session,
filter deleted/replaced container identities in bulk, and persist Container and
Swarm-task statistics in one transaction across sources. Platform samples retain
host totals computed at collection time; later inventory deletion does not
retroactively reduce those totals. Platform history and Container history are
independently committed telemetry. Platform metadata uses the newest supplied
sample and cannot replace a newer persisted summary with an older one. Missing
or invalid disk values stay unavailable, including on empty hosts.

Realtime receives each source sample at ingress; the hub avoids payload creation
without subscribers. Database latency does not impose per-sample commit latency
on websocket updates. Platform writer commits wake threshold evaluation through
`citadel_platform_stats`; durable `alertpending` rows and the configured fallback
recover missed notifications. Identical retry keys do not re-arm evaluated alerts.

Successful writes keep set-based batching across sources. A failed mixed batch is
split by Platform so healthy sources can progress; each Platform retains order and
retries with 1–30 second backoff. Database scope locks have a 500 ms deadline,
statements a five-second deadline and a flush a ten-second outer bound. Permanent
write errors are reported and rejected. Retained telemetry expires after two
minutes; when retention capacity fills, the oldest retained batch is discarded to
admit new samples. Both paths increment explicit counters. Non-finite/invalid
samples are rejected before persistence; partial invalid cycles do not produce
apparently complete host totals.
Shutdown cancels collector admission, closes both queues, and drains accepted
samples with a five-second total final-flush budget per writer (also bounded by
the process shutdown deadline). Unflushed telemetry may be discarded on shutdown;
statistics are not durable business operations.

Effective configuration reports `statsFlushIntervalSeconds`,
`statsBatchSize` and `statsQueueCapacity`. Fixed `PlatformStatsIngress` and
`ContainerStatsIngress` metrics expose accepted sample counts, queue depth,
saturation and enqueue wait. `PlatformStatsFlush` and `ContainerStatsFlush` expose
attempts, successful flushes, persisted logical samples, failures and buffered
rows; `PlatformStatsStale`/`ContainerStatsStale` count discarded ownership mismatches.
`StatsShutdownDrop` counts accepted rows left unflushed at writer shutdown.
`StatsRejected` and `StatsRetentionDrop` distinguish invalid/permanent failures
from expired/capacity-evicted retry data. `age_microseconds` reports the last observed
capture age at Container ingress, retained-batch age at flush, and metadata age for
`DockerMetadataRefresh`. `AgentChannelCreated` counts channel construction, not
network handshakes.

Local and Agent container collection does not wait for image/network/volume/storage
metadata. Each sampler owns at most one cancellable metadata task and expires its
optional result after three minutes; storage usage tracks last success separately
from retry attempts. Container coverage is no longer truncated at 1,024 identities;
only concurrency is capped. Agent cache freshness is bounded by the requested
interval and six seconds, whichever is smaller. Updated Agents return the capture
timestamp, and metadata-only inventory requests skip per-container statistics.
Old peers retain protobuf defaults and the legacy receipt-time fallback. Edge
streams use the same configured monitoring interval as Local/Direct streams.

Committed Git/Swarm failure observations are persisted in `alertobservations` in the
producer transaction. Notifications only wake processing; startup and a 30-second
fallback recover missed wakes. Two-minute claims recover interrupted consumers;
per-rule transactional receipts prevent retries from incrementing match counters
or delivering the same evaluation twice. Successful acknowledgement deletes the
observation and its receipts. Alert delivery also backs off on overdue locked work
and errors, independently of notification volume.

### Background job lifecycle and configuration

Platform adapters implement separate info, health, stats, inventory, observation
and mutation capabilities. Resource collectors receive only their matching port
through `ResourceCollector`; a Container collector cannot enumerate other
resources or fetch Platform info. Swarm collection explicitly also receives
networks, and manager bootstrap additionally receives Platform info. Broad
Platform traits are method-free composition bounds, not scoped job dependencies.
Registration uses only info; supplied-container sampling uses only stats.

Stack update checks admit one live check per Stack after authorization and before
external inspection. A duplicate returns Conflict; distinct Stacks share the
existing four-operation budget. The key remains held through persistence and is
released on completion, error, cancellation or request drop. Database version
checks still fence concurrent edits/Apply. Deployment and Swarm update checks keep
their existing durable claims before external I/O. `StackUpdateDuplicate` counts
rejected duplicate Stack checks without resource labels.

Git synchronization waiters use one shared completion generation per execution
service. Completion/failure commits wake waiters to read authoritative ref state;
a ten-second fallback recovers missed hints or changes made outside that service.
The five-minute wait deadline includes enqueue and database reads, and caller
cancellation stops only that wait. `GitSyncStateRead` counts ref-read attempts.
The Git worker's existing two-second idle claim cadence is independent of these
waiters and remains unchanged.

Core holds a PostgreSQL advisory lease for its background jobs for the entire
process lifetime. A second Core against the same database is rejected before
startup recovery. Stop the previous backend before starting a new binary.
Startup marks abandoned build, backup, restore and automation executions as
interrupted, including when their dispatch is disabled. Periodic recovery
continues to respect active execution deadlines and operation claims.

| Setting | Default | Purpose |
| --- | --- | --- |
| `JobConfiguration__FlashInterval` | `60` seconds | Maximum partial stats-batch age and threshold-alert fallback |
| `JobConfiguration__BatchSize` | `500` | Per-writer row threshold and bounded threshold-alert batch size |
| `EdgeAgent__NodeAgentRemovalGraceMinutes` | `10` | Revoke credentials only for absent Swarm nodes beyond this grace period |
| `EdgeAgent__SupportedNodeArchitectures__0`, `__1`, … | `amd64`, `arm64` | Architectures requiring node-agent coverage; `x86_64`/`aarch64` aliases normalize |
| `Builds__MaxParallelRuns` | `4` | Concurrent build executions |
| `Builds__RunCleanupEnabled` | `true` | Scheduled build retention |
| `Builds__RunRetentionDays` | `90` | Terminal build retention in days; disable cleanup with `Builds__RunCleanupEnabled=false` |
| `Backups__Enabled` | `true` | Backup/restore dispatch and policy scheduling |
| `Backups__MaxParallelRuns` | `2` | Slots in each backup and restore worker; repository/source leases still serialize conflicting work |
| `Backups__PollIntervalSeconds` | `2` | Minimum queue polling delay |
| `Backups__SchedulePollIntervalSeconds` | `30` | Backup scheduling interval |

Every persisted Direct Agent has independent event/statistics subscriptions;
endpoint rejection and reconfiguration do not terminate Core. Unmanaged-container
alerts recheck ownership after a 30-second grace period. Historical Swarm task
pruning honors the platform setting and uses node-aware, non-forced deletion
without removing volumes. Stack AutoFix consumes committed stop/pause events as
well as periodic sweeps. PostgreSQL listeners are registered before producers.

Maintenance runs every minute, independently of dispatch and incoming samples.
It retains seven days of statistics and 90 days of activities/action runs,
removes expired refresh tokens and backup leases, and applies configured build
retention while preserving referenced builds. Deletion batches are bounded.

Image registry scanning runs every 90 minutes and shares a bounded, one-day cache
across Deployment, Compose Stack and managed Swarm Service update checks. The
initial scan barrier honors shutdown; manual/webhook checks still perform fresh
registry I/O. Scheduled Notify checks do not require automatic-deployment
entitlements. Automatic mutations retain their entitlement gates.

Statistics are committed as they arrive. The threshold worker uses persisted
samples as its buffer, takes the median CPU/RAM per flush, and uses only the
newest disk reading (including an unavailable reading). Failed writes retain
one batch with bounded retry delays. Successful threshold alerts consume their
consecutive-match count.

Citadel is unreleased: all SQL changes belong in the declarative schema and
`0001_initial.sql`, not additional migrations. Regenerate with
`cargo run -p xtask -- database refresh-baseline`, then verify with
`cargo run -p xtask -- database verify`.

The initial baseline includes the durable Git Manual/Poll/Apply/Webhook origin
and pending statistics alert markers. Polling includes branches tracked by Git
Stacks; unchanged polls and identical repeated failures do not add activities.
Webhook sync failures publish committed alert snapshots. Late statistics are
included in a subsequent flush; acknowledged history is not replayed at startup.

### Live Swarm job recovery regression

`citadel-adapters --test stack_runtime_local swarm_material_capture` is an opt-in test for an **isolated two-node Swarm**. It deploys test Configs/Secrets, interrupts a release, stops the supplied worker container, and verifies that periodic inventory degrades and then restores the Stack. Do not point it at a development or production Swarm.

Required test-process environment:

- `CITADEL_WORKLOAD_DATABASE_URL`: disposable PostgreSQL database.
- `CITADEL_RUNTIME_DOCKER_SOCKET`: isolated manager's Unix socket.
- `DOCKER_HOST`: the same manager socket, for the Docker CLI.
- `CITADEL_RUNTIME_IMAGE`: image present on both nodes; the fixture uses shell/sleep (tested with `alpine:3.21`).
- `CITADEL_RUNTIME_SWARM_WORKER`: outer Docker-in-Docker worker container, labelled `citadel.test=jobs-live`. The test checks this label before stopping it; outer container commands use the default Docker connection with `DOCKER_HOST` removed.
- A current Docker CLI supporting `stack deploy --detach=false` on the test-process `PATH`, and a writable process-specific `TMPDIR`.

For Docker-in-Docker, bind a dedicated socket directory and add a socket listener there. Do not share the entire `/var/run`: persisted containerd PID files can prevent the test worker restarting. Keep the DinD network private and clean up only the test containers, their volumes and that network afterward.

From `rust/`:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters \
  --test stack_runtime_local swarm_material_capture -- --include-ignored --nocapture
```

### Production Edge Agent jobs regression

`crates/infrastructure/adapters/tests/edge_agent_jobs_acceptance.rs` tests the actual Agent binary against Rust Core's Edge transport and persistence. Build the Agent image from its separate checkout, then run from `rust/`:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters \
  --test edge_agent_jobs_acceptance -- --ignored --nocapture
```

Supply these environment variables:

- `CITADEL_EXECUTION_DATABASE_URL`: a disposable PostgreSQL database.
- `CITADEL_TEST_AGENT_IMAGE`: the production Agent image built for this checkout.
- `CITADEL_TEST_AGENT_NETWORK`: the isolated Docker network.
- `CITADEL_TEST_AGENT_HOST`: an address the Agent container can use to reach the test process's listener (`host.docker.internal` worked with Docker Desktop/WSL).
- `CITADEL_RUNTIME_SWARM_MANAGER`: the disposable DinD container, labelled `citadel.test=jobs-live`.
- `CITADEL_RUNTIME_DOCKER_SOCKET`: that isolated daemon's socket exposed at a host bind-mount path.

The test explicitly uses the outer Docker socket `/var/run/docker.sock` for fixture management. The Agent mounts only the supplied isolated daemon socket. It creates a uniquely named Agent and workload, restarts that Agent, exercises reconnect/revocation, and removes its containers. It does not restart Core or use a development database. The fixture checks the DinD label before any runtime mutation.

## Agent development

`citadel-agent` is a separate executable in the same workspace. It provides startup validation, HTTP/HTTPS health, graceful shutdown
and the complete Direct RPC surface. Edge profiles now enroll, reconnect and send
heartbeats, and ordinary/Build Pool Edge profiles execute all 68 commands.
Swarm-node profiles enforce a restricted dispatcher and validate helper operations;
release verification uses the compatibility and acceptance suites below.

To run the Edge connection host from `rust/`, supply a Core enrollment token and
persistent writable state paths (the token is unnecessary after enrollment):

```bash
CITADEL_AGENT_MODE=edge CITADEL_CORE_URL=http://localhost:8001 \
CITADEL_EDGE_ENROLLMENT_TOKEN='<enrollment token>' \
CITADEL_EDGE_AGENT_KEY_PATH=./data/edge-agent.key \
CITADEL_EDGE_IDENTITY_PATH=./data/edge-agent.identity.json \
  cargo run --locked -p citadel-agent
curl http://127.0.0.1:9000/health
```

`/health` is unsigned process liveness. It does not certify Docker reachability or
an established Core session. Edge profiles bind only `127.0.0.1`; Direct mode
binds `0.0.0.0` and requires `HUB_PUBLIC_KEY`. `CITADEL_AGENT_PORT` defaults to 9000.
Ctrl+C and SIGTERM cancel the outbound connection and drain the health listener,
with a ten-second shutdown limit. The Core URL must reach its HTTP/2 Edge listener.

Configuration is read from the process environment, with no implicit `.env`
loading. The four `.env.*.example` Agent templates are intended for deployment
with Docker's `--env-file`. Build Pool inbound configuration uses the same Direct
host; Edge Build Pool selects `CITADEL_EDGE_AGENT_PROFILE=edge-build-agent`.
Swarm-node configuration requires every injected identity field documented in
[the protocol inventory](docs/agent-protocol.md).

The shared Docker client uses `/var/run/docker.sock` by default. `DOCKER_HOST`
can select another Unix socket, `tcp://host:2375` or `http://host:2375`. The selected
endpoint applies to HTTP requests, streaming/exec and Docker CLI build/stack work;
there is no fallback to a different daemon. HTTPS and Windows named pipes are not
supported by this Linux host. The Agent never connects to PostgreSQL. Edge startup
validates its persisted state before listening; its background connection probes
Docker and retries if Docker or Core is temporarily unavailable.

Persist both Edge state files across restarts. They are private and atomically
written; corrupt files fail startup and remain untouched for recovery. A changed
ordinary enrollment token resets both files only when its fingerprint differs
from the saved token fingerprint. Swarm-node uses its mounted bootstrap credential
and does not reset identity merely because that credential rotates.

HTTPS validates normal trusted roots and the Core hostname. A configured
`CITADEL_EDGE_CORE_CA_CERTIFICATE_PATH` adds trust without disabling those checks.
HTTP connections log a warning. Heartbeats probe Docker every 30 seconds;
reconnects back off from 1 to 60 seconds with jitter. Ordinary and Build Pool
profiles dispatch the full command protocol, including interactive terminal input,
resize, binary output and live progress. Commands have independent cancellation
and deadlines, with at most 16 active commands and bounded input/output queues.
Disconnect and shutdown drop all owned command work. Swarm-node commands must
target the configured node and satisfy the allowlist in the protocol inventory.
Helper creation and binary exec validate ownership, mounts, privileges and exact
command patterns. Restore-volume deletion requires ownership and no container
references, including stopped containers.

`rust/Dockerfile.agent` packages the Agent with the existing Rust volume
helper at `/usr/local/bin/citadel-volume-helper`. The compatibility alias
`/app/Citadel.Agent.VolumeHelper` supports Core's current remote helper requests.
It uses Alpine with the same isolated glibc libraries as Core, preserving the GNU
Rust build. The pinned library source contributes only glibc and its loader cache.
Docker CLI, Buildx, Compose, Git, SSH, CA certificates, Bash and Restic remain
available. Restic is required for the backup helper flow.
Core-only tools (Deno, PostgreSQL client and notification CLI) are not included.
The process runs as root by default for Docker socket and helper compatibility;
an explicit Docker `--user` requires socket access and writable `/app/data`.

From the repository root, build and check the release image with:

```bash
docker build -t citadel-agent:local -f rust/Dockerfile.agent .
bash rust/scripts/test-agent-image.sh citadel-agent:local
bash rust/scripts/test-agent-compatibility.sh citadel-agent:local
```

The smoke script uses disposable containers, a scratch image build and local data.
It checks Direct HTTP/TLS and all Edge profiles, health checks on a custom port,
SIGTERM shutdown, Git submodules, Docker/Compose/Buildx, helper paths and Restic
backup/restore.
It needs Docker and OpenSSL on the host. Override
`CITADEL_AGENT_TEST_DOCKER_SOCKET` only when the test daemon uses another socket.

The compatibility script compiles the compatibility, live Docker and Edge intake
integration tests with the pinned Rust toolchain. It runs the compatibility test
alongside the packaged Agent executable, and the other two suites against the
same isolated services. It creates its
own PostgreSQL database, authenticated registry, and two privileged Docker-in-Docker
daemons on a private bridge network with no published ports. It never mounts the
host Docker socket into those fixtures. All fixture containers, volumes and the
network are removed on exit, including after a failed test.

Coverage includes Direct and both ordinary/Build Pool Edge connections, durable
identity reconnect, real image builds with BuildKit secrets, registry authentication,
push/pull, Compose/Swarm workloads, Swarm mutations, timeout and cancellation.
The worker-node check uses a real Swarm task with manager-observed identity fixtures;
it runs the Agent executable against that worker daemon and checks task rejection,
persisted reconnect and restricted commands. It does not exercise Core's service
installer or replace mixed-version tests against the existing Agent release.
The test prints the Agent's resident memory after connection/readiness; these
local idle samples are not a load benchmark.

To test the complete Core and Agent images through Core's HTTP API:

```bash
docker build -t citadel-core:acceptance -f rust/Dockerfile .
bash rust/scripts/test-agent-acceptance.sh citadel-core:acceptance citadel-agent:local
```

This acceptance test initializes the administrator through HTTP, rejects an Agent
with the wrong signing key, registers Direct and Edge platforms, and applies,
stops, starts and deletes Compose stacks. It checks container inventory, offline
errors, network and volume create/read/delete operations, reconnect, Edge identity
persistence without an enrollment token, recovery after a Core restart, and
revocation. Missing networks and volumes must return HTTP 404 through both connectors.
It also creates a three-node Swarm and uses Core's installer through both Direct
and Edge managers. It verifies digest-pinned node-agent services, worker container
inventory/statistics and inspection, volume/network routing, volume browsing,
manager and worker outage recovery, stale projections, repair, upgrade and removal.
Core has no Docker socket: resource operations must reach the disposable
Docker-in-Docker daemons through an Agent.
Only Core's HTTP port is published, on a random loopback port. PostgreSQL, Agent
data volumes and the network are isolated and removed when the test finishes.
The host needs Docker, Bash and the pinned Rust toolchain.

To also test candidate Core against an existing released Agent, pull an immutable
release reference and pass it as the third argument:

```bash
docker pull "$RELEASED_AGENT_IMAGE"
bash rust/scripts/test-agent-acceptance.sh citadel-core:acceptance citadel-agent:local "$RELEASED_AGENT_IMAGE"
```

`RELEASED_AGENT_IMAGE` must use `repository@sha256:...`. Without it, the runner
explicitly reports that mixed-version compatibility was not tested. Set the
repository variable `CITADEL_AGENT_ROLLBACK_IMAGE` to that reference for native
amd64/arm64 CI. Release publishing requires this gate; it also verifies each
published signature against the publishing workflow identity. Configuring a
baseline does not enable publication or change installation defaults.

Run the same image for inbound Platform or Build Pool connections:

```bash
docker run -d --name citadel-agent --env-file rust/.env.agent.example \
  -p 9000:9000 -v /var/run/docker.sock:/var/run/docker.sock citadel-agent:local
```

Replace the template public key before running it. For Edge, use a configured
copy of `rust/.env.edge.example` or `rust/.env.edge-build-agent.example`, mount a
persistent volume at `/app/data`, and omit the published port. Swarm-node services
use Core's injected identity/bootstrap settings.

Core and Agent share `build/version.rs`: `CITADEL_INFORMATIONAL_VERSION`, then
`CITADEL_VERSION`, then repository `version.json` determine compile-time metadata.
Docker accepts the corresponding `INFORMATIONAL_VERSION` and `VERSION` build args.
`citadel-agent --version` reports informational metadata; Direct info and Edge
enrollment/heartbeats report the same display version as Core (without `+metadata`).
No runtime Git access is required. The built-in `healthcheck` command probes only
loopback, bypasses proxies and redirects, and supports Direct TLS certificates
issued for the public hostname. It reports process liveness even while Core or
Docker is unreachable.

The independent `.github/workflows/agent.yml` pipeline runs on relevant pull
requests and pushes to `main`, release tags, and manual dispatch. It validates
the Agent, contracts and volume helper, builds `rust/Dockerfile.agent` on native
amd64 and arm64 runners, and runs image smoke, isolated compatibility and complete
Core/Agent HTTP acceptance tests on each before allowing publication. It builds
Core as a test fixture without depending on the Core publication pipeline.
`rust/Dockerfile` owns the Core image and contains no Agent stages.
Its build context contains only `rust/` and `version.json`. Release tags must
match `version.json` and belong to `main`. Publication stays disabled until the
Agent compatibility gates pass: then set the repository variable
`CITADEL_RUST_AGENT_RELEASE_ENABLED=true` and retire the competing Agent publisher.
The release job promotes the tested images to
`ghcr.io/<owner>/citadel.agent` and
`docker.io/<DOCKERHUB_NAMESPACE>/citadel-agent`, with version, major.minor and major
aliases. It verifies both architectures, matching alias digests and signs them.
It does not move `latest`. Docker Hub uses the existing namespace/username
variables and token secret; GHCR uses the main repository's package write access.

An opt-in helper test runs real Docker containers through the restricted node
dispatcher. It needs no Swarm cluster or database and removes its own containers
and volumes. From the repository root, build the helper fixture image, then run:

```bash
docker build --target agent-runtime-base -t citadel-agent-helper:test -f rust/Dockerfile.agent .
cd rust
CITADEL_AGENT_HELPER_TEST_IMAGE=citadel-agent-helper:test \
  cargo test --locked -p citadel-agent --lib \
  swarm_volume_browsing_runs_packaged_rust_helper_and_cleans_up -- --ignored
docker image rm citadel-agent-helper:test
```

This checks listing, binary downloads, symlink rejection, restore-volume guards
and cleanup using both helper paths. The fixture target contains the helper but
does not contain the Agent executable; the `agent` target adds that executable.

Shared runtime regression tests use temporary fake daemons and executables:

```bash
cargo test --locked -p citadel-adapters --test docker_transport --test local_runtime
```

These tests do not change local containers or require PostgreSQL. Direct transport
tests exercise authenticated RPCs through a real HTTP/2 listener against temporary
Docker fixtures, including tampering, replay, map fields, TLS and deadlines.

Run a Direct Agent from `rust/` with Core's public signing key:

```bash
HUB_PUBLIC_KEY='<base64 public key>' cargo run --locked -p citadel-agent
```

The local `docker` CLI with its Compose plugin is required for build/stack RPCs.
Core must be able to reach the listener; configure the public key from that Core
installation. Do not use the Core private signing key as `HUB_PUBLIC_KEY`.

An opt-in live test requires Docker and a locally available `redis:latest` image.
It creates uniquely named disposable containers, a network and a volume, then
removes only its own resources. It exercises Core's client, statistics/events,
inspection, terminal/binary exec, logs and deployment apply:

```bash
cargo test --locked -p citadel-agent --test live_docker -- --ignored
```

Run the configuration, Direct transport, TLS and lifecycle checks with:

```bash
cargo test --locked -p citadel-agent
```

TLS tests use `openssl` to create temporary certificates and remove their files
when finished. The tests use ephemeral ports and need no external services.

The opt-in Edge interoperability test runs the Agent against Core's actual
`EdgeIntake`. It needs an **isolated disposable database**: it applies migrations
and creates Platform/Build Pool fixtures. Docker responses are local test fixtures;
it does not contact the development Core or modify workloads.

```bash
CITADEL_AGENT_TEST_DATABASE_URL='<isolated PostgreSQL URL>' \
  cargo test --locked -p citadel-agent --test edge_intake -- --ignored
```

It verifies enrollment, signed reconnects, process restart without a token,
resource identity, revocation, and preservation of credentials during a Core
storage outage. It also checks unary execution, command failures, live events and
cancel/shutdown cleanup for Platform and Build Pool targets. Ordinary Agent tests
require no database.

### Authorization cache ownership

The PostgreSQL identity adapter owns one process-local authorization cache per
connection endpoint/database/user. `PostgresIdentityStore` retains it; mutation
repositories attach to the same weak registry. Actor scope and the global snapshot
share a generation and immutable entry (1,024 actors). Resource ACL contributions
include `None` denials (8,192 keys). LRU indexes and generation metadata are bounded
along with their entries. Both caches have a five-minute safety TTL. Catalog
visibility and transaction-local container authorization remain authoritative SQL.

All application permission writes must acquire `authorization_cache::Mutation`
**before acquiring a database connection**, capture `Impact` before changing old
relationships, and finish through `Mutation::commit`. Commit resolves the new
relationships too. Actor/Team/Role expansion includes service-account members,
disabled principals and removed memberships. Only affected actor generations
change; their scope/snapshot entries are evicted, and old resource entries become
unusable. Rollbacks do not invalidate. Lost commit acknowledgements invalidate
conservatively. A detached commit task retains the cache write fence and publishes
even when its HTTP caller is cancelled. Readers retain the matching read fence
through SQL and cache publication. Do not hold a pool connection while waiting to
enter this fence.

`IdentityService::authorize_resource` checks the cached global grant first.
`permissions_for_resources` deduplicates known IDs, serves hits and resolves all
ACL misses in one `actorid=ANY(...)` query using cached scope. Use that API for
known-ID capability batches; never load an unrestricted catalog and apply it as
an in-memory visibility filter. Git/Registry/Tag catalogs and Build/BuildAgentPool/
AutomationAction known-ID checks use this batching path.

Realtime binds a watch to the same actor generation before its authoritative
subscription check. Changed or evicted generations close the connection and drop
stream guards immediately; periodic authorization checks remain. Cache eviction
can therefore cause a safe reconnect. Runtime observations reuse the connection
authorization lease described below.

Fixed-cardinality runtime families expose scope/resource lookups (iterations)
and hits (units), negative hits, scope/global/resource SQL query counts, and
resident actor invalidations. Tests are `authorization_cache`,
`authorization_mutation_conventions` and the `authorization_cache` adapter unit
module. Database tests require `CITADEL_IDENTITY_DATABASE_URL`; the cancellation test
expects the normal migrated schema. The mutation convention test inventories ACL
SQL owners and rejects unaudited transaction paths.

Immediate invalidation covers application writes within one Core process, matching
the in-process cache ownership. Independent Core writers or manual
SQL updates require coordinated invalidation before sharing cached authorization;
this cache does not provide distributed consistency. Restart Core after manual
ACL maintenance. TTL is a safety bound, not a replacement for commit publication.


### Realtime authorization and committed observation sharing

Each production socket keeps permission decisions for its bound actor generation.
The lease holds at most 1,024 positive/negative decisions, keyed by resource kind,
ID, required level and specific permission. At capacity new decisions fail closed;
no eviction loop turns an event burst into repeated permission SQL. A successful
global decision can satisfy the same resource requirement. Ownership is still
resolved from current persisted rows, and authorized catalog queries retain their
SQL visibility filters. No actor-specific views or permission decisions are shared
between connections.

Runtime events do not reauthenticate a generation-tracked socket. Explicit client
invocations and the configured periodic safety interval authenticate again and
clear the lease; periodic checks revalidate all joined groups, including passive
ones. Actor generation changes cancel the whole connection, including active
streams. Custom readers without an invalidation watch retain event authentication
and uncached permission checks. The process-local consistency restrictions above
still apply; a periodic check is not an external SQL invalidation mechanism.

A published event shares committed raw Platform, full Container inventory, Image
inventory and targeted Container reads through fallible `OnceCell`s. Every reader
checks access before exposing rows, and creates its own view/capability projection.
Cells last only for that event and only match its Platform. Live daemon network/
volume reads and actor-filtered catalogs retain their existing read paths.

Committed container/image/network/volume observations coalesce for 100 ms by
Platform, resource and action. One pending window retains at most 1,024 IDs and
flushes early at capacity. Full inventory refreshes explicitly supersede targeted
container references in the same batch. Claims, completions, logs, terminal data,
security/license notifications and other control changes remain immediate.

Runtime metric families: `RealtimeAuthentication` and `RealtimePermissionMiss`
count executions in iterations; `RealtimePermissionLookup` counts lookups in
iterations and hits in units; `RealtimeSharedRead` counts raw read executions;
`RealtimeAuthorizationInvalidation` counts cancelled connections in units;
`RealtimeObservationInput` and `RealtimeRuntimeInvalidation` count input
observations and published runtime invalidations in units. Labels never include
actor or resource IDs. Run the `platforms_http` tests filtered by `realtime_` with
`CITADEL_PLATFORM_DATABASE_URL` and `--include-ignored --test-threads=1` for the
cross-connection counters and committed revocation gate.

## Appearance preferences

The header palette button and Profile appearance section share one customizer.
Mode, accent, interface font, radius, content width, and density update immediately
and save through `PATCH /api/v1/profile/preferences`. Browser caches are per user;
logout returns to the separate anonymous cache. The authenticated server value
remains authoritative. Failed saves keep the selected local appearance and show
a notification; a later edit retries the pending fields.

Regenerate the Rust OpenAPI before the frontend client:

```bash
cd rust
cargo run --locked -p xtask -- openapi
```

Install frontend dependencies with `npm ci` in `src/Citadel.FrontEnd` first.
The OpenAPI command exports both Rust documents, copies the full `schema/v1.json`
to `src/Citadel.FrontEnd/src/api/schema/swagger.json`, and runs `npm run api:generate`
to regenerate the TypeScript client and resource map from that same document.
`public-v1.json` excludes internal authentication, setup, and administration routes
and is intended for external clients. The frontend uses the full Rust schema without a preference overlay. Correct missing contracts in Rust's DTO descriptors
(or the remaining dynamic JSON schemas), then regenerate; do not edit generated types.

`cargo run --locked -p xtask -- openapi --check` checks the Rust schema artifacts
without requiring Node.js. `npm run api:verify` verifies the generated frontend files.

Shared UI uses runtime CSS variables in `main.css`. Use `AppContent` for page
spacing, `PageHeader` for titles/actions, and `Surface`/`Card` for individual
sections. Use semantic status colors for health; accent selection must never
change the meaning of success, warning, or danger. UI fonts and density do not
change Monaco or terminal monospace metrics.

## Local password policy

`Passwords__MinimumLength` controls new passwords in initial setup (including
bootstrap), user creation, administrator password changes, and profile password
changes. The default is 15; valid settings are 8–128 Unicode characters. The
maximum stays 128. Existing passwords continue to work after raising the minimum.
The browser reads the effective limits from `/api/v1/setup/status`.

Set `Passwords__MinimumLength=8` in `rust/.env` (or `rust/.env.development`
for the development stack), then recreate Core. Compose passes the setting through
its existing `env_file`; Core defaults to 15 when it is absent.
A lower minimum is best paired with `Mfa__Policy=RequiredForAllUsers`; optional MFA
does not protect users who have not enrolled. NIST SP 800-63B-4 recommends at least
15 characters for password-only authentication and permits 8 with MFA.

Spaces, Unicode, password-manager autofill and paste are supported. There are no
mandatory character classes or periodic expiration. Passwords remain Argon2id
hashed. A bundled list of 1,000 common passwords (SecLists; attribution alongside
the data), Citadel defaults, and account-name/email checks reject predictable
choices. This is an offline common-password check, not a complete breached-password
lookup; passwords are never sent to an external service.
