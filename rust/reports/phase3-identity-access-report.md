# Phase 3 identity and access report

Date: 2026-08-16

## Outcome

Phase 3A is implemented. Rust now owns the first production identity slice:
first-run administrator setup, local password login, browser sessions, Actor
authentication/authorization, the permission matrix, offline entitlement
verification, and the complete Service Account credential lifecycle.

This is not the full Phase 3 exit. User/Profile, Team, Role, installed-license,
MFA, and OIDC administration remain on the .NET reference implementation. The
frontend must not be cut over to the Rust server until those flows and their
differential tests move together.

## Implemented

- Actor, User, and Service Account principals retain distinct identities.
  Service Accounts do not receive User rows, passwords, refresh sessions, or
  browser cookies.
- Clean setup creates the initial administrator, Actor, preferences, Admin Role
  assignment, setup state, and browser session atomically.
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
- The explicit route catalog generates 23 full and 16 public OpenAPI operations
  plus matching frontend metadata. Browser-only setup/session operations remain
  outside the public API document.

## Verification

The following passed in the pinned Linux Rust environment:

- `cargo fmt --all --check`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace`
- `rust/scripts/Test-Phase3Identity.ps1`

The Phase 3 harness uses two disposable PostgreSQL databases. It proves clean
setup, versioned password persistence, login/refresh, local-HTTP cookie behavior,
anonymous permission-matrix access after setup, license denial, graceful
process/container restart, persisted login after restart, Actor-scoped ACLs,
immediate token revocation, Role and resource-access assignment/removal,
duplicate-token rejection, archive behavior, and concurrent token limits.

## Remaining Phase 3 work

- Profile details, preferences, password changes, and session management.
- User, Team, Role, Actor-enabled-state, and installed-license APIs.
- TOTP MFA and the accepted MFA policy behavior.
- OIDC provider administration and browser login/callback behavior.
- Frontend session bootstrap and identity/access screens against the Rust API.
- Differential endpoint/authorization matrices against the .NET reference.

Phase 4 Docker read ownership must not depend on an identity behavior that is
still available only from the .NET process.
