# Citadel tests

Use this guide to choose checks and prepare safe fixtures. Individual assertions
and regression cases live beside their test suites; this file is not a test-result
log. Setup is in [Development](../docs/DEVELOPMENT.md), design rules in
[Architecture](../docs/ARCHITECTURE.md), and CI coverage in [CI](../docs/CI.md).
All commands below run from the repository root.

## Isolation first

- Integration tests must use disposable databases and Docker resources. Never
  point a harness at a personal, demo, shared or production installation.
- `SQLX_OFFLINE=true` selects checked-in compile-time metadata; it does **not**
  isolate runtime database access. Cargo `--offline` is a separate dependency flag.
- Opt-in tests use `--ignored`; inspect the suite's fixture requirements before
  running them. Run database tests serially with `--test-threads=1`, and use a fresh
  database per suite unless its fixture explicitly creates isolated children.
- Recovery/restore tests need distinct disposable source and target databases.
  Preserve real application volumes, credentials and migration history.
- Keep credentials, captures and operational journals outside the repository.
  Cleanup must target only the harness's own resources. Privileged Docker-in-Docker
  fixtures isolate test daemons but still require a trusted test host.

## Fast checks

For a focused Rust change, select the crate/module/test target rather than running
all external-service fixtures:

```bash
cargo fmt --all -- --check
SQLX_OFFLINE=true cargo test --locked -p citadel-platforms --lib
# Replace the crate and filter with the area being changed.
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --lib persistence::postgres::platforms
bash test/scripts/test-dev.sh
```

For broader Rust changes, use the workspace check/Clippy/test commands in
[Development](../docs/DEVELOPMENT.md#checks). Ordinary `cargo test` does not run
ignored integration tests and does not prove real Docker or database behavior.

## Source ownership and contracts

Run these when changing module ownership, dependencies or resource wiring:

```bash
bash test/scripts/check-source-ownership.sh
SQLX_OFFLINE=true cargo test --locked -p citadel-server --test resource_structure --test resource_architecture --test workload_architecture --test multi_resource_architecture
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test architecture
npm --prefix src/frontend run test:run -- src/test/architecture.test.ts
```

For API changes, complete [generation and verification](../docs/DEVELOPMENT.md#api-changes)
**before** frontend tests, lint or type checks. `api:verify` temporarily rewrites
outputs; do not run checks concurrently with it. Rust owns schemas and generated
TypeScript resources. Agent/Docker contract changes also require their generated
contract tests and Direct/Edge mappings; see [Agent protocol](../docs/agent-protocol.md).

## Choose a suite

These are entry points, not an exhaustive list. Integration targets use
`cargo test --locked -p <crate> --test <target>`; append `-- --list` to discover
cases, or a case/module filter to run a focused subset. Inspect the target's helper
modules for required fixtures and explicit opt-ins.

| Area | Primary test targets / location |
| --- | --- |
| Identity, roles and authorization | Adapters: `identity_access`, `authorized_read`, `authorization_cache`, `secret_use_authorization`; Server: `users_http`, `roles_http`, `mfa_http`, `service_accounts_http` |
| Instance configuration and job diagnostics | Server: `configuration`, `mfa_http`, Jobs/realtime unit tests; Adapters: `job_history_persistence` (fresh statistics fixture; retention, compaction and seven-day workload audit); database migration parity |
| Platform inventory, health and reads | Adapters: `platform_inventory_persistence`, `container_inspection`; Server: `platforms_http`, `platform_creation_http` |
| Statistics, retention and query work | Adapters: `stats_batch_persistence`, `maintenance_retention`, `connection_pool`, `idle_worker_queries` |
| Resource metadata | Adapters: `resource_metadata_persistence`; Server: `resources_http`, `registries_http` |
| Deployments, Stacks and Swarm | Adapters: `deployment_persistence`, `deployment_apply_persistence`, `stack_persistence`, `swarm_service_persistence`; Server: `deployments_http`, `stacks_http`, `swarm_services_http` |
| Builds, Backups, Restores and Scripts | Adapters: `build_execution`, `backup_execution`, `script_execution`, `git_repository_execution`; Server: `execution_resources_http`, `script_http_execution` |
| Runtime and transport | `citadel-docker`: `docker_transport`; Adapters: `agent_transport`, `edge_transport`, `daemon_event_compatibility`, `swarm_inventory_transport` |
| Resource schedules | Server coordinator tests in `workers::schedules`, PostgreSQL admission tests in `resource_schedules`, and the Instance `schedules.test.tsx` suite |
| Updates and recovery | `citadel-updates` unit tests; Adapters: `instance_update_journal`, `instance_updates`, `update_preflight`, `agent_updates`; Server: `update_release_cli`, `bootstrap_process` |
| Schema and migration parity | `citadel-database`: `migrations`; [parity cases](../src/infrastructure/database/tests/parity/) |
| Frontend | Tests beside the affected feature/shared component; [architecture test](../src/frontend/src/test/architecture.test.ts) |

Browse [adapter tests](../src/infrastructure/adapters/tests/),
[Server tests](../src/server/tests/) and [harness scripts](scripts/) for additional
acceptance suites. Feature unit tests live with their service implementations.
Use real-runtime acceptance when mocks cannot establish cancellation, cleanup,
transport or durable recovery behavior.

## Test fixture settings

Use PostgreSQL compatible with the current catalog. Set only the variables needed
by the selected suite, pointing to **disposable** databases:

| Fixture | Variable |
| --- | --- |
| Authorized reads | `CITADEL_AUTHORIZED_READ_DATABASE_URL` |
| Identity/access | `CITADEL_IDENTITY_DATABASE_URL` |
| Platform inventory/HTTP | `CITADEL_PLATFORM_DATABASE_URL` |
| Resource metadata | `CITADEL_METADATA_DATABASE_URL` |
| Deployment/Stack/Swarm persistence | `CITADEL_WORKLOAD_DATABASE_URL` |
| Build/Backup/Restore/Script/Alert execution | `CITADEL_EXECUTION_DATABASE_URL` |
| Statistics | `CITADEL_STATISTICS_DATABASE_URL` |
| Migration parity and bootstrap | `CITADEL_TEST_DATABASE_URL` |
| Instance update state/preflight | `CITADEL_UPDATE_TEST_DATABASE_URL` |
| Agent job acceptance | `CITADEL_AGENT_TEST_DATABASE_URL` |
| Token safety | `CITADEL_TOKEN_SAFETY_DATABASE_URL` |
| Checkpoint restore source / target | `CITADEL_CHECKPOINT_DATABASE_URL` / `CITADEL_CHECKPOINT_TARGET_DATABASE_URL` |

The seven-day `job_history_persistence` audit requires a disposable PostgreSQL
server started with `-c shared_preload_libraries=pg_stat_statements` and permission
to create that extension. Run this fixture serially; it resets statement statistics.

Platform HTTP, migration/bootstrap and statistics cases that create child
databases need `CREATE DATABASE` permission on a disposable PostgreSQL server.
Update/checkpoint database names must start with `citadel_update_test_` where
required by their safety guards. Checkpoint creation uses
`CITADEL_UPDATE_TEST_DATABASE_URL` as its source; standalone recovery uses
`CITADEL_CHECKPOINT_DATABASE_URL`. Set `CITADEL_PG_DUMP` and `CITADEL_PG_RESTORE`
to compatible PostgreSQL executables for real checkpoint tests.

Example after provisioning the appropriate fresh fixture and exporting its URL:

```bash
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test stats_batch_persistence -- --ignored --test-threads=1
# Use a separate migration fixture for this command:
SQLX_OFFLINE=true cargo test --locked -p citadel-database --test migrations -- --ignored --test-threads=1
```

Confirm that the intended cases actually ran; a missing fixture must not be
mistaken for successful database validation. Existing `Test-*.ps1` harnesses
provision subsystem fixtures and export their variables; read the selected
script's parameters rather than borrowing an installation's connection string.

## Statistics and performance checks

Use [stats_batch_persistence](../src/infrastructure/adapters/tests/stats_batch_persistence/)
for storage, compaction, range correctness and query plans. Its `history_plans`,
`query_efficiency` and storage cases exercise raw, summarized and mixed windows,
partial boundaries, resets/nulls, removed sources, pending alerts and maintenance
rollback. The migration parity suite verifies fresh and upgraded schemas.

For health/inventory, use `platform_inventory_persistence` and `platforms_http`.
For collector payloads, use Docker `docker_transport` and Agent collection tests.
Reduced collector responses must not break full inspection consumers. Pool and
idle-worker suites cover recurring connection/claim overhead separately from
history queries.

Measure realistic small installations first. Cover every supported time preset
and arbitrary boundaries, both custom and generic plans, and production query
settings. Record examined rows/buffers, temporary writes, WAL, elapsed time and
result equivalence as relevant; keep hardware-specific timings out of correctness
assertions. Bounded work and correct results matter more than one warm-cache run.
Distinguish PostgreSQL allocations from filesystem cache when interpreting memory.

Run load probes and `EXPLAIN ANALYZE` experiments on disposable fixtures, not a
live demo. Keep before/after captures outside the checkout and summarize findings
in the change review, rather than appending benchmark histories here.

## Docker and Agent tests

Requires Bash, Docker, OpenSSL and the pinned Rust toolchain. Build the current
Core/Agent pair, then increase scope as needed:

```bash
docker build -t citadel-agent:local -f Dockerfile.agent .
bash test/scripts/test-agent-image.sh citadel-agent:local
bash test/scripts/test-agent-compatibility.sh citadel-agent:local
docker build -t citadel-core:acceptance -f Dockerfile .
bash test/scripts/test-agent-acceptance.sh citadel-core:acceptance citadel-agent:local
```

The compatibility/acceptance harnesses create disposable registries, databases
and privileged Docker-in-Docker daemons; they exercise real Direct/Edge and
execution behavior without passing the host socket to fixture Core. Use current
builds together, not a historical image matrix. See
[Agent development](../docs/DEVELOPMENT.md#agent-development) for connection setup.

`CITADEL_AGENT_COMPATIBILITY_CACHE_DIR` optionally retains a dedicated compilation
cache. Keep it outside the repository, separate by architecture/builder, and do
not share it across concurrent runs. Default fixture cleanup remains isolated.
Other image/runtime runners are in [scripts/](scripts/), including Stack, script
SDK and real backup acceptance. Do not substitute production daemons for fixtures.

## Updates and release tests

Fast release/setup tooling tests use private files and mocked external boundaries:

```bash
python3 -m unittest discover -s test/release -p 'test_*.py'
SQLX_OFFLINE=true cargo test --locked -p citadel-updates
SQLX_OFFLINE=true cargo test --locked -p citadel-adapters --test instance_update_journal
SQLX_OFFLINE=true cargo test --locked -p citadel-server --test update_release_cli
```

Database update/preflight/bootstrap tests require the fixtures above. Checkpoint
round trips additionally need distinct empty source/target databases and compatible
PostgreSQL tools. Journal/Compose unit tests alone do not qualify actual
replacement, checkpoint restoration or a published release transition.

Build a local Core runtime image before running isolated registration acceptance:

```bash
docker build -t citadel-core:local -f Dockerfile .
python3 test/scripts/test-update-registration.py citadel-core:local --automatic
python3 test/scripts/test-update-registration.py citadel-core:local --enable-updates
```

Add `--rootless` for the real rootless fixture. These runners require Python 3 and
Docker, create their own privileged daemon and private registry, and remove only
their own resources. They check identity/key/storage preservation and registration;
they do not publish images or establish production signature trust.

### Browser and full-lifecycle PostgreSQL upgrades

With the local Core runtime image built:

```bash
python3 test/scripts/test-instance-upgrade-local.py --runtime-image citadel-core:local
python3 test/scripts/test-instance-upgrade-local.py --runtime-image citadel-core:local --database-source 18
# Optional real rootless daemon:
python3 test/scripts/test-instance-upgrade-local.py --runtime-image citadel-core:local --database-source 18 --rootless
# Retain a fixture for manual browser approval:
python3 test/scripts/test-instance-upgrade-local.py --runtime-image citadel-core:local --browser --port 18081
```

The runner builds test-only source/target executables with the pinned container
toolchain, then exercises real review, approval, replacement, interruption recovery
and checkpoint restoration. Its release discovery/trust fixture is private and
never shipped. Use `--source-tree /path/to/separate/checkout --scenario upgrade`
for a specific source revision. Run `--help` for individual interruption scenarios.
Database mode qualifies the supported PostgreSQL 18 minor replacement, not a major
upgrade or PostgreSQL 17 transition.

Browser mode needs installed frontend dependencies, prints its own login and leaves
approval to you. Ctrl+C cleans up its installation; private failure evidence stays
outside the checkout. Browser mode does not automatically perform the checkpoint
round trip. Real publication still requires signed artifacts and verification of
the intended source catalog against the target; see
[release publication](../docs/release-publication.md).

For the corresponding Agent replacement fixture:

```bash
python3 test/scripts/test-agent-upgrade-local.py --runtime-image citadel-core:local --agent-runtime-image citadel-agent:local
```

## Frontend and browser tests

After sequential API generation/verification, run affected tests:

```bash
npm --prefix src/frontend run test:run -- path/to/affected.test.tsx
npm run lint --prefix src/frontend
npm exec --prefix src/frontend -- tsc -b src/frontend/tsconfig.json
```

Use the full `test:run` suite for shared/broad changes; it includes architecture
checks. The [E2E guide](e2e/README.md) owns Playwright setup, smoke, runtime,
accessibility, visual, OIDC and Vault commands. Its Compose environment uses an
isolated daemon and fresh database, and some tests deliberately interrupt services
or restore data. Never direct those suites at a shared installation.

## Documentation checks

```bash
npm run validate --prefix docs
```

This validates product content and links. For repository Markdown changes, also
check relative file/heading links in the edited guides and run `git diff --check`.
Documentation-only edits do not require Rust or frontend suites.
