# Citadel Rust migration workspace

## API parity gate

Phase 7.5 is still in progress; this workspace is not yet a full replacement
for the .NET Core. Run `cargo run -p xtask -- parity` from `rust/` to compare all
.NET operation IDs, HTTP methods, paths, and public/internal exposure with the
Utoipa handler contracts. The command deliberately fails while omissions remain;
there is no allowlist that hides missing operations.

This is separate from `cargo run -p xtask -- openapi --check`, which checks
generated-artifact freshness and compatibility of the implemented subset.
Neither command replaces HTTP/persistence tests or live transport acceptance.
See [Phase 7.5 implementation evidence](reports/phase7-5-api-parity.md).

## OpenAPI generation

Run `bash rust/scripts/build.sh` from the repository root to build the API and
export `schema/v1.json` and `schema/public-v1.json`. The VS Code build/run and
debug workflows do this automatically. Plain `cargo build` only compiles.

The API uses Utoipa and utoipa-axum to share route registration and documentation.
Schemas derive from Rust DTOs; the former central HTTP catalog and handwritten
schema generator have been removed. See [OpenAPI ownership and migration boundary](crates/server/src/openapi/README.md).

## Unattended first run and recovery

For unattended Rust Core setup, supply all three settings:
`Bootstrap__AdminName`, `Bootstrap__AdminEmail`, and
`Bootstrap__AdminPasswordFile`. Mount the password file read-only and use its
absolute path inside Core. It must be a regular UTF-8 file of at most 1 KiB;
one trailing newline is allowed. Do not put the password itself in environment
variables. With no bootstrap settings, use the normal browser setup.

Partial settings fail startup without completing setup. Successful setup creates
one administrator and the two disabled example automations in one transaction.
It does not issue a browser session. After setup, these settings are ignored and
the password mount can be removed before restarting.

Offline `restore-system` requires the original `Jwt__Key` and
`Secrets__EncryptionKey` from the backed-up installation. Preserve these outside
the database backup. A missing/malformed encryption key is rejected before target
database work; a correctly sized **wrong** key is not detected by this preflight
and cannot decrypt restored secrets. Restore into a disposable environment and
verify the retained keys before replacing an installation.

Process, HTTP/PostgreSQL and recovery test commands and remaining parity gaps
are tracked in [the test-port report](reports/phase7-5-test-port-progress.md).

## Migration background

This workspace contains the isolated migration work described in
`Citadel.Internals/specs/dotnet-to-rust-migration.md`. Rust owns its generated
v1 database baseline and migration runner and now hosts the Phase 3A identity
and Service Account routes, the Phase 3B current-profile foundation, the Phase
3C profile-preferences API, Phase 3D browser-session management, and Phase 3E
local password changes. Phase 3F exposes authenticated application build
information. Phase 3G adds the compatible Activity read API and atomic safe
activity evidence for profile mutations. Phase 3H adds the administrator User
read boundary. Phase 3I adds atomic administrator User creation, patch, rename,
Role/resource-access assignment, and deletion. Phase 3J adds the administrator
Team aggregate and complete Team read/mutation boundary. Phase 3K adds Role
administration and the compatible anonymous permission matrix. Phase 3L adds
installed-license administration, safe entitlement projections, stable
instance-request metadata, and atomic License Activity evidence. It does not
yet replace the .NET Core. Phase 3M ports local TOTP MFA, and Phase 3N ports
OIDC provider administration plus browser authorization-code/PKCE login. Phase
3O proves the unchanged React login/session flow against Rust with real
Keycloak and production SPA hosting. Phase 3P proves the administrator Access
and License screens against the same Rust-hosted production bundle. Phase 3Q
adds restart-safe time-bound License transitions and the shared frontend's
authenticated, metadata-free License notification path.

Phase 3T replaces the former horizontal Domain/Application catch-all with the
first two bounded-context crates. `citadel-identity` owns Identity models, use
cases, and ports; `citadel-platforms` owns authorized Platform projections and
runtime capability contracts. The small `citadel-domain` crate is now a shared
kernel, concrete infrastructure remains in `citadel-adapters`, and consumers
import the owning context directly. New crates are added only for substantial
behavior boundaries, not per resource or handler.

The prototype proves a small release server, Citadel's existing authorized
Platform read, a generated Docker Engine read/stream subset over a Unix socket,
the Phase 0B signed gRPC handshake/read/stats stream against the active .NET
Agent, and the Phase 0C authorized versioned WebSocket subscription with
bounded overflow/resync behavior. See the corresponding decision and effective
configuration reports under `reports/`, and `scripts/README.md` for repeatable
build, interoperability, realtime, and measurement commands.

Phase 1 also contains a deterministic inventory, accepted-contract hashes,
configuration defaults, specification traceability, a breaking-change ledger,
and the accepted database-baseline plan. Pinned DPM 0.3.2 passed the complete
product-schema proof, so Phase 1 is complete and Phase 2 database work is no
longer blocked. See `phase1/README.md` and
`reports/phase1-decision-report.md`.

Phase 2 adds the production foundation: validated Direct TLS, trusted reverse
proxy, and explicitly insecure transport modes; request/security middleware;
generated full/public OpenAPI and frontend metadata; embedded checksummed
database migrations; SQLx offline query checking; a generated Swarm inspect
operation; bounded queues; setup/readiness behavior; SPA fallback; and cgroup
diagnostics. Run `./rust/scripts/Test-Phase2Foundation.ps1` for the disposable
PostgreSQL, startup, restart, generation, and Actor-scoped query checks. See
`reports/phase2-foundation-report.md` for scope and evidence.

Phase 3A adds first-run administrator setup, local login and persisted browser
sessions, Actor/RBAC/resource-ACL evaluation, offline entitlement verification,
and the Service Account credential lifecycle. Phase 3B introduces the real
User aggregate and authenticated current-profile read/rename operations. Phase
3C adds lazy profile-preference defaults plus validated, transactional
preference persistence. Phase 3D lists active browser sessions and provides
ownership-scoped individual and atomic revoke-other operations. Phase 3E
verifies and replaces local passwords transactionally, revokes the appropriate
refresh sessions, and prevents a login verified with a stale password from
creating a session after the change. Phase 3F reads version information embedded
at compile time from release inputs or the repository `version.json`; it never
depends on frontend constants or runtime Git access. Phase 3G ports the complete
Activity discriminator set, owns typed safe profile event payloads, writes state
and evidence in one transaction, and exposes the existing authorized
`listActivities`/`getActivity` contract so the current frontend resource can use
the Rust server unchanged. Phase 3H exposes the existing administrator-only
`listUsers`, `searchUsers`, and `getUser` contracts from bounded PostgreSQL
projections, including Roles, Teams, enabled state, resource overrides, paging,
and capabilities. Phase 3I ports the matching User mutation routes with
transactional assignment checks, last-administrator serialization,
refresh-session invalidation, and typed safe Activity evidence. Run
`./rust/scripts/Test-Phase3Identity.ps1` for the disposable PostgreSQL,
HTTP-session, application-info, profile/preference/password, restart, ACL,
revocation, activity, and concurrent-token checks.
Phase 3J ports Team list/search/detail, create, merge patch, rename,
member/Role/resource-access assignment, and deletion with the same atomic
Activity and administrator-survival guarantees.
Phase 3K ports Role list/detail, custom Role creation, permission replacement,
rename, and deletion. System Roles remain immutable; custom-access licensing
blocks authority expansion but never blocks reduction or cleanup. Role state
and typed lifecycle Activity commit in one transaction, and the permission
matrix retains its existing anonymous keyed-object contract.
Phase 3L ports the five existing License routes. Every authenticated Actor can
read the minimal entitlements view; administrator metadata, request, install,
replacement, and removal retain their License permission boundaries. The
persisted instance ID is immutable, raw licenses never leave storage, and
replacement checks, singleton writes, and typed safe Activity evidence share
one transaction. Static Ed25519 verification may be cached by fingerprint,
while temporal status is recalculated on every effective-state read.
Phase 3M ports the complete local TOTP MFA boundary without changing the
existing browser contract. Setup and login branch into completed, verification,
or required-enrollment outcomes; short-lived challenge/setup cookies are
HttpOnly; TOTP steps and recovery codes are replay-safe; and credential
consumption, session issuance, recovery-code rotation, session revocation, and
typed safe Activity evidence commit atomically. The nine existing MFA routes
are covered through a real PostgreSQL-backed Axum test, including concurrent
challenge completion.
Phase 3N preserves all twelve existing OIDC operation IDs. Provider secrets are
encrypted and never projected, login state is stored only by hash and consumed
atomically, authorization uses code flow with PKCE and nonce, and signed ID
tokens are validated against bounded discovery/JWKS responses before Citadel
links or provisions a User and issues its normal refresh session. A fake signed
issuer test exercises discovery, token exchange, JWKS, issuer, audience,
signature, and nonce validation; a real PostgreSQL-backed Axum test covers
provider lifecycle, trusted redirects, replay/concurrency, disabled Users,
required claims, verified-email linking, and default-Role provisioning.
Phase 3O runs the existing Keycloak Playwright suite against the production
React bundle served by Rust. It covers first-run setup, requested-route
preservation, OIDC callback extension parameters, scoped refresh cookies,
session restoration after reload, idempotent cookie-based logout, and required-
claim rejection. Rust SPA fallback returns 200 for browser deep links while
unknown API paths remain 404 Problem Details responses.
Phase 3P adds production-browser coverage for User and Team creation/list/detail,
system Roles, Community-gated Service Accounts, License rendering, and OIDC
provider list/detail. Community license denials retain the exact frontend-facing
Problem Details contract.
Phase 3Q persists `notBefore`, expiry, grace-period, and invalid-state
transitions under the singleton License lock. Only the first observer of a
changed status writes typed System Activity; overlapping checks and restarts do
not duplicate evidence. The worker sleeps until the next known boundary plus a
one-second margin, never longer than one hour, and wakes immediately when a
License is installed or removed. Rust advertises `WebSocketV1` while .NET
advertises `SignalR`, allowing the same SPA to select the supported transport.
The versioned Rust event has an empty payload and only invalidates authoritative
License queries.

Phase 3R closes the identity/access exit with a shared authorization fixture.
It verifies all 22 resource capability entries plus direct/Team Roles,
resource overrides, disabled Actors/Teams, permission aggregation, and the
human-versus-Service-Account administrator boundary through both the .NET and
Rust PostgreSQL implementations. See
`reports/phase3-identity-access-report.md`.

Phase 3U keeps identity HTTP routes thin without recreating .NET attributes or
a mediator pipeline. User, Team, Role, and Service Account handlers use one
typed `identity_result` adapter and ordinary `?` propagation; policy order is
still visible in each handler and the success path adds no allocation or
dynamic dispatch.

Phase 4 implements the five Platform/Docker read slices for Local Docker and
the active .NET Agent protocol. Authorized Platform, Container, Image, Network,
Volume, and Swarm inventory routes read durable PostgreSQL projections. Docker
and Agent events trigger independent coalesced refreshes, a 30-minute sweep
repairs missed events, container statistics are streamed and retained for seven
days, and the versioned WebSocket uses normal Citadel bearer authentication and
Platform ACLs. Generated Docker API 1.49 operations and the handwritten
transport remain the runtime boundary; Bollard is not used. See
`reports/phase4-platform-docker-reads-report.md` for the .NET-test mapping and
the explicit Edge/cgroup exit gates.

The Platform registration prerequisite is also available through the existing
React form and the compatible Rust `createPlatform` endpoint. Local Docker and
the configured signed Agent discover their initial inventory before one
transaction commits the Platform, projections, tags, and typed Activity;
duplicate daemon and Swarm cluster identities are serialized and rejected.
See `reports/platform-registration-prerequisite-report.md`. Edge enrollment
and arbitrary per-Platform Agent dialing remain explicit migration boundaries.

Phase 5 adds the `citadel-resources` bounded context and ports the simple
mutation/metadata boundary: Tags, Platform metadata, Registries, Git repository
definitions, resource bindings, internal/external Secret definitions,
Vault-compatible Secret-provider definitions, and selected Docker Network and
Volume mutations. State, tag links, and safe Activity evidence are
transactional; authorization is resource-scoped; Local and Agent mutations are
transport-equivalent and do not retry ambiguous failures; and the frontend
receives metadata-free realtime invalidations. Platform background behavior is
now organized under `citadel-platforms::jobs`, while
`citadel-server::workers` contains only runtime scheduling and supervision. See
`reports/phase5-simple-mutations-metadata-report.md` and run
`./rust/scripts/Test-Phase5Metadata.ps1` for the disposable acceptance gate.

Phase 6A adds the `citadel-deployments` bounded context and ports the existing
Deployment list/detail/config/create/update/metadata/rename/duplicate/delete
contracts. PostgreSQL remains compatible with the .NET discriminator and
PascalCase spec representation, while browser JSON remains camelCase. ACL
filtering and capabilities are computed in the collection query; state, Tags,
copied bindings, and typed Activity evidence commit transactionally. Deletion
claims the complete batch before Docker mutation, survives request loss, has a
bounded timeout, removes every linked container through Local or signed Agent
transport, and restores the claim if runtime or completion fails. See
`reports/phase6a-deployment-crud-report.md` and run
`./rust/scripts/Test-Phase6ADeployments.ps1` for the disposable gate.

The Phase 6A gate maps every Deployment CRUD/config test in the .NET
integration and unit suites that applies to this slice. It includes full and
partial merge patching, direct/Team ACLs, exact filters, duplicate failures,
Local-image projections, concurrent mutation/delete exclusion, and caller-loss
recovery. The .NET acceptance project has no isolated Deployment CRUD scenario;
runtime Apply acceptance is covered by the next gate.

Phase 6B ports Standalone Deployment Apply through Local and signed Agent
transports. Apply is permission-checked and transactionally claimed before
runtime work; at most four Apply operations execute concurrently. The bounded
JSON progress stream is independent of the browser connection, referenced
variables and internal Secrets are resolved with Deployment-over-Global
precedence, persisted evidence masks Secret values, and runtime failures are
redacted. Docker health checks decide readiness when present. Success or
failure atomically records the Container link, Deployment state, and typed
Activity. Interrupted or timed-out outcomes remain claimed until the bounded
reconciliation worker observes the ownership label and finalizes them. Run
`./rust/scripts/Test-Phase6BDeploymentApply.ps1`; see
`reports/phase6b-deployment-apply-report.md`.

Build-backed images and external Secret-provider execution remain explicit
Phase 7 boundaries. Edge Agent Apply remains unavailable until its inbound
mutation transport is migrated; it is not silently routed through a manager.

Phase 6C ports the core managed Docker Swarm Service lifecycle. Rust now owns
typed CRUD, Apply, scale, force update, delete, bounded progress, transactional
Activities, Local/signed-Agent mutation dispatch, current task projections,
variable/internal-Secret injection, and bounded rollout reconciliation. Docker
acceptance remains distinct from rollout success, and current health/counts
come from the latest Swarm inventory rather than a stale persisted label. Run
`./rust/scripts/Test-Phase6CManagedSwarmServices.ps1`; see
`reports/phase6c-managed-swarm-services-report.md`.

Adoption, duplicate drafts, webhooks, update checks, logs, terminal,
statistics, build-backed images, external Secret providers, and Edge Agent
mutation remain later migration boundaries.

Phase 6D–6F ports Web Editor Stacks across Docker Standalone and Docker Swarm:
CRUD, duplicate drafts, bounded Apply progress, transactional releases,
rollback, deletion, Standalone state actions, fail-closed Swarm Compose
preflight, non-mutating Compose/Swarm import, drift inspection and safe repair,
and manual-image update evaluation. Stack ownership labels match the existing
container and Swarm projection contracts. Import claims the complete runtime
namespace atomically, while ambiguous Apply outcomes remain durable for bounded
reconciliation instead of being retried.

Standalone state actions also claim their full batch before Docker I/O,
persist the stable release state and typed Activity on success, restore
definite failures, and leave ambiguous timeouts for stable-only reconciliation.

Run `./rust/scripts/Test-Phase6Stacks.ps1`; see
`reports/phase6d-f-stacks-report.md`. Add `-RunSwarmLifecycle` only when the
test Docker daemon is already an active Swarm manager. The gate never changes
Swarm membership. Git materialization/webhooks, registry digest I/O,
build-backed images, external Secret-provider execution, retained Swarm
Secret/Config reconstruction and Edge Agent mutation remain Phase 7/10
boundaries; the Phase 6 update evaluator does not pretend those transports are
available.

Phase 7 now includes the bounded external-process foundation; encrypted Git
accounts and durable repository synchronization; repository browsing,
comparison, Compose discovery, polling, and authenticated webhooks; bounded
Deno automation runs and scheduling; local Docker build runs; Alert resource
lifecycle, Build/Automation event evaluation, and durable retrying Shoutrrr
channel delivery; and durable local Docker-volume
backup/restore runs with scheduling, cancellation, leases, recovery, retention,
execution-time run-as authorization, immutable workload-to-volume plans, and
per-volume outcomes. Citadel-system policies create a PostgreSQL recovery
bundle, validate it before Restic upload, and can be restored only through the
explicitly confirmed offline `restore-system` command. Platform CPU/RAM/disk
threshold observations and Platform/Build/Automation failure events use the
same durable Alert pipeline. The release image pins and includes Git, Docker
CLI, Deno, Shoutrrr, Restic, and the PostgreSQL client tools used by these paths.

Regular-Agent builds and Git-backed Stack Apply now use signed, bounded source
transport and persist their immutable Git provenance. Build-backed Deployment
Apply resolves and records the exact successful Build Run it deploys.
Stack Build-image bindings use the same immutable provenance rule through a
generated Compose override without rewriting user source.
Eligible Stacks are also checked for drift on a bounded five-minute schedule;
one failing Stack cannot prevent the remaining candidates from being checked.
Standalone Stack Apply executes configured pre/post commands on Local and
regular-Agent connectors and resolves Registry authentication only for the
active Apply attempt. Swarm rejects those Compose-only options instead of
silently ignoring them. An opt-in two-database recovery test verifies that a
Citadel-system bundle restores persisted state into a clean PostgreSQL target.
S3-compatible Volume backup/restore now runs through the configured regular
Agent or exact-node Edge session. Edge workload execution, enrolled Build Pool
execution and incremental bounded/redacted Build logs are wired. Node snapshots
persist isolated Container/Image/Volume/Network projections and publish the
existing UI realtime contracts. A real Local Docker/Restic acceptance test
verifies backup results and restored files at the volume root.

Build Pool Test/edit/rename/metadata now preserve the .NET contracts and write
transactional activities. Pool HTTP/realtime rows include capabilities. Signed
inbound execution uses the selected pool endpoint; node-ID rebind is fenced by
proof, identity, membership and grace-period checks. Real Deno/Shoutrrr tests
verify process outcomes, cancellation, token redaction and delivery retry. Run
`./rust/scripts/Test-Phase7AutomationExternal.ps1` with the development workspace
running; it uses disposable fixtures and extracts tools from `-CoreImage`.
Automation lifecycle Activities commit with their state changes, mask webhook
secrets, and reject stale edits/completions. Metadata-only edits do not create
configuration Activities. HTTP progress streaming and tag/filter parity are
covered by the follow-up tests below.

Automation manual/Test progress and Build lifecycle/tag filtering/Pool health
are now covered by HTTP and persistence tests, including real Deno execution.
The shared webhook listener also dispatches Automation, Build and Backup Policy
runs. It verifies the saved authentication scheme and fences configuration
changes before committing a queued run. Build execution synchronizes and pins
its Git source; Automation workers share configurable concurrency and timeout
limits, and Backup automated execution rechecks its license entitlement.
The node-agent coverage endpoint reports manager/satellite coverage, stale
runtime data, unsupported nodes and system-Service drift without exposing
enrollment credentials. Removal now uses the existing streamed-progress endpoint:
it requires Platform Execute plus ManageNodeAgents, validates the pinned manager,
removes only owned infrastructure and revokes node credentials. It preserves the
manager connection and node-agent state volumes. Failed removals can be retried;
cleanup warnings identify resources left for review. Install, Repair and Upgrade
now share a bounded system-Service workflow through the existing progress UI.
Configure `EdgeAgent__PublicGrpcUrl` to a node-reachable HTTP(S) origin (not localhost)
and `CITADEL_EDGE_AGENT_IMAGE` to the Agent image. The workflow resolves its digest,
checks Linux architecture coverage and uses a short-lived, hash-only bootstrap
credential mounted as a Docker Secret. An optional
`CITADEL_NODE_AGENT_CA_CERTIFICATE_PATH` supplies the Core CA bundle (up to 1 MiB).
Completion requires the current Service Tasks and their live satellite connections;
paused rollouts, cancellation and timeout persist failure and revoke bootstrap access.
A manager-only cluster needs no satellite Service. The real three-node
Install/Repair/Upgrade workflow and exact-worker RustFS backup/restore now pass
against the signed Agent candidate recorded in
[`reports/phase7-edge-execution-report.md`](reports/phase7-edge-execution-report.md).
That report is the current Phase 7 implementation, test-port and exit-gate record;
the earlier paragraphs describe chronological migration checkpoints.

Node-local endpoints, logs/terminal, Build consumer queues, update producers and
Stack/Service webhooks use the existing HTTP/realtime contracts. Unsupported
connector paths fail closed and never fall back to Core's Docker daemon.
Repeat the database/contract gate with `./rust/scripts/Test-Phase7AExternalExecution.ps1`;
use `Test-Phase7LocalBackup.ps1 -UseRustFs -MultiNode` for actual worker routing,
`Test-Phase7Integrations.ps1` for real Forgejo/Vault execution, and
`Test-Phase7SystemRecovery.ps1` for PostgreSQL bundle restore. These fixtures are
disposable. Full release-image installation, browser acceptance and memory soak
remain Phase 8.

## Interactive development

On Windows, use VS Code in WSL with the repository on the Linux filesystem,
running the Rust API and frontend directly and PostgreSQL in Docker Desktop.
The committed Dev Container remains available as a fallback. The complete setup,
run, debugging, test, shutdown, and troubleshooting commands are documented in
[DEVELOPMENT.md](DEVELOPMENT.md).

Authentication cookies are scoped to `/api/v1`, which keeps them away from SPA
assets while allowing the existing profile-session endpoints to identify and
protect the current refresh session. Logout also expires the former
authentication-only and root cookie paths during migration.

## Slice delivery gate

Every new Rust slice starts by inventorying the matching .NET unit,
integration, acceptance, authorization, frontend, and public-contract tests.
The slice report must map relevant .NET scenarios to Rust tests, with an
explicit reason for anything that is not applicable. Rust tests are added
before production behavior, then the implementation is completed until the
focused tests and the full Rust regression suite pass.

Tests preserve observable behavior rather than private .NET structure. HTTP
routes are tested through the Axum router, persistence through real PostgreSQL,
and Docker, Agent, Edge Agent, restart, and cross-process behavior through the
appropriate integration or acceptance harness. A slice is not considered
implemented merely because its production code compiles.

## Repository direction

The completed Core-first phases stay isolated under `rust/` while the existing
.NET Agent repository remains compatibility evidence. Before Rust Agent work
begins, its relevant Git history will be imported into this repository. The
target is one Cargo workspace with separate Core, Agent, and Edge Agent
binaries, not one combined process and not one source repository per binary.

Regular Agent and Edge Agent will share a bounded Agent runtime while retaining
separate transport composition, images, configuration, security profiles,
component tags, release jobs, installation, and rollback. The Contracts
submodule remains authoritative only for active .NET compatibility; the final
Rust workspace will own the language-neutral contract sources and verify every
affected binary on contract changes.
