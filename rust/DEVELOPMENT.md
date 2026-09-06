# Rust development environment

The VS Code Dev Container is the supported interactive environment for the
Rust migration. It runs Citadel in Linux with PostgreSQL, Docker access, the
React frontend, and the Rust debugger while the repository remains on the host.

## Prerequisites

- Docker Desktop with Linux containers enabled
- Visual Studio Code
- The **Dev Containers** VS Code extension

Rust, Node.js, CodeLLDB, rust-analyzer, and the frontend dependencies are
installed inside the development container.

## Open the development container

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

For normal browser testing, press `Ctrl+Shift+B`. This runs the default
**Citadel: Run application (API + UI)** task. The task waits for the API to be
ready before starting Vite.

Open these addresses from the host:

- UI: <http://localhost:5173> or <http://127.0.0.1:5173>
- API health: <http://localhost:8000/health>

The task creates separate **citadel-api** and **citadel-ui** terminal panels.
The API is ready when its panel reports `Rust foundation server listening`.

The Dev Container enables file-watcher polling at one-second intervals so Vite
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

Use separate terminals inside the development container when investigating one
side of the application:

```bash
cd /workspace/rust
cargo run --locked -p citadel-server -- serve
```

```bash
cd /workspace/src/Citadel.FrontEnd
npm run dev -- --host 0.0.0.0 --port 5173 --strictPort
```

The second command runs only the UI. API-backed pages will report that Citadel
is unreachable unless the first command or the debugger is also running.

## Checks

Run the committed tasks from **Terminal > Run Task**:

- **Citadel: Check Rust workspace** runs formatting, Clippy, and Rust tests.
- **Citadel: Check frontend** runs frontend lint and unit tests.
- **Citadel: Initialize development dependencies** refreshes locked Cargo and
  npm dependencies when their lockfiles change.

## Database schema changes

The declarative schema in `crates/database/src/schema/schema.sql` is the Rust
database authority. Until the first Rust release, keep a single generated
baseline and fold schema changes into it; do not hand-write migration SQL:

```bash
cd /workspace/rust
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
