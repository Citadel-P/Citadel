# Rust migration commands

Run these commands from the repository root in PowerShell.

```powershell
# Rebuild and verify the generated Docker subset.
docker run --rm --mount type=bind,source=${PWD},target=/repo -w /repo/rust `
  rust:1.97.1-bookworm cargo run --locked -p xtask -- docker --check

# Format, lint, and run unit/Unix-socket compatibility tests.
./rust/scripts/Test-Phase0A.ps1

# Run the PostgreSQL Actor-authorization fixture against an isolated database.
./rust/scripts/Test-Phase0AAuthorizedRead.ps1

# Build the active .NET Agent and prove the signed Phase 0B handshake, read,
# stream, cancellation, and Local/Agent equivalence.
./rust/scripts/Test-Phase0BAgentInterop.ps1

# Prove the Phase 0C versioned WebSocket protocol, PostgreSQL authorization,
# bounded overflow/resync, slow/reconnecting clients, current .NET Agent,
# local statistics, 20-container workload, cgroup limit, and clean shutdown.
./rust/scripts/Test-Phase0CRealtime.ps1

# Build the pinned release container.
./rust/scripts/Build-Phase0A.ps1

# Short local measurement.
./rust/scripts/Measure-Phase0A.ps1 -DurationHours 0.1

# Run the complete combined decision duration only when requesting the formal
# Phase 0 decision. Short runs are sufficient for local protocol verification.
./rust/scripts/Test-Phase0CRealtime.ps1 -DurationSeconds 86400 -SkipBuild
```

Measurement output is written to the ignored `rust/artifacts/` directory. The
Compose file enforces `memory.max=100 MiB` and disables swap for the prototype.

Windows-native Cargo commands additionally require the Visual Studio Build
Tools C++ workload because the installed Rust host is `x86_64-pc-windows-msvc`.
The scripts use the pinned Linux toolchain so the release path does not depend
on the host linker.

## Phase 1 inventory and contracts

```powershell
# Regenerate deterministic inventories and contract hashes.
./rust/scripts/Generate-Phase1Inventory.ps1

# Fail when generated inventories drift or HTTP/protobuf/specification coverage
# is inconsistent.
./rust/scripts/Test-Phase1Inventory.ps1
```

The selected schema-tool proof imports the generated .NET baseline without EF
history metadata, retains all product seeds, and proves structural convergence:

```powershell
./rust/scripts/Evaluate-Phase1DpmCandidate.ps1
```

Rejected candidate evaluations remain as reproducible negative evidence:

```powershell
# Exits non-zero because Atlas Community converts rename intent to drop/add.
./rust/scripts/Evaluate-Phase1AtlasCandidate.ps1

# Exits non-zero because Drizzle cannot converge its generated 83-table schema.
./rust/scripts/Evaluate-Phase1DrizzleCandidate.ps1
```

All evaluations use exact disposable Docker resource names and temporary
directories and clean them in `finally` blocks. They do not touch Citadel's
configured PostgreSQL database.

# Phase 2 foundation

Run `./rust/scripts/Test-Phase2Foundation.ps1` from the repository root to
exercise the embedded Rust migration runner against a disposable PostgreSQL
database and verify the generated database, Docker, OpenAPI, and frontend
artifacts. The script tests concurrent startup, restart idempotence, seed data,
and checksum refusal. It removes its uniquely named container and network when
the run finishes.

# Phase 3 identity and access

Run `./rust/scripts/Test-Phase3Identity.ps1` from the repository root. It uses
disposable PostgreSQL databases and server containers to verify the Phase 3A
local identity, session, Actor authorization, Service Account ACL/token, license
gate, concurrency, and restart behavior. All resources are uniquely named and
removed in the script's `finally` block.
