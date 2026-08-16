# Phase 0A commands

Run these commands from the repository root in PowerShell.

```powershell
# Rebuild and verify the generated Docker subset.
docker run --rm --mount type=bind,source=${PWD},target=/repo -w /repo/rust `
  rust:1.97.1-bookworm cargo run --locked -p xtask -- generate-docker --check

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
