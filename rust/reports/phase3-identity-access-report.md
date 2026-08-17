# Phase 3 identity and access report

Date: 2026-08-17

## Outcome

Phase 3A through Phase 3T are implemented, and the Phase 3 identity/access exit
criteria are satisfied.
Rust now owns the first production identity slices:
first-run administrator setup, local password login, browser sessions, Actor
authentication/authorization, the permission matrix, offline entitlement
verification, the complete Service Account credential lifecycle, and
authenticated current-profile read/display-name update and preference
read/update operations, browser-session listing and revocation, and local
password changes. Authenticated clients can also read the Core build identity
used by the shared layout, authorized Activity history, and administrator User
reads and mutations, administrator Team reads and mutations, and administrator
Role reads and mutations, installed-license administration, and OIDC provider
administration/browser login through the existing frontend contracts.

The unchanged frontend login, session, profile, OIDC, administrator Access, and
License-notification browser paths run against Rust. A language-neutral
authorization contract now executes through both the .NET/Dapper and Rust/SQLx
PostgreSQL paths, so Phase 4 no longer depends on identity behavior available
only from the .NET process.

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
- `GET /api/v1/application/info` returns `Citadel`, a display-safe version, and
  the complete informational version embedded at compile time. Release builds
  accept the existing version pipeline values; local builds deterministically
  fall back to the root `version.json`. The endpoint requires an authenticated
  Actor, remains setup-gated, performs no database read, and sets `no-store`.
- Rust ports every persisted `ActivityStatus`, `ActivityResourceType`, and
  `ActivityEventType` discriminator from .NET. The command-side Activity
  aggregate currently owns the five profile payloads required by this slice:
  profile rename, preference update, password change, individual session
  revoke, and revoke-other.
- Activity changed fields are constructed from a four-value allow-list rather
  than arbitrary strings. Password-change evidence has only its discriminator;
  passwords, password hashes, refresh tokens, cookies, and session secrets
  cannot be supplied to its payload.
- Profile rename, changed preferences, password replacement/session cleanup,
  and session revocation insert their required Activity row before the same
  SQLx transaction commits. No-op rename/preference/revoke operations remain
  quiet. A failed activity insert rolls back the state mutation.
- `GET /api/v1/activities` and `GET /api/v1/activities/{id}` preserve the .NET
  response shape, typed string discriminators, actor/platform projections,
  bounded paging, filter names, and `no-store` behavior. Administrators see the
  full ledger; other Actors see only resource types/IDs granted through their
  direct or Team Role permissions and resource overrides. Administrator-only
  identity/license/OIDC activity remains hidden from non-administrators.
- The read adapter accepts existing .NET rows, bounds Activity JSON at 256 KiB,
  verifies that `$type` matches `EventType`, and maps persisted Pascal-case
  payload fields to the camel-case HTTP contract. The existing Citadel frontend
  already calls these two operation IDs, so no duplicate Activity UI was added.
- Administrator-only `GET /api/v1/users`, `/api/v1/users/search`, and
  `/api/v1/users/{id}` preserve the current list, assignment-search, and detail
  response shapes. Service Accounts and non-administrator Users cannot use the
  routes even if they hold a User permission.
- The User read adapter uses bounded paging/search, deterministic ordering, and
  aggregate PostgreSQL projections for Roles, Teams, enabled state, and detail
  resource overrides. List rows deliberately omit detail-only overrides, so it
  does not create an N+1 query or load unnecessary ACL data on the table path.
- Administrator-only User create, merge patch, rename, Role assignment/removal,
  resource-access assignment/removal, and batch delete preserve the current
  HTTP request and response contracts. Mutations require a human administrator
  and the matching User permission; a Service Account cannot invoke them.
- User, Actor, Team/Role/resource-access assignments, refresh-session cleanup,
  last-administrator checks, and required Activity evidence commit in one SQLx
  transaction. A transaction-scoped PostgreSQL advisory lock serializes the
  User and Team changes that can affect administrator survival.
- Create and patch validate name/email conflicts and all referenced assignment
  targets before commit. Custom Roles, Teams with custom access, and expanded
  resource overrides remain independently license-gated. A failed validation,
  missing assignment, duplicate assignment, Activity write, or final-admin
  check leaves no partial User, Actor, assignment, or ghost Activity row.
- Password patches apply the same identity-aware policy as .NET, hash through
  the bounded Argon2 service, and delete the User's refresh sessions in the
  same transaction. Disabling a User also invalidates those sessions.
- User lifecycle evidence uses typed `UserCreated`, `UserUpdated`,
  `UserRenamed`, and `UserDeleted` payloads compatible with the .NET
  discriminators and snapshots. Plaintext passwords and password hashes cannot
  enter those payload types.
- Administrator-only Team list, search, detail, create, merge patch, rename,
  member/Role/resource-access assignment and removal, and batch delete preserve
  the current operation IDs and response projections. Members include both
  Users and Service Accounts; bulk `userIds` replacement deliberately changes
  only User members and retains Service Account members, matching .NET.
- Team, Actor, User-member/Role/resource-access assignments, enabled state,
  final-administrator validation, and typed `TeamCreated`, `TeamUpdated`,
  `TeamRenamed`, and `TeamDeleted` evidence commit in one SQLx transaction.
  User and Team mutations take the same transaction-scoped advisory lock, so
  concurrent changes cannot each remove the final enabled administrator.
- Team custom Roles, resource overrides, additions of members to custom-access
  Teams, and Service Account membership enforce Custom Access Control only when
  access is expanded. Removals remain available so an expired license cannot
  trap administrators in an unsafe configuration.
- Administrator-only Role list/detail, custom Role creation, permission
  replacement, rename, and deletion preserve the active operation IDs and DTO
  shapes. The permission matrix remains anonymous and uses the existing object
  keyed by resource type rather than a Rust-specific array shape.
- System Roles are immutable. Creating a custom Role and expanding its
  permissions require Custom Access Control, while reduction, rename, and
  deletion remain available after license expiry. Name create/rename checks use
  the shared transaction-scoped identity advisory lock because the compatible
  schema has no unique Role-name index.
- Role rows, their bounded permission set, and typed `RoleCreated`,
  `RoleUpdated`, `RoleRenamed`, and `RoleDeleted` evidence commit in one SQLx
  transaction. Failed validation, conflict, licensing, or system-Role checks
  leave no partial permissions or ghost Activity.
- Passwords use the versioned `cit_pwd_v1$` Argon2id format. Unknown login names
  still execute a real dummy verification to reduce account-enumeration timing.
- Access and refresh JWTs use an explicit issuer, audience, token type, version,
  expiry, and unique ID. Authentication reloads the Actor so disabling it takes
  effect without waiting for token expiry.
- Refresh sessions are persisted, bounded per User, touched on refresh, deleted
  on logout, and survive a Core restart.
- Local setup and password login share one MFA-aware completion decision:
  completed authentication issues a refresh session, enabled MFA creates only
  a short-lived verification challenge, and required policy creates only a
  short-lived enrollment session. No normal access or refresh token exists
  before verification or mandatory enrollment succeeds.
- The nine existing authentication/profile/administrator MFA operation IDs,
  DTO shapes, and human/Admin boundaries are preserved. Challenge and setup
  identifiers live in scoped HttpOnly SameSite cookies and are never returned
  in JSON. Logout removes refresh, challenge, and setup cookies.
- TOTP uses 20 random bytes, Base32 provisioning, SHA-1, six digits, 30-second
  steps, and a bounded adjacent-step window. Accepted time steps advance under
  the same transaction that consumes the challenge, preventing replay and
  concurrent double-session issuance.
- Recovery codes use an ambiguity-reduced alphabet, are displayed once,
  normalized before use, and stored only as purpose-derived HMAC-SHA-256
  values. Recovery consumption, challenge consumption, session issuance, and
  safe `UserMfaRecoveryCodeUsed` evidence are atomic.
- Enrollment, disable, recovery regeneration, administrator reset, failed
  verification evidence, MFA state cleanup, and refresh-session revocation use
  typed commands and PostgreSQL transactions. Administrator reset also works
  for a disabled User and removes every pending MFA credential/session.
- `Mfa__Policy` supports `Optional`, `RequiredForAdministrators`, and
  `RequiredForAllUsers`, with validated bounded challenge/setup lifetimes,
  failed-attempt limits, and recovery-code counts in effective configuration.
- The three anonymous browser OIDC routes and nine administrator provider
  routes preserve the active method, path, operation ID, DTO, redirect, and
  public-document boundaries. Administrator responses expose only
  `hasClientSecret`; plaintext and ciphertext never enter response or Activity
  payload types.
- Provider create/update/delete and typed safe lifecycle evidence commit in one
  SQLx transaction. Client secrets use the shared versioned AES-256-GCM
  protector, and an omitted, null, or empty update preserves the current
  encrypted value.
- Authorization-code login uses PKCE S256, a nonce, a 256-bit opaque state, and
  a trusted return URL. PostgreSQL stores only the SHA-256 state digest, removes
  expired rows on insertion, and consumes the matching row with one
  `DELETE ... RETURNING`, so concurrent callbacks cannot both create sessions.
- Discovery, token, and JWKS responses are limited to 1 MiB, redirects are not
  followed, and caches are time- and size-bounded. ID tokens accept only
  asymmetric signing algorithms and compatible signature-use JWKs, then verify
  signature, issuer, audience/authorized party, expiry, not-before, subject,
  and nonce before any identity mutation.
- Existing identities resolve by `(ProviderId, sub)`. Verified-email linking is
  opt-in and requires exactly one matching local User; provisioning is opt-in,
  serializes with other identity mutations, creates the Actor/User/external
  link/default Role and safe User Activity transactionally, and never assigns
  administrator authority from claims. Disabled linked Users are rejected.
- The production React bundle is served directly by Rust. Browser deep links
  return `index.html` with 200, while unmatched `/api/*` paths return a bounded
  404 Problem Details response rather than HTML.
- Refresh, MFA, and setup cookies use the existing names and secure/insecure
  SameSite policy. Rust scopes them to `/api/v1`: this excludes SPA assets but
  lets the existing profile-session endpoints identify the current refresh
  session. The narrower .NET authentication-only path cannot do that in a real
  browser. Logout expires the current API path plus the legacy authentication
  and root paths. It remains refresh-cookie based and idempotent, so it does not
  race the frontend's immediate removal of its in-memory bearer token.
- OIDC callbacks accept provider extension parameters such as Keycloak's
  `session_state` and `iss` while continuing to validate the explicit
  `code`/`state` or error fields. The setup route retains the originally
  requested browser destination until the authenticated route owns it.
- OpenAPI verification checks every Rust `/api/v1` operation against the
  generated frontend schema for operation ID, method/path, request/response
  schema reference, and non-cookie parameters.
- The production Access UI now exercises User and Team creation, list and
  detail views, system Roles, Community-gated Service Accounts, the License
  page, and OIDC provider list/detail routes against Rust. The existing
  Community licensing smoke test remains unchanged. Rust license-denial
  Problem Details preserve its canonical type URI, display capability,
  effective edition, status, and user-facing detail instead of returning the
  smaller internal identity error shape.
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
- The versioned language-neutral authorization fixture covers all 22 resource
  permission-matrix entries and 13 Actor scenarios: no access, disabled Actors,
  direct and Team Roles, disabled Teams, direct and Team resource overrides,
  level/specific-permission aggregation, resource isolation, and the distinct
  human-administrator and Service Account boundaries. The .NET integration
  suite and Rust SQLx integration suite deserialize the same file and execute
  it against their real PostgreSQL permission paths.
- Differential execution found one concrete mismatch: Rust resource-level
  authorization admitted a preconstructed disabled Actor principal because its
  resource query did not independently constrain the principal Actor. The
  Actor scope now requires an enabled member for both direct and Team-derived
  grants, matching .NET and protecting internal callers in addition to bearer
  authentication.
- Team/Enterprise/legacy Business license envelopes are verified with pinned
  Ed25519 public keys, instance binding, temporal checks, versioned payload
  rules, and a bounded one-entry static-payload cache. Invalid or unavailable
  entitlement state does not disclose details through authentication.
- Internal AES-256-GCM secret envelopes use the versioned `cit_secret_v1`
  format and authenticated encryption. Production configuration requires a
  separate 32-byte secret-encryption key.
- The explicit route catalog generates 90 full and 38 public OpenAPI operations
  plus matching frontend metadata. Browser-only setup/session operations remain
  outside the public API document, as do the UI-only profile operations.
  Axum registration and generation now consume the same typed method, path,
  parameter, successful-response, and error-response contracts. Known 400, 401,
  403, 404, 409, 429, 500, and 503 responses are emitted explicitly as Problem
  Details while `default` remains the fallback for unexpected failures. UUID
  path parameters and Activity, User, and Service Account query filters are
  declared explicitly without operation-specific generator logic.
- OpenAPI verification compares the declared Rust error statuses with the
  generated .NET/frontend contract. Users path and JSON extractor failures are
  normalized to Citadel Problem Details, and authorization is evaluated before
  a malformed create payload is disclosed.
- `GET /api/v1/license/entitlements` returns only effective status, edition,
  and the five implemented capabilities to any authenticated Actor. It exposes
  no instance, customer, fingerprint, timestamp, warning, or raw-license
  metadata.
- The administrator `getLicense`, `installLicense`, `removeLicense`, and
  `getLicenseRequest` contracts retain `ResourceType.License` read/write/execute
  authorization. The request endpoint creates the singleton instance identity
  atomically and the same UUID survives service reconstruction and Core restart.
- Compact JWS verification accepts only the pinned Ed25519 algorithm, protected
  type, and known key IDs. Schema 1 Business compatibility maps to explicit
  Team capabilities; schema 2 enables only known signed capability keys.
  Temporal state is recalculated on every read even when the verified static
  payload is cached by fingerprint.
- Install/replacement locks the singleton table and compares an optimistic
  fingerprint under that lock. Concurrent first installs cannot both commit;
  replacements must name the installed license and a future license cannot
  displace an active one. Upsert/delete and typed `LicenseInstalled`,
  `LicenseReplaced`, or `LicenseRemoved` evidence commit together. Raw license
  material is stored only in `installedlicenses` and never serialized into API
  or Activity payloads.
- The supervised License transition monitor derives its next check from
  `notBefore`, `expiresAt`, and `graceUntil`, checks one second after a boundary,
  and caps idle checks at one hour. Install/remove notifications wake it so a
  newly installed License cannot inherit an obsolete sleep deadline.
- Validation timestamps and status are updated under the singleton License
  lock. A changed status and its typed `LicenseEnteredGracePeriod`,
  `LicenseExpired`, or `LicenseValidationFailed` System Activity commit in one
  transaction. Concurrent checks produce exactly one transition Activity.
- Application information explicitly advertises `SignalR` from .NET and
  `WebSocketV1` from Rust. The SPA keeps one implementation, authenticates the
  Rust socket with its in-memory bearer token, and invalidates License queries
  only after an authenticated subscription or a versioned
  `licenseStateChanged` envelope. The event payload is empty and contains no
  status, edition, customer, fingerprint, or raw License data.

## .NET test inventory and Rust parity

The slice started from the active .NET tests rather than its implementation
classes. Each applicable public behavior has a real PostgreSQL-backed Axum
test; additional Rust cases cover atomic rollback and ghost-Activity failures
that were implicit in the .NET Unit of Work behavior.

| .NET evidence | Rust evidence |
| --- | --- |
| `UserViewTests`: list, search, detail, and non-admin denial | `list_users_returns_the_dotnet_paging_projection_and_capabilities`, `search_users_matches_name_or_email_and_honors_the_limit`, `get_user_returns_enabled_state_roles_teams_and_resource_accesses`, `user_reads_require_a_human_administrator_even_with_user_execute_permission`, and compatible invalid/not-found cases |
| `UserCreateTests`: create, assignments, duplicate name, duplicate email | `create_user_persists_an_enabled_actor_and_safe_activity`, `create_user_with_assignments_persists_teams_roles_and_resource_access`, and both conflict tests |
| `UserPatchTests`: email/password/enabled, rename, Role add/remove, resource access add/remove, replacement, and clearing | Matching patch/rename/Role/resource-access/replacement/clear tests, plus persisted-identity password-policy and refresh-session invalidation checks |
| `UserDeleteTests`: delete, final administrator, concurrent administrator deletion | `delete_user_removes_the_user_and_sessions_but_keeps_safe_activity`, `deleting_the_last_administrator_returns_conflict_and_rolls_back`, and `concurrent_administrator_deletes_keep_one_enabled_administrator` |
| `IdentityActivityEndpointTests`: safe User lifecycle payloads and no ghost assignment evidence | Lifecycle assertions in create/patch/rename/delete tests plus `duplicate_role_assignment_returns_conflict_without_a_ghost_activity` |
| .NET Unit of Work atomicity implicit in the endpoint tests | `create_user_with_missing_assignment_rolls_back_every_row` and final-administrator rollback verify database state and Activity state after failure |
| Custom-access license policy used by the .NET User handlers | `create_user_with_custom_access_requires_the_license_and_rolls_back` proves the endpoint denial and absence of partial User/Actor rows |
| `TeamViewTests`: list, search, and detail | `list_search_and_get_preserve_the_team_projection` covers paging, capabilities, deterministic search, enabled state, Roles, Users, and Actor-aware members |
| `TeamCreateTests`: basic create, assignments, duplicate name | `create_with_assignments_is_atomic_and_writes_safe_activity` and `create_conflicts_and_missing_assignments_leave_no_partial_rows` cover the persisted aggregate and rollback boundaries |
| `TeamPatchTests`: enabled state, rename, member/Role/resource access add/remove, replacement, and clearing | The patch and incremental-route tests cover each mutation and prove that `userIds` replacement retains Service Account members |
| `TeamDeleteTests` and administrator guard behavior | `rename_and_delete_persist_lifecycle_activity_without_ghost_rows` and `last_administrator_guard_rolls_back_team_member_removal` prove lifecycle evidence and atomic safety rollback |
| Team Custom Access Control and Service Account membership policy | `custom_access_expansion_and_service_account_membership_require_the_license` proves endpoint denial without partial assignments or ghost Activity |
| `RoleViewTests` and `RolePermissionMatrixTests`: administrator-only Role reads and anonymous keyed matrix | `list_and_get_return_persisted_roles_permissions_and_capabilities`, `license_authorization_and_request_bounds_are_enforced`, and `permission_matrix_is_anonymous_keyed_and_matches_known_capabilities` |
| `RoleCreateTests` and matrix validation: create, conflict, invalid permission | `create_is_transactional_and_writes_a_safe_activity` plus `concurrent_same_name_creates_are_serialized` prove validation, persistence, safe evidence, and race serialization |
| `RolePatchTests`, `RoleDeleteTests`, and license downgrade behavior | `patch_replaces_permissions_and_license_downgrade_allows_only_reduction` and `system_roles_are_immutable_but_custom_roles_can_be_renamed_and_deleted` prove replacement, cleanup, immutable system Roles, and retained Activity |
| `LicenseVerifierTests` and `LicenseStateProviderTests`: signed-envelope validation, schema compatibility, temporal state, and explicit capabilities | Adapter verifier tests exercise real Ed25519 signatures, tampering, algorithm/key/instance failures, temporal boundaries, duplicate/legacy validation, and unknown capability filtering; application tests prove inactive metadata retention without effective capabilities |
| `LicenseEntitlementTests`: Community/request projections, authorization, install/replacement/removal, safe persistence, and stable identity | The six PostgreSQL-backed `licenses_http` cases prove minimal authenticated entitlements, administrator permissions, restart-stable identity, normalized opaque persistence, safe typed Activity, replacement conflict types, idempotent removal, and concurrent-install serialization |
| Accepted MFA integration matrix: normal login, optional/mandatory enrollment, verification, recovery, replay, lockout, policy, disable, rotation, and administrator reset | `mfa_endpoints_preserve_login_enrollment_recovery_replay_and_reset_semantics` executes the real nine-route Axum boundary and PostgreSQL adapter; MFA crypto unit tests separately prove Base32, provisioning URI, TOTP window/vector behavior, recovery normalization, HMAC verification, and generated-code shape |
| `OidcAuthenticationServiceTests`: PKCE authorization URL and trusted return URLs | Rust application unit tests preserve the same parameter/challenge and origin-validation cases and add string/array required-claim coverage |
| `OidcProviderEndpointTests` and `OidcAuthenticationCommandTests`: protected CRUD, discovery, state, auto-link, auto-provision, and default Role | `oidc_endpoints_preserve_admin_login_link_provision_and_replay_semantics` executes all twelve Axum routes against PostgreSQL and additionally proves ciphertext containment, secret-preserving PATCH, expiry, replay, concurrent callback exclusion, required claims, and disabled-User denial |
| `KeycloakOidcCompatibilityTests` protocol behavior | `protocol_discovers_exchanges_and_validates_a_signed_id_token` uses a local standards-shaped discovery/token/JWKS server and a real RSA-signed ID token; no public provider or network account is required |
| Existing frontend generated-client/resource flow and `oidc.spec.ts` Keycloak browser suite | The generic OpenAPI subset check verifies every migrated operation. `Test-Phase3Frontend.ps1` serves the production React bundle from Rust and runs the unchanged first-run/OIDC/profile/reload/logout/required-claim browser scenarios against real Keycloak. |
| Existing Access resource pages and Community licensing smoke coverage | `access.spec.ts` creates a User and Team through the production forms, verifies their list/detail routes, system Roles, gated Service Accounts, License rendering, and OIDC provider list/detail wiring. The unchanged `licensing.spec.ts` proves UI gating and exact 403 Problem Details against Rust. |
| `LicenseTransitionMonitorJobTests` and the transition worker's atomic persistence behavior | Application unit tests preserve the one-hour cap, one-second boundary margin, and ordered temporal boundaries. `concurrent_transition_checks_persist_status_and_one_system_activity_atomically` proves real PostgreSQL serialization and exactly one safe System Activity; `realtime_transition_notification_is_authenticated_versioned_and_metadata_free` proves the production WebSocket envelope. The SignalR provider test proves the shared SPA resynchronizes both License queries without consuming payload metadata. |
| Existing acceptance coverage | The Phase 3 HTTP harness creates, patches, and renames a User and Team, reduces and renames a seeded custom Role, proves all survive Core restart, then deletes them and verifies retained safe lifecycle Activity. |

## Verification

The following passed in the pinned Linux Rust environment:

- `cargo fmt --all --check`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace --all-targets`
- `cargo run --locked -p xtask -- openapi --check` (90 full and 38 public
  HTTP contracts)
- `rust/scripts/Test-Phase3Identity.ps1`
- `rust/scripts/Test-Phase3Frontend.ps1 -SkipBuild` (the Keycloak, Access, and
  Community licensing Playwright scenarios passed against the Rust server)
- `npm run build:image`
- `npm run test:run` (100 files and 340 tests passed)
- `npm run lint`
- focused frontend route/setup tests: 10 passed, 0 failed
- the complete .NET Unit suite through its Microsoft Testing Platform runner:
  1,021 passed, 0 failed
- the pre-slice complete .NET Integration baseline through its Microsoft Testing
  Platform runner: 827 passed, 0 failed
- the focused .NET Identity and Permissions integration regression after the
  differential slice: 117 passed, 0 failed
- the shared differential oracle: 22 resource capability entries, 13 Actor
  scenarios, and 38 permission probes passed through both PostgreSQL
  implementations
- `rust/scripts/Test-Phase3Identity.ps1`, including the .NET and Rust
  differential runners, database-backed Axum suites, restart checks, and
  cleanup

The Phase 3 harness uses two disposable PostgreSQL databases. It proves clean
setup, versioned password persistence, login/refresh, local-HTTP cookie behavior,
authenticated and setup-gated application information with restart-stable build
metadata, profile read/rename, lazy preference defaults, preference
validation/upsert and restart persistence, concurrent partial-patch safety,
direct Role projection,
authorization metadata, anonymous permission-matrix access after setup, license
denial, graceful process/container restart, Actor-scoped ACLs, immediate token
revocation, active browser-session ordering, current-session protection,
individual and revoke-other session invalidation, wrong/weak/external-password
rejection, current/all-session password-change invalidation, stale-login
rejection, atomic Activity writes and forced rollback, safe Activity payloads,
authorized Activity list/detail filtering, Activity HTTP contract compatibility,
Activity restart persistence, administrator User, Team, and Role projections,
User, Team, and Role mutation restart persistence, Role and resource-access
assignment/removal, duplicate-token rejection, archive behavior, and concurrent
token limits.

The User, Team, Role, License, MFA, and OIDC boundaries each have dedicated Rust HTTP integration test binaries.
It executes the real Axum routes, Identity authorization service, SQLx adapters,
PostgreSQL schema, JSON serialization, cache headers, and problem responses. Its
twenty-five cases mirror the .NET User read/mutation suites and add invalid
paging/search, not-found, unauthenticated, non-administrator, Service Account,
session invalidation, rollback, ghost Activity, and concurrency coverage.
Application unit tests independently prove filter trimming, offset overflow,
page/search bounds, not-found mapping, assignment deduplication, and permission
validation without touching storage. The ten Team endpoint cases additionally
cover Service Account member projection/retention, license denial, and prove
that concurrent Team deletion cannot remove every enabled administrator.
The seven Role cases additionally cover its keyed anonymous permission matrix,
malformed JSON normalization, license expansion/reduction policy, immutable
system Roles, transactional lifecycle evidence, and concurrent same-name
creation. The eight License cases additionally cover metadata minimization,
permission boundaries, stable instance identity, raw-source containment,
replacement conflicts, removal idempotence, concurrent singleton writes,
atomic temporal transitions, and authenticated metadata-free notifications.
The cross-resource authorization differential matrix is part of the normal
Phase 3 harness. Existing dedicated Axum User, Team, Role, License, Activity,
MFA, OIDC, profile, session, and Service Account tests retain endpoint-level
authentication, human/Admin, capability, status-code, and Problem Details
coverage around the shared permission core.

The accepted user-profile specification describes the display name as
non-unique, while the active .NET login and Rust Phase 3A login both accept a
name as an account identifier and the .NET profile mutation rejects duplicate
names. Phase 3B preserves the implemented unique-name behavior. The full User
cutover must explicitly resolve that specification mismatch before changing
login or uniqueness semantics.

## Remaining Phase 3 work

None. New identity/resource permissions added by later phases must extend the
shared fixture and keep both differential runners green.

The safe profile Activity ledger is complete. Other Rust-owned resource slices
must add typed command payloads and atomic evidence when their mutations move;
the compatibility reader intentionally supports their existing .NET rows
without introducing an untyped Rust write path.

Phase 4 Docker read ownership can now use the completed Actor/RBAC/resource-ACL
boundary.

## Phase 3T boundary cleanup

Identity is now an explicit bounded-context crate. It owns the User, Team,
Role, Service Account, MFA, OIDC, profile, session, and authorization models,
use cases, and narrow storage/protocol ports. Platform authorized reads and
runtime observations are owned by the separate `citadel-platforms` crate.

The cleanup is structural only: SQLx, Docker, Agent, cryptography, and OIDC HTTP
implementations stay in `citadel-adapters`; Axum and process composition stay
in `citadel-server`; no command bus, service locator, event bus, compatibility
facade, or per-resource crate was introduced. Existing unit and HTTP/database
integration tests moved with or import the owning context and remain the
behavioral proof.

## Phase 3U HTTP control-flow cleanup

User, Team, Role, and Service Account handlers now propagate expected failures
with ordinary Rust `Result`/`?` control flow through one shared
`identity_result` adapter. The adapter preserves the existing centralized
Problem Details mapping and request ID while removing repeated `match` and
early-return blocks. Authentication, authorization, validation, and use-case
ordering remain explicit in each route. The success path adds no allocation,
boxed future, dynamic dispatch, macro expansion, or request-scoped policy
object.
