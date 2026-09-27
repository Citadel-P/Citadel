# Rust migration commands

Agent release image checks (Bash, Docker and OpenSSL):

```bash
docker build -t citadel-agent:local -f rust/Dockerfile.agent .
bash rust/scripts/test-agent-image.sh citadel-agent:local
bash rust/scripts/test-agent-compatibility.sh citadel-agent:local
docker build -t citadel-core:acceptance -f rust/Dockerfile .
bash rust/scripts/test-agent-acceptance.sh citadel-core:acceptance citadel-agent:local
# Optional third argument: an already-pulled released Agent pinned by digest.
# CI requires this baseline before release publication.
bash rust/scripts/test-agent-acceptance.sh citadel-core:acceptance citadel-agent:local "$RELEASED_AGENT_IMAGE"
```

The smoke test removes its disposable containers and build image. The compatibility,
live Docker and Edge intake suites require the pinned Rust toolchain and share a private registry, database
and two privileged Docker-in-Docker daemons; it removes only its own fixtures.
The acceptance test uses the complete Core and Agent images and calls Core's HTTP
API for setup, Direct/Edge registration, stacks, inventory, reconnect, Core restart
and revocation. It also exercises Core's three-node Swarm installer and worker
routing through Direct and Edge managers, including outage recovery, repair,
upgrade and removal. It needs Rust, creates its own database and Docker daemons,
and removes only its own containers, volumes and network.
See
[Agent development](../DEVELOPMENT.md#agent-development) for runtime configuration
and the release publication gate.

Run these commands from the repository root in PowerShell.

Docker-based Cargo test suites use a unique `citadel-rust-test-<id>` build
volume per invocation and remove it in `finally`, including after test failures.
Dependencies/toolchains remain cached; compiled output is reused within a suite
but not between suites. A forcibly killed PowerShell process or unavailable
Docker daemon can prevent cleanup; the script warns when Docker refuses removal.
Cleanup never prunes unrelated volumes or forcibly removes an in-use volume.
Run `./rust/scripts/Test-BuildCache.ps1` to test these safety rules without Docker.

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

The original inventories are archived evidence. Their generators have been
retired. Run `bash rust/scripts/check-source-ownership.sh` to verify source
ownership and `cargo test --locked -p citadel-contracts` from `rust/` to check
the accepted protocol baseline.

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
disposable PostgreSQL databases and server containers to verify Phase 3A-R,
including local identity, authenticated application information, current
profile, lazy and persisted profile preferences, active browser-session listing
and revocation, password changes, atomic safe Activity evidence, authorized
Activity list/detail compatibility, Actor authorization, Service Account ACL/token,
license administration, concurrency, and restart behavior. The run executes
the versioned authorization matrix through the Rust PostgreSQL implementation.
It also runs the dedicated
database-backed Axum User, Team, Role, License, MFA, and OIDC endpoint suites for reads and administrator mutations,
including conflicts, assignments, password/session invalidation, safe Activity
evidence, rollback, and concurrent last-administrator protection. The HTTP flow
proves User and Team create/patch/rename persistence, Role permission
reduction/rename persistence, and stable License instance identity across restart and deletion
after restart. All resources are uniquely named and removed in the script's `finally`
block.

The OIDC portion also runs a local signed-JWT issuer fixture. It validates the
real discovery, token, and JWKS protocol adapter without relying on a public
identity provider or storing provider tokens.

Run `./rust/scripts/Test-Phase3Frontend.ps1` to build the production React
bundle, serve it from the Rust process, and execute browser compatibility tests
against disposable PostgreSQL and Keycloak services. The default remains the
existing Keycloak OIDC suite. Pass one or more repository-relative Playwright
files through `-TestFiles` to exercise other migrated screens, for example:

```powershell
./rust/scripts/Test-Phase3Frontend.ps1 -TestFiles @(
  'tests/compatibility/oidc.spec.ts',
  'tests/compatibility/access.spec.ts',
  'tests/smoke/licensing.spec.ts'
)
```

The combined Phase 3 browser run covers first-run setup, OIDC redirect and
callback, browser-session restoration and logout, User/Team production forms,
Roles, Community-gated Service Accounts, License rendering, and OIDC provider
administration. For repeated browser runs, opt into a retained compiler cache
with `-BuildCacheVolume citadel-rust-e2e-local` on the first build, then pass that
same option with `-SkipBuild`. This explicitly retained cache is not removed by
the script; delete it when finished. Without this opt-in, the default temporary
cache is always cleaned, so `-SkipBuild` requires an explicit retained volume.

# Phase 4 Platform and Docker reads

Run `./rust/scripts/Test-Phase4Reads.ps1` from the repository root. It creates
a disposable PostgreSQL database and exercises Local Docker and signed Agent
mapping/transport, transactional inventory and statistics persistence, the
real authorized Axum Platform/Docker/Swarm read routes, reconciliation worker
behavior, and bounded authenticated realtime subscriptions. The script removes
its uniquely named PostgreSQL container and network in `finally`.

# Phase 5 simple mutations and metadata

Run `./rust/scripts/Test-Phase5Metadata.ps1` from the repository root. It
creates one disposable PostgreSQL database and verifies Tag, Registry, Git
repository, resource-binding, internal/external Secret, and Secret-provider
lifecycles through the real store and Axum routes. It also covers Local and
Agent Network/Volume mutation parity, mutation retry safety, authorization,
activities, realtime invalidations, and deterministic OpenAPI/Docker contract
generation. The script removes its uniquely named PostgreSQL container and
network in `finally`.

# Phase 6A Deployment CRUD

Run `./rust/scripts/Test-Phase6ADeployments.ps1` from the repository root. It
creates a disposable PostgreSQL database and verifies the Deployment domain,
.NET-compatible persisted JSON, authorized SQLx CRUD/duplicate/delete
transactions, typed Activity evidence, the real Axum endpoints, deletion claim
rollback and timeout behavior, Local Unix-socket and signed Agent container
delete transport equivalence, and deterministic OpenAPI/Docker generation. It
also exercises the complete applicable .NET CRUD/config characterization set:
merge-patch validation, duplicate conflicts/failures, Local-image projection,
direct and Team ACL filtering, Platform authorization, concurrency exclusion,
and recovery after request cancellation. The .NET acceptance project contains
no standalone Deployment CRUD scenario to port. It
does not deploy a container: Deployment Apply, progress, and reconciliation
belong to Phase 6B. The script removes its uniquely named PostgreSQL container
and network in `finally`.

# Phase 7A external execution and Git accounts

Run `./rust/scripts/Test-Phase7AExternalExecution.ps1` from the repository
root. It verifies the bounded child-process primitive, Git argument and cache
safety, encrypted Git-account persistence, authorized HTTP lifecycle, and
generated OpenAPI parity against a disposable PostgreSQL database. The gate also
covers Git synchronization, Automation/Build/Backup execution, Alert persistence,
Edge enrollment/transport, node projection isolation and authorized HTTP/realtime
responses. Agent command responders are test peers, not released Agents.

For the real Local Docker/Restic volume round trip, start the development
container and run `./rust/scripts/Test-Phase7LocalBackup.ps1` from the host.
Use `-DevContainer <name>` if the workspace container has a different name.
It creates a separate disposable PostgreSQL database and uniquely named Docker
volumes, backs up a file, restores it at the volume root, verifies persisted
results and rejects an unapproved overwrite. It uses the existing development
compiler cache and does not build a Citadel image or touch the development DB.
The test removes its fixture volumes; the script removes its fixture database.
This does not replace the Agent/Edge, RustFS or multi-node acceptance gates.

Normal browser development (`dev.sh compose-up`) builds and mounts release Core
by default. Use `CITADEL_COMPOSE_PROFILE=dev` for faster unoptimized builds; use
the default release profile when comparing CPU/I/O with a production .NET image.
Native debugger tasks retain their development profile.
