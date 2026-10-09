# Rust development and verification commands

Packaged Agent checks (Bash, Docker and OpenSSL):

```bash
docker build -t citadel-agent:local -f Dockerfile.agent .
bash test/scripts/test-agent-image.sh citadel-agent:local
bash test/scripts/test-agent-compatibility.sh citadel-agent:local
docker build -t citadel-core:acceptance -f Dockerfile .
bash test/scripts/test-agent-acceptance.sh citadel-core:acceptance citadel-agent:local
# Optional third argument: an already-pulled released Agent pinned by digest.
bash test/scripts/test-agent-acceptance.sh citadel-core:acceptance citadel-agent:local "$RELEASED_AGENT_IMAGE"
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
The Swarm image-transfer regression also checks digest-pinned inputs in a clean
Docker daemon, verifying that save/load preserves the selected image identity.
Run it independently with:

```bash
cargo test --locked -p citadel-agent --test acceptance stages_digest_pinned_image_without_registry_digest_metadata -- --ignored --nocapture
```

See [Agent development](../docs/DEVELOPMENT.md#agent-development) for runtime
configuration and local image setup.

To reuse compatibility-test compilation locally, set
`CITADEL_AGENT_COMPATIBILITY_CACHE_DIR` to a dedicated directory outside the
checkout before running `test-agent-compatibility.sh`. Use a separate directory
for each architecture and pinned builder, and do not share it between concurrent
runs. The script retains this explicitly supplied directory while removing its
temporary fixtures. Without this variable, compilation remains disposable.

Run these commands from the repository root in PowerShell.

Docker-based Cargo test suites use a unique `citadel-rust-test-<id>` build
volume per invocation and remove it in `finally`, including after test failures.
Dependencies/toolchains remain cached; compiled output is reused within a suite
but not between suites. A forcibly killed PowerShell process or unavailable
Docker daemon can prevent cleanup; the script warns when Docker refuses removal.
Cleanup never prunes unrelated volumes or forcibly removes an in-use volume.
Run `./test/scripts/Test-BuildCache.ps1` to test these safety rules without Docker.

```powershell
# Rebuild and verify the generated Docker subset.
docker run --rm --mount type=bind,source=${PWD},target=/repo -w /repo `
  rust:1.97.1-bookworm cargo run --locked -p xtask -- docker --check

# Format, lint, and run unit/Unix-socket compatibility tests.
./test/scripts/Test-Workspace.ps1

# Run the PostgreSQL Actor-authorization fixture against an isolated database.
./test/scripts/Test-AuthorizedRead.ps1

# Build the pinned release container.
./test/performance/Build-MeasurementImage.ps1

# Short local measurement.
./test/performance/Measure-Server.ps1 -DurationHours 0.1

```

Measurement output is written to the ignored `artifacts/` directory. The
Compose file enforces `memory.max=100 MiB` and disables swap for the measurement container.

Windows-native Cargo commands additionally require the Visual Studio Build
Tools C++ workload because the installed Rust host is `x86_64-pc-windows-msvc`.
The scripts use the pinned Linux toolchain so the release path does not depend
on the host linker.

## Source ownership and contracts

Run `bash test/scripts/check-source-ownership.sh` to verify source ownership and
`cargo test --locked -p citadel-contracts` from the repository root to check the protocol
inventory. Database generation uses the Rust declarative schema; see
[database development](../docs/DEVELOPMENT.md#database-schema-changes).

## Test fixture settings

Test names and fixture variables describe the subsystem they exercise:

| Fixture | Database variable |
| --- | --- |
| Authorized reads | `CITADEL_AUTHORIZED_READ_DATABASE_URL` |
| Identity and access | `CITADEL_IDENTITY_DATABASE_URL` |
| Platform inventory and reads | `CITADEL_PLATFORM_DATABASE_URL` |
| Resource metadata | `CITADEL_METADATA_DATABASE_URL` |
| Deployments, Stacks and Swarm workloads | `CITADEL_WORKLOAD_DATABASE_URL` |
| Git, Builds, Backups, Automation and Alerts execution | `CITADEL_EXECUTION_DATABASE_URL` |
| Statistics persistence | `CITADEL_STATISTICS_DATABASE_URL` |

These must point to disposable test databases. The PowerShell harnesses set them
for their fixture processes. Recovery tests use separate source and target databases.
Run ignored database tests serially (`--ignored --test-threads=1`).

The `platforms_http` suite requires `CREATE DATABASE` permission on
`CITADEL_PLATFORM_DATABASE_URL`. Each test creates and removes its own database
so administrator list and realtime reads cannot pick up another test's fixtures.

`Test-Workspace.ps1` runs formatting, Clippy and workspace tests. The server's
optional Docker and Agent probes are `docker-smoke` and `agent-smoke`.
`realtime-client.mjs` takes its token from `CITADEL_REALTIME_TEST_TOKEN` and connects
to the URL supplied through `--url`; the Core endpoint is `/api/v1/realtime`.

## foundation

Run `./test/scripts/Test-Foundation.ps1` from the repository root to
exercise the embedded Rust migration runner against a disposable PostgreSQL
database and verify the generated database, Docker, OpenAPI, and frontend
artifacts. The script tests concurrent startup, restart idempotence, seed data,
and checksum refusal. It removes its uniquely named container and network when
the run finishes.

## identity and access

Run `./test/scripts/Test-Identity.ps1` from the repository root. It uses
disposable PostgreSQL databases and server containers to verify identity and access flows,
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

Run `./test/scripts/Test-Frontend.ps1` to build the production React
bundle, serve it from the Rust process, and execute browser compatibility tests
against disposable PostgreSQL and Keycloak services. The default remains the
existing Keycloak OIDC suite. Pass one or more repository-relative Playwright
files through `-TestFiles` to exercise other screens, for example:

```powershell
./test/scripts/Test-Frontend.ps1 -TestFiles @(
  'tests/compatibility/oidc.spec.ts',
  'tests/compatibility/access.spec.ts',
  'tests/smoke/licensing.spec.ts'
)
```

The combined identity browser run covers first-run setup, OIDC redirect and
callback, browser-session restoration and logout, User/Team production forms,
Roles, Community-gated Service Accounts, License rendering, and OIDC provider
administration. For repeated browser runs, opt into a retained compiler cache
with `-BuildCacheVolume citadel-rust-e2e-local` on the first build, then pass that
same option with `-SkipBuild`. This explicitly retained cache is not removed by
the script; delete it when finished. Without this opt-in, the default temporary
cache is always cleaned, so `-SkipBuild` requires an explicit retained volume.

## Platform and Docker reads

Run `./test/scripts/Test-PlatformReads.ps1` from the repository root. It creates
a disposable PostgreSQL database and exercises Local Docker and signed Agent
mapping/transport, transactional inventory and statistics persistence, the
real authorized Axum Platform/Docker/Swarm read routes, reconciliation worker
behavior, and bounded authenticated realtime subscriptions. The script removes
its uniquely named PostgreSQL container and network in `finally`.

## simple mutations and metadata

Run `./test/scripts/Test-ResourceMetadata.ps1` from the repository root. It
creates one disposable PostgreSQL database and verifies Tag, Registry, Git
repository, resource-binding, internal/external Secret, and Secret-provider
lifecycles through the real store and Axum routes. It also covers Local and
Agent Network/Volume mutation parity, mutation retry safety, authorization,
activities, realtime invalidations, and deterministic OpenAPI/Docker contract
generation. The script removes its uniquely named PostgreSQL container and
network in `finally`.

## Deployment CRUD

Run `./test/scripts/Test-Deployments.ps1` from the repository root. It
creates a disposable PostgreSQL database and verifies the Deployment domain,
persisted JSON contracts, authorized SQLx CRUD/duplicate/delete
transactions, typed Activity evidence, the real Axum endpoints, deletion claim
rollback and timeout behavior, Local Unix-socket and signed Agent container
delete transport equivalence, and deterministic OpenAPI/Docker generation. It
also exercises CRUD/config regression scenarios:
merge-patch validation, duplicate conflicts/failures, Local-image projection,
direct and Team ACL filtering, Platform authorization, concurrency exclusion,
and recovery after request cancellation. Container deployment, progress and
reconciliation are covered by `Test-DeploymentApply.ps1`. The script removes its uniquely named PostgreSQL container
and network in `finally`.

## external execution and Git accounts

Run `./test/scripts/Test-ExternalExecution.ps1` from the repository
root. It verifies the bounded child-process primitive, Git argument and cache
safety, encrypted Git-account persistence, authorized HTTP lifecycle, and
generated OpenAPI parity against a disposable PostgreSQL database. The gate also
covers Git synchronization, Automation/Build/Backup execution, Alert persistence,
Edge enrollment/transport, node projection isolation and authorized HTTP/realtime
responses. Agent command responders are test peers, not released Agents.

For the real Local Docker/Restic volume round trip, start the development
container and run `./test/scripts/Test-LocalBackup.ps1` from the host.
Use `-DevContainer <name>` if the workspace container has a different name.
It creates a separate disposable PostgreSQL database and uniquely named Docker
volumes, backs up a file, restores it at the volume root, verifies persisted
results and rejects an unapproved overwrite. It uses the existing development
compiler cache and does not build a Citadel image or touch the development DB.
The test removes its fixture volumes; the script removes its fixture database.
This does not replace the Agent/Edge, RustFS or multi-node acceptance gates.

Normal browser development (`dev.sh compose-up`) builds and mounts release Core
by default. Use `CITADEL_COMPOSE_PROFILE=dev` for faster unoptimized builds; use
the default release profile for comparable CPU/I/O measurements.
Native debugger tasks retain their development profile.
