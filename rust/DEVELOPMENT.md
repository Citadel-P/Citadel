# Rust development environment

On Windows, prefer **VS Code in WSL Ubuntu**, with the repository on the Linux
filesystem. Rust and Vite run directly in Ubuntu; Docker Desktop supplies Docker
and PostgreSQL. The Dev Container remains a supported fallback.

This removes the workspace container and Windows bind-mount file watching from
the normal development loop. It does not remove WSL's VM or guarantee a memory
ceiling: Rust compilation, rust-analyzer, Docker workloads and filesystem cache
still consume memory. Cargo builds use two parallel jobs by default, and the
editor runs `cargo check` instead of Clippy on save. Full Clippy checks remain
available in the check task. Development breakpoints and variable inspection
are preserved.

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
   Features that execute external tools also need their Linux executables
   (for example Deno for Automations and Restic for backups).
4. Put the checkout under `~/projects/Citadel`, **not `/mnt/d`**. For a clean
   checkout, clone recursively from your remote. For an existing dirty checkout,
   copy the repository **including `.git` and submodule metadata** into a new,
   empty directory; preserve staged, unstaged and untracked source files. Skip
   disposable `node_modules`, Rust `target`, and .NET `bin`/`obj` artifacts.
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

In WSL, preparation starts only PostgreSQL, with project name `citadel-wsl`,
exposing it on `127.0.0.1:15432`. This is a **new development database**, separate
from the devcontainer's existing database. Runtime data lives under
`~/.local/share/citadel-wsl`; compiler artifacts remain under `rust/target`.
Nothing deletes or migrates your old database, volumes, or checkout. To retain
an existing Citadel instance, use its backup/restore procedure and preserve its
encryption/signing keys and runtime data; don't merely point at an old database
with newly generated keys. A custom `DATABASE_URL` skips starting PostgreSQL.

Stop the old devcontainer API/UI before running WSL to avoid port conflicts.
To stop the WSL development database without deleting its data:

```bash
docker compose -p citadel-wsl -f .devcontainer/compose.yaml \
  -f .devcontainer/compose.wsl.yaml stop postgres
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
**Citadel: Run application (API + UI)** task. The task waits for the API to be
ready before starting Vite.

Open these addresses from the host:

- UI: <http://localhost:5173> or <http://127.0.0.1:5173>
- API health: <http://localhost:8000/health>

The task creates separate **citadel-api** and **citadel-ui** terminal panels.
The API is ready when its panel reports `Rust foundation server listening`.

Only the Dev Container enables file-watcher polling at one-second intervals so Vite
detects edits made on the Windows host. Without polling, mounted files can change
while Vite continues serving an older transformed module, even after a browser
refresh.

To stop both processes, run **Tasks: Terminate Task** from the Command Palette
and select **Citadel: Run application (API + UI)**. Closing or rebuilding the
development container also stops them.

## Debug Rust

1. Open **Run and Debug** with `Ctrl+Shift+D`.
2. Select **Citadel: Debug application**.
3. Add breakpoints in the Rust source.
4. Press `F5`.

This launch configuration starts Vite and the unoptimized Rust API under
CodeLLDB. Press `Shift+F5` to stop both. Do not start the normal application
task at the same time because both workflows use ports 5173 and 8000.

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

The declarative schema in `crates/database/src/schema/schema.sql` is the Rust
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
same `Jwt__Key` and `Secrets__EncryptionKey` external configuration recorded by
the recovery manifest.

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

## Current migration boundary

The React UI contains functionality that has not yet migrated to Rust. A page
can therefore return a deliberate missing or unsupported-operation response
even when the API health endpoint succeeds. The migration ledger and slice
tests remain authoritative for what is currently implemented.

### Container-driven resource status

Daemon container events update the persisted runtime identity and its Deployment
or Compose Stack status in one transaction. Local Core inspects the affected
container; Direct and Edge Agents supply the observed container state. Confirmed
deletions preserve the resource binding long enough to mark the owner degraded.
Status-change activities use the .NET payload names, and realtime invalidations
are published after commit. Duplicate observations do not create duplicate activities.

Full inventory reconciliation remains the recovery path for missed events and
new resource discovery. It runs at startup, after recovery, and at the configured
`JobConfiguration__SwarmReconciliationIntervalSeconds` interval (30 minutes by
default). Platform health is checked every five seconds with a two-second timeout,
three failures before going offline, and two successes before recovery.
`JobConfiguration__MonitoringInterval` controls the readiness probe. An unreachable daemon degrades its Deployments; recovery
refreshes inventory before restoring observed health. Separate node-Agent
container projections are preserved when the manager disconnects.

Swarm task health remains owned by Swarm reconciliation. Existing container,
Deployment Apply, and Stack operation recovery jobs continue to recover expired
operations; event synchronization does not overwrite active operation claims.

### Background job lifecycle and configuration

Core holds a PostgreSQL advisory lease for its background jobs for the entire
process lifetime. A second Core against the same database is rejected before
startup recovery. Stop the previous backend before starting a new binary.
Startup marks abandoned build, backup, restore and automation executions as
interrupted, including when their dispatch is disabled. Periodic recovery
continues to respect active execution deadlines and operation claims.

| Setting | Default | Purpose |
| --- | --- | --- |
| `JobConfiguration__FlashInterval` | `60` seconds | Flush CPU/RAM threshold observations even when sample input is idle; matches shipped .NET settings |
| `JobConfiguration__BatchSize` | `500` | Flush threshold observations early at this persisted sample count |
| `EdgeAgent__NodeAgentRemovalGraceMinutes` | `10` | Revoke credentials only for absent Swarm nodes beyond this grace period |
| `EdgeAgent__SupportedNodeArchitectures__0`, `__1`, … | `amd64`, `arm64` | Architectures requiring node-agent coverage; `x86_64`/`aarch64` aliases normalize |
| `Builds__MaxParallelRuns` | `4` | Concurrent build executions |
| `Builds__RunCleanupEnabled` | `true` | Scheduled build retention |
| `Builds__RunRetentionDays` | `90` | Terminal build retention; nonpositive disables cleanup |
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

- `CITADEL_PHASE6_DATABASE_URL`: disposable PostgreSQL database.
- `CITADEL_PHASE6_DOCKER_SOCKET`: isolated manager's Unix socket.
- `DOCKER_HOST`: the same manager socket, for the Docker CLI.
- `CITADEL_PHASE6_RUNTIME_IMAGE`: image present on both nodes; the fixture uses shell/sleep (tested with `alpine:3.21`).
- `CITADEL_PHASE6_SWARM_WORKER`: outer Docker-in-Docker worker container, labelled `citadel.test=jobs-live`. The test checks this label before stopping it; outer container commands use the default Docker connection with `DOCKER_HOST` removed.
- A current Docker CLI supporting `stack deploy --detach=false` on the test-process `PATH`, and a writable process-specific `TMPDIR`.

For Docker-in-Docker, bind a dedicated socket directory and add a socket listener there. Do not share the entire `/var/run`: persisted containerd PID files can prevent the test worker restarting. Keep the DinD network private and clean up only the test containers, their volumes and that network afterward.

From `rust/`:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters \
  --test stack_runtime_local swarm_material_capture -- --include-ignored --nocapture
```

### Production Edge Agent jobs regression

`crates/adapters/tests/edge_agent_jobs_acceptance.rs` tests the actual Agent binary against Rust Core's Edge transport and persistence. Build the Agent image from its separate checkout, then run from `rust/`:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters \
  --test edge_agent_jobs_acceptance -- --ignored --nocapture
```

Supply these environment variables:

- `CITADEL_PHASE7_DATABASE_URL`: a disposable PostgreSQL database.
- `CITADEL_PHASE7_AGENT_IMAGE`: the production Agent image built for this checkout.
- `CITADEL_PHASE7_AGENT_NETWORK`: the isolated Docker network.
- `CITADEL_PHASE7_AGENT_HOST`: an address the Agent container can use to reach the test process's listener (`host.docker.internal` worked with Docker Desktop/WSL).
- `CITADEL_PHASE6_SWARM_MANAGER`: the disposable DinD container, labelled `citadel.test=jobs-live`.
- `CITADEL_PHASE6_DOCKER_SOCKET`: that isolated daemon's socket exposed at a host bind-mount path.

The test explicitly uses the outer Docker socket `/var/run/docker.sock` for fixture management. The Agent mounts only the supplied isolated daemon socket. It creates a uniquely named Agent and workload, restarts that Agent, exercises reconnect/revocation, and removes its containers. It does not restart Core or use a development database. The fixture checks the DinD label before any runtime mutation.
