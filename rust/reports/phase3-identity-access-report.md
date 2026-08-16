# Phase 3 identity and access report

Date: 2026-08-16

## Outcome

Phase 3A and the bounded Phase 3B/C/D/E profile foundation are implemented.
Rust now owns the first production identity slices:
first-run administrator setup, local password login, browser sessions, Actor
authentication/authorization, the permission matrix, offline entitlement
verification, the complete Service Account credential lifecycle, and
authenticated current-profile read/display-name update and preference
read/update operations, browser-session listing and revocation, and local
password changes.

This is not the full Phase 3 exit. Administrator User APIs, Team, Role,
installed-license, MFA, and OIDC
administration remain on the .NET reference implementation. The frontend must
not be cut over to the Rust server until those flows and their differential
tests move together.

## Implemented

- Actor, User, and Service Account principals retain distinct identities.
  Service Accounts do not receive User rows, passwords, refresh sessions, or
  browser cookies.
- Clean setup creates the initial administrator, Actor, Admin Role assignment,
  setup state, and browser session atomically. It does not create a preference
  row until the User explicitly saves preferences.
- A private Rust `User` aggregate now owns User identity, audit metadata, name,
  email, and password-hash state. Authentication retains its smaller
  `UserAuthentication` read projection instead of loading the aggregate on the
  login hot path.
- `GET/PATCH /api/v1/profile` derive the User exclusively from the authenticated
  human principal. One PostgreSQL projection loads non-sensitive User metadata,
  direct Roles, Teams, and current OIDC ownership without per-item queries or
  reading the password hash. The update loads the command-side aggregate,
  writes only the name field in a transaction, and returns refreshed
  authorization and authentication capabilities.
- `GET /api/v1/profile/preferences` returns System date/time and theme defaults
  with a null timezone without writing to PostgreSQL. PATCH uses real merge
  semantics, rejects explicit null and empty patches, validates bundled IANA
  timezone names, and transactionally upserts the single User-owned row. The
  adapter locks the User before reloading and merging current values, so
  concurrent partial patches cannot overwrite unrelated fields.
- `GET /api/v1/profile/sessions` returns only active sessions owned by the
  current User, identifies the current HttpOnly refresh-cookie session, and
  orders it first. User-agent display names match the .NET profile contract.
- Individual session revocation combines ownership and current-session
  protection in one delete. Revoke-other takes the same short User lock as
  session creation, then uses one PostgreSQL statement that locks and
  revalidates the current active session before deleting every other token.
  This prevents both check/delete races and concurrently created sessions from
  escaping ambiguously.
- `POST /api/v1/profile/change-password` is restricted to human, local-password
  profiles. It verifies the current password through the bounded Argon2 worker,
  applies the centralized password policy, and replaces the hash while revoking
  refresh sessions in one transaction. A valid current refresh session is
  retained; without one, every refresh session is removed. Session creation
  locks the same User row and rechecks the password hash used for authentication,
  so a login verified against a stale password cannot commit after the change.
- Passwords use the versioned `cit_pwd_v1$` Argon2id format. Unknown login names
  still execute a real dummy verification to reduce account-enumeration timing.
- Access and refresh JWTs use an explicit issuer, audience, token type, version,
  expiry, and unique ID. Authentication reloads the Actor so disabling it takes
  effect without waiting for token expiry.
- Refresh sessions are persisted, bounded per User, touched on refresh, deleted
  on logout, and survive a Core restart.
- External Service Account credentials use the exact
  `cit_sa_<credential-id>.<secret>` contract. Only the SHA-256 secret digest is
  stored, comparisons are constant-time, and malformed reserved-prefix tokens
  cannot fall back to JWT authentication.
- Service Account create, patch, rename, archive, Role assignment, resource ACL
  assignment, token creation/list/revocation, list visibility, and response
  capabilities use Actor-scoped SQL and the existing human/Admin boundaries.
- Account creation and archive are transactional. Archive locks accounts in a
  stable order, refuses active run-as dependencies, disables Actors, and revokes
  credentials in the same transaction.
- Token creation locks the parent account, rejects disabled/archived accounts
  and duplicate names, and enforces ten active tokens under concurrency.
- Last-used writes are sent to a fixed-capacity queue, coalesced per credential
  for five minutes, persisted monotonically by one supervised worker, and never
  block authentication.
- Team/Role/global/resource permissions are combined at query time. Resource
  decisions are not stored in an unbounded cache, so access changes take effect
  without token reissue.
- Team/Enterprise/legacy Business license envelopes are verified with pinned
  Ed25519 public keys, instance binding, temporal checks, versioned payload
  rules, and a bounded one-entry static-payload cache. Invalid or unavailable
  entitlement state does not disclose details through authentication.
- Internal AES-256-GCM secret envelopes use the versioned `cit_secret_v1`
  format and authenticated encryption. Production configuration requires a
  separate 32-byte secret-encryption key.
- The explicit route catalog generates 31 full and 16 public OpenAPI operations
  plus matching frontend metadata. Browser-only setup/session operations remain
  outside the public API document, as do the UI-only profile operations.
  Parameterized operations now declare their UUID path parameters explicitly.

## Verification

The following passed in the pinned Linux Rust environment:

- `cargo fmt --all --check`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace --all-targets`
- `rust/scripts/Test-Phase3Identity.ps1`

The Phase 3 harness uses two disposable PostgreSQL databases. It proves clean
setup, versioned password persistence, login/refresh, local-HTTP cookie behavior,
profile read/rename, lazy preference defaults, preference validation/upsert and
restart persistence, concurrent partial-patch safety, direct Role projection,
authorization metadata, anonymous permission-matrix access after setup, license
denial, graceful process/container restart, Actor-scoped ACLs, immediate token
revocation, active browser-session ordering, current-session protection,
individual and revoke-other session invalidation, wrong/weak/external-password
rejection, current/all-session password-change invalidation, stale-login
rejection, Role and resource-access
assignment/removal, duplicate-token rejection, archive behavior, and concurrent
token limits.

The accepted user-profile specification describes the display name as
non-unique, while the active .NET login and Rust Phase 3A login both accept a
name as an account identifier and the .NET profile mutation rejects duplicate
names. Phase 3B preserves the implemented unique-name behavior. The full User
cutover must explicitly resolve that specification mismatch before changing
login or uniqueness semantics.

## Remaining Phase 3 work

- Application-information API and the safe profile activity ledger.
- Administrator User, Team, Role, Actor-enabled-state, and installed-license APIs.
- TOTP MFA and the accepted MFA policy behavior.
- OIDC provider administration and browser login/callback behavior.
- Frontend session bootstrap and identity/access screens against the Rust API.
- Differential endpoint/authorization matrices against the .NET reference.

The Rust activity ledger is not implemented yet, so the profile, preference,
password, and session mutations are not ready for frontend cutover even though
their state changes are covered here. Their required safe activity records must
move atomically with the activity subsystem rather than introducing a second
temporary audit format.

Phase 4 Docker read ownership must not depend on an identity behavior that is
still available only from the .NET process.
