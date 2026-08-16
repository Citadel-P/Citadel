# Citadel Rust migration workspace

This workspace contains the isolated migration work described in
`Citadel.Internals/specs/dotnet-to-rust-migration.md`. Rust owns its generated
v1 database baseline and migration runner and now hosts the Phase 3A identity
and Service Account routes, the Phase 3B current-profile foundation, the Phase
3C profile-preferences API, Phase 3D browser-session management, and Phase 3E
local password changes. It does not yet replace the .NET Core.

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
creating a session after the change. Run
`./rust/scripts/Test-Phase3Identity.ps1` for the disposable PostgreSQL,
HTTP-session, profile/preference/password, restart, ACL, revocation, and
concurrent-token checks.
The full Phase 3 exit is intentionally still open for the remaining Profile and
User operations, Team, Role, License, MFA, OIDC, and frontend cutover; see
`reports/phase3-identity-access-report.md`.

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
