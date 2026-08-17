# Citadel Rust migration workspace

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
