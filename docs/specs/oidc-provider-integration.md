# Citadel OIDC Provider Integration

## Goal

Add support for external OpenID Connect login providers so Citadel users can authenticate through providers such as Keycloak, Auth0, Entra ID, Google Workspace, or any standards-compliant OIDC issuer.

OIDC only proves identity. After a successful provider login, Citadel still issues its own access token and refresh cookie, and Citadel remains the source of truth for roles, teams, resource access, permissions, audit, and session lifecycle.

Provider discovery must use the issuer metadata endpoint instead of hardcoded authorization, token, and JWKS URLs.

## Current Citadel Context

Citadel already has a local authentication flow:

- Public auth routes live under `/api/v1/authentication` in `src/Citadel.WebApi/Routes/PublicEndpoints.cs`.
- `Authentication.cs` endpoint methods are thin and delegate to Mediator commands.
- `LoginCommand` validates local credentials, creates a Citadel access token, creates a refresh token, persists the refresh token, and uses `IJwtService.CreateRefreshToken()` to set the `refresh_token` cookie.
- `RefreshTokenCommand` reads the `refresh_token` cookie and returns a new access token.
- `LogoutCommand` deletes the refresh cookie and removes the persisted refresh token.
- `User.GetJwtClaims(UserAuthInfo)` is the canonical Citadel access-token claim builder.
- Identity is modeled as `User` plus a user `Actor`; roles are assigned through `ActorRoles`; teams are assigned through `UsersTeams`.
- Database schema changes are modeled in `src/Citadel.Infrastructure.Migrations/EntityFramework/ApplicationDbContext.cs`, then EF migrations and SQL scripts are generated.
- Public route changes require updating OpenAPI snapshots and generated frontend API resources.
- API models used by endpoints must be added to `src/Citadel.WebApi/Routes/Endpoints/STJContext/ApplicationJsonContext.cs`.

The OIDC implementation should plug into this flow rather than introduce a parallel session system.

## MVP Scope

### In Scope

- Admin-configured OIDC providers.
- Multiple providers.
- Login page buttons for enabled providers.
- Authorization Code Flow with PKCE.
- Provider discovery from issuer URL.
- ID token validation.
- Just-in-time user creation when explicitly enabled.
- Linking an OIDC identity to a local Citadel user when explicitly enabled.
- Optional default role assignment for newly provisioned users.
- Existing Citadel JWT and refresh-cookie behavior after successful login.
- Activity events for provider CRUD and login/provisioning/linking outcomes where the current activity model can support them.

### Deferred From MVP

- SCIM provisioning.
- SAML.
- Provider-managed authorization as the primary permission model.
- Storing provider refresh tokens.
- Calling provider APIs after login.
- Dynamic provider icons/custom branding beyond display name.
- Automatic admin assignment from OIDC claims.
- Full claim-based role/team synchronization with stale assignment removal.
- RP-initiated provider logout.

Role/team claim mapping is valuable, but Citadel currently does not track whether an `ActorRoles` row or `UsersTeams` row was created manually or by an external mapping. Do not implement removal of stale mapped assignments until assignment source is modeled.

## Recommended Flow

Use Authorization Code Flow with PKCE. Do not implement implicit flow or password grant.

Citadel should handle the flow server-side:

1. User opens `/login`.
2. Frontend calls `GET /api/v1/authentication/oidc/providers`.
3. User clicks `Continue with <Provider>`.
4. Browser navigates to `GET /api/v1/authentication/oidc/{providerId}/login?returnUrl=/stacks`.
5. Backend validates the provider and creates:
   - `state`
   - `nonce`
   - `code_verifier`
   - `code_challenge`
6. Backend stores temporary auth state server-side.
7. Backend redirects to the provider authorization endpoint.
8. Provider redirects to `GET /api/v1/authentication/oidc/{providerId}/callback?code=...&state=...`.
9. Backend validates and consumes `state`.
10. Backend exchanges `code` for tokens.
11. Backend validates the ID token:
    - signature using provider JWKS
    - issuer
    - audience/client ID
    - expiration
    - nonce
    - required claims
12. Backend resolves, links, or creates a local Citadel user.
13. Backend assigns the configured default role for a newly created user, if present.
14. Backend issues the normal Citadel refresh cookie using the same logic as local login.
15. Backend redirects back to the validated `returnUrl`.
16. The frontend bootstrap refresh flow calls the existing refresh endpoint, obtains a Citadel access token, and redirects to the originally requested route when applicable.

Do not place the Citadel access token in the URL query string. The current implementation uses the existing refresh-cookie bootstrap path rather than a one-time frontend completion code.

## Backend Design

### Feature Layout

Use the existing project organization:

- Domain entities: `src/Citadel.Domain/Entities/Oidc`
- Application commands/queries/models: `src/Citadel.Application/Features.Oidc`
- Application services: `src/Citadel.Application/Services/Oidc`
- Persistence repositories and DTOs: `src/Citadel.Infrastructure/Persistence`
- Endpoint methods: `src/Citadel.WebApi/Routes/Endpoints/Oidc.cs`
- Endpoint request/response models: `src/Citadel.WebApi/Routes/Endpoints/Resources/Oidc`
- Route registration: `src/Citadel.WebApi/Routes/PublicEndpoints.cs`
- Tests:
  - unit: `test/Citadel.Tests.Unit/Application/Features/Oidc`
  - integration: `test/Citadel.Tests.Integration/Application/Features.Oidc`

Register OIDC services and repositories through `ApplicationModule.cs`, `UnitOfWork.cs`, and `IUnitOfWork`.

### Session Issuing

Extract the shared token issuing logic from `LoginCommand` into an application service, for example:

```csharp
public interface ICitadelSessionService
{
    Task<LoginResponse> CreateSessionAsync(UserAuthInfo userAuthInfo, CancellationToken cancellationToken);
}
```

Responsibilities:

- Create access token with `User.GetJwtClaims(userAuthInfo)`.
- Create refresh token with `IJwtService.CreateRefreshToken()`.
- Persist `RefreshToken`.
- Enforce the existing max refresh-token count per user.
- Commit through the current unit of work boundary used by the command.
- Populate `IRoleCache`.

Then use this service from both `LoginCommand` and OIDC callback completion.

## Data Model

Use classes for mutable domain entities, consistent with identity entities such as `User`, `Actor`, and `Role`.

Use `Guid.CreateVersion7()` for new entity IDs and `DateTime.UtcNow` for timestamps. Prefer `DateTime` over `DateTimeOffset` because the current schema consistently uses `timestamp with time zone` mapped from `DateTime`.

### `OidcProvider`

```csharp
namespace Domain.Entities.Oidc;

public sealed class OidcProvider : IAuditedEntity
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; }
    public string DisplayName { get; private set; }
    public string Issuer { get; private set; }
    public string ClientId { get; private set; }
    public string? ClientSecretCiphertext { get; private set; }
    public string Scopes { get; private set; } = "openid profile email";
    public bool Enabled { get; private set; } = true;
    public bool AutoProvisionUsers { get; private set; }
    public bool AllowEmailAutoLink { get; private set; }
    public bool RequireEmailVerified { get; private set; } = true;
    public string? AllowedEmailDomains { get; private set; }
    public string? RequiredClaimName { get; private set; }
    public string? RequiredClaimValues { get; private set; }
    public Guid? DefaultRoleId { get; private set; }
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public Guid CreatedByActorId { get; private set; }
    public DateTime UpdatedAt { get; private set; } = DateTime.UtcNow;
}
```

Notes:

- `Name` is the stable admin-facing key. Enforce uniqueness on normalized name if a normalized column is added.
- `DisplayName` is shown on the login button.
- `Issuer` should be normalized by trimming trailing slashes for comparison, but preserve a canonical string.
- `AllowedEmailDomains` and `RequiredClaimValues` can be stored as JSON text, following existing JSON column patterns.
- Do not expose `ClientSecretCiphertext` through API responses.
- Use the existing secret encryption approach if one exists by the time this is implemented; otherwise introduce a focused `ISecretProtector` service and use it for the client secret only.

### `OidcExternalLogin`

```csharp
namespace Domain.Entities.Oidc;

public sealed class OidcExternalLogin
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid ProviderId { get; private set; }
    public Guid UserId { get; private set; }
    public string Subject { get; private set; }
    public string? EmailAtLinkTime { get; private set; }
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public DateTime? LastLoginAt { get; private set; }
}
```

Constraints:

- Unique `(ProviderId, Subject)`.
- Unique `(ProviderId, UserId)`.
- Foreign key `ProviderId -> OidcProviders.Id` with cascade delete.
- Foreign key `UserId -> Users.Id` with restrict delete unless user deletion flow explicitly cleans up external logins.

### `OidcAuthState`

Temporary auth state should be server-side. For the MVP, a database table is acceptable and easier to run in a single-node deployment without adding a distributed cache.

```csharp
namespace Domain.Entities.Oidc;

public sealed class OidcAuthState
{
    public string State { get; private set; }
    public Guid ProviderId { get; private set; }
    public string Nonce { get; private set; }
    public string CodeVerifier { get; private set; }
    public string? ReturnUrl { get; private set; }
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public DateTime ExpiresAt { get; private set; }
    public DateTime? ConsumedAt { get; private set; }
}
```

Rules:

- Expiry: 5 minutes.
- One-time use only.
- `State` is the primary key.
- Delete consumed/expired states periodically or opportunistically during login start/callback.

### Deferred: `OidcLoginCompletion`

The current implementation does not need a separate completion code because the callback sets the normal refresh cookie and redirects to the frontend. If the frontend must later exchange a short-lived local code for `LoginResponse`, add a one-time table:

```csharp
public sealed class OidcLoginCompletion
{
    public string Code { get; private set; }
    public Guid UserId { get; private set; }
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public DateTime ExpiresAt { get; private set; }
    public DateTime? ConsumedAt { get; private set; }
}
```

Rules:

- Expiry: 1 minute.
- One-time use only.
- Use this only to bridge provider callback redirects to the existing SPA auth state.

## Deferred Data Model For Claim Sync

Do not add these tables in the MVP unless role/team mapping is implemented immediately:

- `OidcRoleMappings`
- `OidcTeamMappings`
- assignment source columns for `ActorRoles` and `UsersTeams`

When implementing claim-based sync, first add:

```csharp
public enum AssignmentSource
{
    Manual = 1,
    OidcMapping = 2,
    System = 3
}
```

Then add source metadata to `ActorRoles` and `UsersTeams` so OIDC can remove only OIDC-managed assignments and preserve manual admin assignments.

## API Endpoints

Keep routes under the existing `/api/v1/authentication` group where possible.

### Public OIDC Login Endpoints

```http
GET /api/v1/authentication/oidc/providers
```

Returns enabled providers for the login page.

```json
[
  {
    "id": "provider-id",
    "displayName": "Company SSO"
  }
]
```

```http
GET /api/v1/authentication/oidc/{providerId:guid}/login?returnUrl=/stacks
```

Starts OIDC login.

Behavior:

- Validate provider exists and is enabled.
- Validate `returnUrl` is either root-relative, same-origin with the callback URL, or an explicitly configured allowed CORS origin.
- Generate state, nonce, PKCE verifier/challenge.
- Store auth state.
- Redirect to provider authorization endpoint.
- Apply strict auth rate limiting.

```http
GET /api/v1/authentication/oidc/{providerId:guid}/callback
```

Handles provider callback.

Behavior:

- Accept `code`, `state`, and provider error query values.
- Validate provider id matches auth state.
- Validate and consume state.
- Exchange authorization code.
- Validate ID token.
- Resolve/link/create Citadel user.
- Issue the normal Citadel refresh cookie.
- Redirect to the stored `returnUrl`.

### Admin Provider Endpoints

Use a protected resource group under `/api/v1/oidcProviders`, consistent with existing resource route naming such as `/resourceBindings` and `/gitRepositories`.

```http
GET /api/v1/oidcProviders
GET /api/v1/oidcProviders/{id:guid}
POST /api/v1/oidcProviders
PATCH /api/v1/oidcProviders/{id:guid}
DELETE /api/v1/oidcProviders/{id:guid}
POST /api/v1/oidcProviders/{id:guid}/testDiscovery
```

Authorization:

- Require normal authorization on the route group.
- For MVP, protect provider management with an admin-level global permission using the existing permission attribute/policy pattern.
- If a dedicated resource type is added, add it to `ResourceType`, `PermissionMatrix`, seeded permissions, and frontend capability views.

Do not expose client secrets in any response. Allow replacing a secret by sending a new value; leaving the secret field omitted must preserve the current secret.

## User Resolution Rules

After ID token validation:

1. Read `sub`.
2. Find `OidcExternalLogin` by `(providerId, sub)`.
3. If found:
   - Load linked user auth info.
   - Require the linked user actor to be enabled.
   - Update `LastLoginAt`.
4. If not found:
   - Read `email`.
   - If `AutoProvisionUsers` is false and `AllowEmailAutoLink` is false, reject login.
   - If `RequireEmailVerified` is true, require `email_verified = true` before provisioning or email auto-link.
   - If `AllowedEmailDomains` is configured, require email domain match.
   - If `RequiredClaimName` is configured, require at least one configured claim value.
   - If `AllowEmailAutoLink` is true and exactly one local user exists with the verified email, link to that user.
   - Otherwise, if `AutoProvisionUsers` is true, create a new `Actor` and `User`.
   - Otherwise reject login.
5. If a user was created and `DefaultRoleId` is configured, add that role to the user's actor.
6. Return `UserAuthInfo` for token issuing.

Important:

- Never auto-create or auto-assign an admin user from provider claims in the MVP.
- Never auto-link by unverified email.
- Do not use email as the permanent OIDC identifier. Use `(ProviderId, sub)`.
- For generated local usernames, derive a readable base from preferred username/email/name and resolve conflicts deterministically.
- Provisioned users should have a random unusable local password unless Citadel adds a first-class external-only password state.

## Security Requirements

### Token Validation

Validate:

- ID token signature.
- Issuer equals configured provider issuer.
- Audience contains configured client ID.
- Token expiration with minimal clock skew.
- Nonce.
- Allowed signing algorithms. Reject `none`.
- Required claims: `sub`; `email` only when needed for linking/provisioning rules.

Use `Microsoft.IdentityModel.Protocols.OpenIdConnect` and `Microsoft.IdentityModel.JsonWebTokens` or `System.IdentityModel.Tokens.Jwt`.

### Provider Metadata

- Fetch discovery from `{issuer}/.well-known/openid-configuration`.
- Cache discovery metadata and JWKS.
- Refresh JWKS on key miss.
- Do not allow arbitrary callback URLs.
- Redirect URI must be exact:
  - `{CitadelBaseUrl}/api/v1/authentication/oidc/{providerId}/callback`
- Add a `PublicUrl`/`BaseUrl` configuration if Citadel does not already have one reliable way to build external callback URLs behind reverse proxies.

### Secrets

- Encrypt client secrets at rest.
- Never log client secrets, authorization codes, access tokens, refresh tokens, or ID tokens.
- Never return client secrets through API.
- Let admins replace a secret without revealing the existing one.

### State And Redirects

- State is one-time use.
- Auth state expires after 5 minutes.
- `returnUrl` must be root-relative, callback-origin, or an explicitly configured allowed frontend origin.
- Callback errors should be generic in UI and detailed in server logs/activity metadata without raw tokens.
- Apply rate limiting to login start and callback endpoints.

### Session

- Do not store provider access tokens or refresh tokens in the MVP.
- After successful OIDC login, issue normal Citadel access token and refresh cookie.
- Existing logout invalidates only the Citadel refresh session.
- Local admin login must remain available as break-glass access.

## Frontend UX

### Login Page

Update `src/Citadel.FrontEnd/src/features/auth/login.tsx`:

- Fetch enabled providers with `useRead('listOidcLoginProviders')`.
- Show buttons below or above the local login form:
  - `Continue with Company SSO`
  - `Continue with Keycloak`
- Button click should navigate the browser to `/api/v1/authentication/oidc/{providerId}/login?returnUrl=<current return target>`.
- Keep local username/password login available.

The callback redirects to the frontend after the backend sets the refresh cookie. The existing `AuthProvider` bootstrap refresh then obtains the access token and redirects authenticated users away from `/login`.

### Admin Provider Management

Add a new settings item under the existing Settings sidebar:

- Label: `OIDC Providers`
- Route: `/oidc-providers`

Implement as a regular resource feature following current frontend patterns:

- `src/Citadel.FrontEnd/src/features/oidc-providers/index.tsx`
- `table.tsx`
- `actions.tsx`
- `form/index.tsx`
- `form/form.tsx`
- `form/actions.tsx`

Provider table:

- Checkbox multiselect.
- Dropdown row actions.
- Columns: Display Name, Issuer, Client ID, Enabled, Auto Provision, Updated.

Provider form fields:

- Display name.
- Name.
- Issuer URL.
- Client ID.
- Client secret replacement field.
- Scopes.
- Enabled.
- Auto-provision users.
- Allow email auto-link.
- Require verified email.
- Allowed email domains.
- Required claim name.
- Required claim values.
- Default role.

Actions:

- Save.
- Disable/enable.
- Delete.
- Test discovery.
- Copy redirect URI.

## Backend Services

### `OidcProviderService`

Responsibilities:

- Validate provider create/update input.
- Encrypt/decrypt client secret.
- Fetch and validate discovery metadata.
- Build exact redirect URI.
- Return provider views without secrets.

### `OidcLoginService`

Responsibilities:

- Start login.
- Create auth state.
- Build authorization URL with PKCE.
- Handle callback.
- Exchange authorization code.
- Validate ID token.
- Resolve/create/link local user.
- Issue the normal Citadel refresh cookie and redirect to the validated return URL.

### `OidcUserResolver`

Responsibilities:

- Resolve existing external login.
- Enforce email/domain/required-claim gates.
- Link external login to local user when allowed.
- Create actor and user when auto-provisioning is allowed.
- Assign default role for newly provisioned users.
- Return `UserAuthInfo`.

## Persistence Requirements

Add repository interfaces to `IUnitOfWork`:

- `IOidcProviderRepository`
- `IOidcExternalLoginRepository`
- `IOidcAuthStateRepository`
- `IOidcLoginCompletionRepository` only if completion codes are added later.

Use Dapper repositories under `src/Citadel.Infrastructure/Persistence`, DTOs under `Persistence/Dtos`, and mapping extensions under `Persistence/Mappers` where useful.

Repository methods should avoid per-item loops in hot paths. Fetch provider, state, external login, and user auth info in as few queries as practical during callback.

## Migration Requirements

Model schema changes in `ApplicationDbContext.cs`, then generate an EF migration.

Expected MVP tables:

- `OidcProviders`
- `OidcExternalLogins`
- `OidcAuthStates`
- `OidcLoginCompletions` only if completion codes are added later

Indexes:

- `OidcProviders.Name` unique, or `NormalizedName` unique if added.
- `OidcProviders.Issuer, ClientId` unique if duplicate client registrations should be rejected.
- `OidcExternalLogins.ProviderId, Subject` unique.
- `OidcExternalLogins.ProviderId, UserId` unique.
- `OidcAuthStates.ExpiresAt`.
- `OidcLoginCompletions.ExpiresAt`.

After migration:

- Update generated SQL script under `src/Citadel.Infrastructure/Scripts`.
- Regenerate OpenAPI snapshots and frontend generated API files.

## Activity Events

The current activity model is resource-oriented and only supports known `ActivityResourceType`/`ActivityEventType` pairs. For MVP, either:

- Add `OidcProvider`/`Authentication` resource/event types and activity info records, or
- Log security-relevant login failures/successes through structured server logs first and add full activity support as a small follow-up.

If adding activity events, include:

- OIDC provider created.
- OIDC provider updated.
- OIDC provider deleted.
- OIDC provider enabled/disabled.
- OIDC login success.
- OIDC login failed.
- OIDC user auto-provisioned.
- OIDC identity linked to existing user.

Do not include raw tokens, authorization codes, client secrets, or full claim payloads in activity details.

## Testing Plan

### Unit Tests

- PKCE verifier/challenge generation.
- Return URL validation rejects external URLs.
- OIDC claim extraction supports string and array claims.
- User resolver rejects unknown users when provisioning is disabled.
- User resolver rejects unverified email when required.
- User resolver links only when email auto-link is enabled and the local email match is unique.
- User resolver creates user and actor when provisioning is enabled.
- Default role assignment happens only for newly provisioned users.
- Secret replacement preserves existing secret when omitted.

### Integration Tests

- Public provider list returns only enabled providers and never includes secrets.
- Admin CRUD protects routes and never returns secrets.
- Test discovery validates required metadata fields.
- OIDC login start stores state and redirects to the provider authorization endpoint.
- Callback rejects missing, expired, consumed, or mismatched state.
- Callback rejects invalid issuer/audience/nonce/signature.
- Successful callback sets the normal refresh token cookie and redirects to the validated return URL.
- Refresh token cookie is usable by the existing refresh endpoint after callback.
- Local login still works.
- Logout still clears the Citadel refresh session.

Use a fake OIDC metadata/JWKS/token server in integration tests rather than relying on a public provider.

## Acceptance Criteria

### Provider Management

- Admin can create, edit, enable/disable, and delete OIDC providers.
- Admin can test discovery before enabling a provider.
- Redirect URI can be copied from the provider form.
- Client secret is encrypted and never returned by API.
- Disabled providers do not appear on the login page.

### Login

- User can log in through a configured provider.
- Citadel validates state, nonce, issuer, audience, expiry, and signature.
- Citadel creates its own normal session after OIDC login.
- Provider tokens are not stored.
- Citadel access token is not exposed in URL query strings.

### Provisioning And Linking

- Unknown users are rejected when auto-provisioning and email auto-link are both disabled.
- Unknown users are created when auto-provisioning is enabled and gates pass.
- Email auto-link only works when explicitly enabled and email is verified when required.
- Existing external login resolves by provider plus subject.
- Disabled linked users cannot log in.

### Security

- Implicit flow is not implemented.
- Password grant is not implemented.
- Client secrets, provider tokens, authorization codes, and ID tokens are not logged.
- Callback state is one-time use.
- Local admin login remains available.

## Implementation Order

1. Add domain entities, repository interfaces, EF model configuration, migration, SQL script.
2. Add provider CRUD commands/queries/endpoints/views.
3. Add provider discovery service and `testDiscovery`.
4. Extract `ICitadelSessionService` from local login.
5. Add OIDC login start state generation and redirect.
6. Add token exchange and ID token validation.
7. Add user resolver/linking/provisioning/default role assignment.
8. Add login page provider buttons.
9. Add OIDC provider admin UI.
10. Regenerate OpenAPI/frontend API.
11. Add unit/integration tests and run backend/frontend builds.
