# Citadel Offline Licensing and Quota Enforcement

## Goal

Add fully offline licensing to Citadel Core.

Citadel must keep working without a license as Community edition, with built-in quota limits. A signed Business license raises those limits for a specific Citadel installation without requiring internet access.

Licensing must be non-destructive. If a license is missing, invalid, expired, or bound to another instance, Citadel falls back to Community limits and blocks only operations that would increase usage beyond the effective quota. Existing resources are not deleted, disabled, disconnected, or modified.

## Citadel Alignment

Follow existing Citadel conventions:

- Public API routes are under `/api/v1`.
- Add route registration in `src/Citadel.WebApi/Routes/PublicEndpoints.cs`.
- Keep endpoint methods thin under `src/Citadel.WebApi/Routes/Endpoints`.
- Put license application logic under `src/Citadel.Application/Features.Licensing`.
- Put license domain values under `src/Citadel.Domain/Entities/Licensing` or `src/Citadel.Domain/Contracts/Resources/Licensing`.
- Put infrastructure persistence under `src/Citadel.Infrastructure/Persistence`.
- Add repository interfaces to `IUnitOfWork`.
- Keep repository code compatible with Dapper AOT. Pass anonymous objects directly to Dapper calls and avoid custom parameter wrapper records.
- Add request and response records to `src/Citadel.WebApi/Routes/Endpoints/STJContext/ApplicationJsonContext.cs`.
- Add domain JSON contracts to `DomainJsonContext.cs` when domain serialization is required.
- Update generated frontend API files through the existing generation flow, not by hand.
- Add database schema changes through `src/Citadel.Infrastructure.Migrations/EntityFramework/ApplicationDbContext.cs`.
- Generate EF migrations and SQL scripts. Do not hand-write scripts under `src/Citadel.Infrastructure/Scripts`.
- Use `TimeProvider` instead of `DateTime.UtcNow` in license validation and transition checks.
- Use result-based expected failures, not exceptions, for quota and license validation branches.

## Community Limits

Community edition is the default state when no valid license is installed.

| Resource | Community allowance |
| --- | ---: |
| OIDC providers | 1 |
| Edge Agent platforms | 1 |
| Secret providers | 1 |
| Custom roles | 0 |
| Active users | 10 |
| Total platforms | 5 |

Built-in system roles remain free:

```text
Admin
Operator
Viewer
```

Use stable quota identifiers:

```csharp
public enum LicenseLimit
{
    OidcProviders,
    EdgeAgentPlatforms,
    SecretProviders,
    CustomRoles,
    ActiveUsers,
    Platforms
}
```

Signed license payload keys must be stable strings, not numeric enum values:

```text
oidc-providers
edge-agent-platforms
secret-providers
custom-roles
active-users
platforms
```

Effective limits are:

```text
Community limit                    when no valid signed license is active
max(Community limit, signed limit) when a license is Valid or GracePeriod
```

Missing signed limits fall back to Community. Do not introduce an unlimited sentinel. Large customers should receive explicit numeric limits.

## License File Format

Use compact JWS:

```text
BASE64URL(protected-header).BASE64URL(payload).BASE64URL(signature)
```

The license is signed but not encrypted. It can be stored as:

```text
<customer>.citadel-license
```

Use Ed25519 keys with the fully specified JOSE algorithm identifier `alg = Ed25519`.

RFC 9864 registers `Ed25519` and `Ed448` as fully specified JOSE algorithm identifiers and marks the polymorphic `EdDSA` identifier as deprecated. Citadel has not shipped a licensing format yet, so it should start with the current identifier instead of accepting a deprecated value.

This is a compact JWS containing a Citadel license payload. It is not a JWT. The payload uses Citadel-owned field names such as `issuer`, `audience`, `issuedAt`, `notBefore`, and `expiresAt` rather than JWT registered claim names.

Protected header:

```json
{
  "alg": "Ed25519",
  "typ": "citadel-license+jws",
  "kid": "citadel-license-2026-01"
}
```

Verifier requirements:

- Accept only `alg = Ed25519`.
- Accept only `typ = citadel-license+jws`.
- Require a known `kid`.
- Verify against an embedded Citadel public key selected by `kid`.
- Reject `alg = none`.
- Reject `alg = EdDSA`.
- Reject HMAC, RSA, ECDSA, and unknown algorithms.
- Reject public keys embedded in the license.
- Reject remote key references such as `jku`, `x5u`, and similar headers.
- Reject unsupported critical JOSE headers.
- Do not use Citadel's normal JWT authentication configuration for license verification.

The private signing key must never live in this repository, the Docker image, application settings, CI logs, test fixtures outside test-only keys, or the frontend.

## License Payload

Schema version `1`. `replacedLicenseId` is present only for renewed or rehosted licenses that supersede a previous license; first-time licenses omit it.

```json
{
  "schema": 1,
  "product": "citadel",
  "issuer": "citadel-p",
  "audience": "citadel-core",
  "licenseId": "lic_2026_000001",
  "replacedLicenseId": "lic_2025_000123",
  "customer": {
    "id": "customer-example",
    "name": "Example Corp"
  },
  "edition": "Business",
  "instanceId": "019f0000-0000-7000-8000-000000000001",
  "issuedAt": "2026-07-12T00:00:00Z",
  "notBefore": "2026-07-12T00:00:00Z",
  "expiresAt": "2027-07-12T00:00:00Z",
  "graceUntil": "2027-07-26T00:00:00Z",
  "limits": {
    "oidc-providers": 5,
    "edge-agent-platforms": 20,
    "secret-providers": 10,
    "custom-roles": 50,
    "active-users": 100,
    "platforms": 50
  }
}
```

Validation rules:

- `schema` must be supported.
- `product` must be `citadel`.
- `issuer` must be `citadel-p`.
- `audience` must be `citadel-core`.
- `licenseId` must be non-empty.
- `replacedLicenseId` is optional. When present, it must be non-empty, must not equal `licenseId`, and is used to record renewals or rehosted licenses that supersede a previous license.
- `edition` must be `Business` for paid licenses.
- `instanceId` must match the persisted Citadel instance ID.
- `issuedAt`, `notBefore`, and `expiresAt` must be valid UTC timestamps.
- `issuedAt <= notBefore`.
- `issuedAt <= expiresAt`.
- `notBefore <= expiresAt`.
- `graceUntil` is optional but, when present, must be greater than or equal to `expiresAt`.
- Limit values must be non-negative integers.
- Unknown limit keys are ignored for forward compatibility and surfaced in validation diagnostics.
- Known missing limit keys fall back to Community limits.

Input size and defensive parsing limits:

- Compact license input: maximum 64 KB.
- `kid`: maximum 128 characters.
- `licenseId`: maximum 128 characters.
- `replacedLicenseId`: maximum 128 characters, when present.
- `customer.id`: maximum 128 characters.
- `customer.name`: maximum 256 characters.
- `limits`: maximum 32 entries.
- JSON depth: use a small explicit parser depth limit.

Do not include customer email in the signed license payload. The license is signed, not encrypted, so anyone with the license file can read the payload. Customer email belongs in the private issuance registry.

Input normalization rules:

- The server may trim leading and trailing ASCII whitespace introduced by copy/paste.
- Reject whitespace inside the compact JWS.
- Store the resulting exact three-segment string.
- Verify that exact string.
- Calculate the fingerprint from that exact stored string.
- Do not decode, reserialize, re-encode, reorder, or otherwise normalize any JWS segment before signature verification or fingerprint calculation.

## Instance Identity

Citadel Core needs a stable instance ID for offline license binding.

Add a singleton persisted identity row:

```text
CitadelInstanceIdentity
- Id                integer primary key, constrained to 1
- InstanceId        uuid not null unique
- CreatedAt         timestamptz not null
```

Rules:

- Generate the instance ID once on first startup or first license request.
- Store it in the database so it survives container rebuilds and application restarts.
- Do not derive it from machine identifiers, Docker host identifiers, IP addresses, hostnames, or deployment paths.
- Do not regenerate it automatically after database restore.
- Include it in the license request endpoint so an administrator can request an offline license.
- The repository must perform an atomic singleton insert-or-read operation. Two Core instances starting concurrently must not generate competing identities.
- The instance identity is immutable; do not add `UpdatedAt`.

## Persistence

Add a singleton installed-license table:

```text
InstalledLicenses
- Id                    int primary key, fixed value 1
- RawLicense            text, required
- Fingerprint           text, required
- InstalledAt           timestamptz, required
- InstalledByActorId    uuid, nullable
- LastValidatedAt       timestamptz, nullable
- LastValidationStatus  text, nullable
- LastValidationErrorCode text, nullable
```

The raw license is the source of truth but must never be returned by API responses or activity events.

The fingerprint is a SHA-256 hash of the exact stored compact JWS string after trimming only surrounding ASCII whitespace from pasted input. It is safe to show in API responses and activity events.

Repository additions:

```csharp
public interface IInstanceIdentityRepository
{
    Task<CitadelInstanceIdentity> GetOrCreateAsync(
        Guid candidateInstanceId,
        DateTimeOffset createdAt,
        CancellationToken cancellationToken);

    Task<CitadelInstanceIdentity?> GetAsync(CancellationToken cancellationToken);
}

public interface IInstalledLicenseRepository
{
    Task<InstalledLicense?> GetAsync(CancellationToken cancellationToken);
    Task<int> UpsertAsync(InstalledLicense license, CancellationToken cancellationToken);
    Task<int> DeleteAsync(CancellationToken cancellationToken);
    Task<int> UpdateValidationStatusAsync(
        LicenseStatus status,
        DateTimeOffset validatedAt,
        string? validationErrorCode,
        CancellationToken cancellationToken);
}
```

Dapper AOT notes:

- Use direct anonymous parameter objects in SQL calls.
- Prefer database-side counts for usage queries.
- Avoid loading all resources into memory just to count quotas.

## License State

Use these statuses:

```csharp
public enum LicenseStatus
{
    Community,
    Valid,
    GracePeriod,
    NotYetValid,
    Expired,
    Invalid,
    InstanceMismatch,
    UnsupportedSchema,
    UnknownSigningKey
}
```

Effective limit behavior:

| Status | Effective limits |
| --- | --- |
| Community | Community |
| Valid | Signed license merged with Community |
| GracePeriod | Signed license merged with Community |
| NotYetValid | Community |
| Expired | Community |
| Invalid | Community |
| InstanceMismatch | Community |
| UnsupportedSchema | Community |
| UnknownSigningKey | Community |

Temporal state must be derived from `TimeProvider.GetUtcNow()` whenever the effective license state is requested:

```text
now < notBefore
    => NotYetValid

notBefore <= now <= expiresAt
    => Valid

expiresAt < now <= graceUntil
    => GracePeriod

now > graceUntil
    => Expired

When graceUntil is absent:
    now > expiresAt => Expired
```

The verifier must validate temporal field ordering before deriving the status:

```text
issuedAt <= notBefore
issuedAt <= expiresAt
notBefore <= expiresAt
graceUntil >= expiresAt, when present
```

Add application services:

```csharp
public interface ILicenseStateProvider
{
    ValueTask<LicenseState> GetCurrentAsync(CancellationToken cancellationToken);
    ValueTask ReloadAsync(CancellationToken cancellationToken);
}

public interface ILicenseVerifier
{
    LicenseVerificationResult Verify(string rawLicense, CitadelInstanceIdentity instance, DateTimeOffset now);
}
```

`ILicenseStateProvider` may cache parsed and verified static data, but it must not cache `Valid`, `GracePeriod`, or `Expired` indefinitely. Signature verification, payload parsing, key lookup, instance binding, schema validation, and static payload validation may be cached. Time-dependent status must be derived from `TimeProvider.GetUtcNow()` on every `GetCurrentAsync` call.

The provider must not hold an `IUnitOfWork` or database connection for the lifetime of the cache. Use scoped resolution on demand, similar to long-running services that rely on `IServiceScopeFactory` and `IDbWorkQueue`.

Invalidate the cache after license install and removal.

## Quota Usage

Add one repository method that returns the current quota usage directly from the database:

```csharp
Task<LicenseUsageSnapshot> GetLicenseUsageSnapshotAsync(CancellationToken cancellationToken);

public sealed record LicenseUsageSnapshot(
    int OidcProviders,
    int EdgeAgentPlatforms,
    int SecretProviders,
    int CustomRoles,
    int ActiveUsers,
    int Platforms);
```

Use a single PostgreSQL query with scalar subqueries or CTEs. This avoids six round trips and gives the license page and quota validator one consistent database state. Specialized existing count methods can remain only where already useful outside licensing.

Exact mapping:

| Limit | Count source |
| --- | --- |
| `OidcProviders` | Rows in `OidcProviders` |
| `EdgeAgentPlatforms` | `Platforms` where `ConnectorType = EdgeAgent` |
| `SecretProviders` | Rows in `SecretProviders` for external providers |
| `CustomRoles` | `Roles` where `RoleType = Custom` |
| `ActiveUsers` | Users whose actor is enabled |
| `Platforms` | Rows in `Platforms` |

Count disabled OIDC providers and disconnected Edge Agent platforms. They are configured capacity and should still count.

Count active users only. Creating a disabled user does not increase `ActiveUsers`; enabling a disabled user does.

## Quota Enforcement

Add:

```csharp
public interface ILicenseQuotaService
{
    ValueTask<LicenseQuotaOverview> GetOverviewAsync(CancellationToken cancellationToken);

    ValueTask<Result> EnsureCanIncreaseAsync(
        IReadOnlyDictionary<LicenseLimit, int> increases,
        IUnitOfWork unitOfWork,
        CancellationToken cancellationToken);
}
```

Quota-increasing commands must call `EnsureCanIncreaseAsync` before persisting the change, using the same `IUnitOfWork` and transaction as the write.

For concurrent create requests, protect check-and-insert with a transaction-scoped lock. The service must:

- Acquire the singleton `CitadelInstanceIdentity` row lock.
- Fetch one `LicenseUsageSnapshot`.
- Validate every requested increase.
- Keep the transaction and lock active until the resource write is committed or rolled back.

Compound mutations must pass every affected limit at once. For example, Edge Agent platform creation increases both total platforms and Edge Agent platforms:

```csharp
await quotaService.EnsureCanIncreaseAsync(
    new Dictionary<LicenseLimit, int>
    {
        [LicenseLimit.Platforms] = 1,
        [LicenseLimit.EdgeAgentPlatforms] = 1
    },
    unitOfWork,
    cancellationToken);
```

Enforcement points:

- `CreateOidcProvider`
- Any OIDC restore or duplicate flow if added later
- `CreatePlatform`
- `PatchPlatform` when changing a non-edge platform to `EdgeAgent`
- `CreateVaultKvV2SecretProvider`
- `CreateRole` because it creates `RoleType.Custom`
- `CreateUser` when `IsEnabled = true`
- `PatchUser` or `PatchActorEnabled` when enabling a disabled user actor
- OIDC JIT auto-provisioning in `CompleteOidcLogin`

Operations that do not increase usage must remain allowed even when over quota:

- Rename resources.
- Edit existing configuration.
- Disable resources or users.
- Delete resources or users.
- Change an Edge Agent platform to a non-edge connector.
- Replace an expired or invalid license.

## Quota Errors

Quota failures return `403 Forbidden` problem details.

Add a result error type:

```csharp
public sealed record LicenseQuotaViolation(
    LicenseLimit Limit,
    int Current,
    int Requested,
    int Maximum);

public sealed record LicenseQuotaExceededError(
    IReadOnlyList<LicenseQuotaViolation> Violations,
    LicenseStatus LicenseStatus,
    string Edition) : Error;
```

Problem details extension example:

```json
{
  "type": "https://citadel.local/problems/license-quota-exceeded",
  "title": "License quota exceeded",
  "status": 403,
  "detail": "Community edition allows 1 Edge Agent platform.",
  "violations": [
    {
      "limit": "EdgeAgentPlatforms",
      "current": 1,
      "requested": 1,
      "maximum": 1
    }
  ],
  "licenseStatus": "Community",
  "edition": "Community"
}
```

Keep endpoint classes thin. The mapping from `LicenseQuotaExceededError` to problem details belongs in the shared result-to-HTTP mapping layer.

## API Contract

Add a license route group:

```text
/api/v1/license
```

Use POST for install/replace. Do not use PUT.

```http
GET    /api/v1/license
POST   /api/v1/license
DELETE /api/v1/license
GET    /api/v1/license/request
```

Suggested route names:

```text
getLicense
installLicense
removeLicense
getLicenseRequest
```

Authorization:

- Add `ResourceType.License` in `Hosting.Common`.
- Add it to `PermissionMatrix`.
- Grant Admin read, update, and delete capability for licenses.
- Do not grant Operator or Viewer by default.
- Require `ResourceType.License` read for `GET /license` and `GET /license/request`.
- Require `ResourceType.License` write for install and replacement. This maps to the user-facing Update capability.
- Require `ResourceType.License` execute for removal, matching existing Citadel delete-command permission patterns. This maps to the user-facing Delete capability.

Do not use a broad `ResourceType.System` for licensing. It is likely to accumulate unrelated settings later and become too powerful.

### GET `/api/v1/license`

Returns the current license status, effective limits, usage, and safe metadata.

```csharp
public sealed record LicenseView(
    LicenseStatus Status,
    string Edition,
    string InstanceId,
    string? LicenseId,
    string? ReplacedLicenseId,
    string? CustomerId,
    string? CustomerName,
    string? Fingerprint,
    DateTimeOffset? IssuedAt,
    DateTimeOffset? NotBefore,
    DateTimeOffset? ExpiresAt,
    DateTimeOffset? GraceUntil,
    IReadOnlyList<LicenseLimitView> Limits,
    IReadOnlyList<string> Warnings);

public sealed record LicenseLimitView(
    LicenseLimit Limit,
    int Current,
    int Maximum,
    bool OverQuota);
```

Do not return `RawLicense`.

### POST `/api/v1/license`

Installs or replaces the singleton license.

Accept JSON:

```csharp
public sealed record InstallLicenseInput(string License);
```

Optional multipart upload can be added later if the UI needs file upload. Keep the first implementation simple unless there is already a shared upload pattern.

Behavior:

- Trim only surrounding ASCII whitespace from the submitted string.
- Reject input larger than 64 KB.
- Reject whitespace inside the compact JWS.
- Parse and verify the submitted license before writing it.
- Accept `Valid`, `GracePeriod`, and `NotYetValid` licenses only if the signature and instance binding are valid.
- Reject malformed, unknown-key, unsupported-schema, invalid-signature, and instance-mismatched licenses.
- Store the exact compact JWS string only after successful verification.
- Replace the existing license atomically.
- Record `LicenseInstalled` or `LicenseReplaced`.
- Return the same shape as `GET /api/v1/license`.

### DELETE `/api/v1/license`

Removes the installed license and returns Community status.

Behavior:

- Deleting a missing license is idempotent and returns the Community view.
- Record `LicenseRemoved` only when a license existed.

### GET `/api/v1/license/request`

Returns safe data needed to request an offline license:

```csharp
public sealed record LicenseRequestView(
    string Product,
    string InstanceId,
    string CoreVersion,
    DateTimeOffset GeneratedAt);
```

No secrets, host identifiers, IP addresses, platform addresses, user emails, or raw system inventory should be included.

## Activity Events

Add activity event types:

```csharp
LicenseInstalled
LicenseReplaced
LicenseRemoved
LicenseEnteredGracePeriod
LicenseExpired
LicenseValidationFailed
```

Use an activity snapshot that excludes the raw license:

```csharp
public sealed record LicenseActivitySnapshot(
    string? LicenseId,
    string? ReplacedLicenseId,
    string Edition,
    string? CustomerId,
    string? CustomerName,
    string? Fingerprint,
    LicenseStatus Status,
    DateTimeOffset? ExpiresAt,
    DateTimeOffset? GraceUntil);
```

Add an activity resource type for `License`. The activity UI should show readable labels and safe metadata.

For status transitions, add a lightweight monitor that checks the installed license periodically and records transition events only when the persisted validation status changes. Do not emit repeated `Expired` or `GracePeriod` events on every read.

## Frontend

Add a License page under Settings:

```text
/settings/license
```

If the settings shell does not exist yet, add a small settings route/shell and keep the first page focused on License.

The page should show:

- Edition and status.
- Instance ID with copy action.
- Safe license metadata.
- Limits and current usage.
- Over-quota rows highlighted with clear text.
- Paste/install license form.
- Remove license action.
- License request details for support.

Use existing Citadel UI patterns:

- Shared page width.
- Existing button, table, badge, dialog, and toast components.
- No landing-page style hero.
- Generated API client from `src/Citadel.FrontEnd/src/api/generated`.
- Frontend route protected by the same capability/permission model used by other admin pages.

When quota errors are returned from any create or enable flow, show the server-provided problem detail instead of generic failure text.

## Issuer Tool

The license issuer must live outside this public repository, preferably in a private repository.

Private issuer responsibilities:

```text
citadel-license keys generate
citadel-license license issue
citadel-license license inspect
citadel-license license verify
```

Use one key representation everywhere:

- Private key: encrypted PKCS#8 PEM.
- Public key: SubjectPublicKeyInfo PEM.

Generate a master signing key:

```powershell
citadel-license keys generate `
  --key-id citadel-license-2026-01 `
  --private-key ./keys/citadel-license-2026-01.private.pem `
  --public-key ./keys/citadel-license-2026-01.public.pem
```

The key generator must:

- Refuse to overwrite an existing key by default.
- Never print private-key material.
- Print the key ID and public-key fingerprint.
- Read the private-key password interactively or from a protected environment variable.
- Automatically verify that the generated key pair matches.

The matching public key must be added to Citadel Core's embedded license public-key registry before issuing licenses with its `kid`.

Generate a client license:

```powershell
citadel-license license issue `
  --request ./customer-license-request.json `
  --customer-id customer-example `
  --customer-name "Example Corp" `
  --limit oidc-providers=5 `
  --limit edge-agent-platforms=20 `
  --limit secret-providers=10 `
  --limit custom-roles=50 `
  --limit active-users=100 `
  --limit platforms=50 `
  --expires 2027-07-12 `
  --grace-days 14 `
  --key-id citadel-license-2026-01 `
  --private-key ./keys/citadel-license-2026-01.private.pem `
  --out ./licenses/example-2027.citadel-license
```

The issuer must:

- Validate the license request schema.
- Validate `product = citadel`.
- Read the customer's instance ID from the request.
- Generate a unique license ID.
- Validate every known quota key.
- Refuse limits below Community defaults.
- Build and sign the compact JWS with `alg = Ed25519` and `typ = citadel-license+jws`.
- Include `replacedLicenseId` in the signed payload only when the issuer operator passes `--replaces`.
- Reopen and verify the generated file with the public key.
- Calculate its fingerprint.
- Store an internal issuance record.
- Print only safe metadata.

Verify a generated license:

```powershell
citadel-license license verify `
  ./licenses/example-2027.citadel-license `
  --public-key ./keys/citadel-license-2026-01.public.pem
```

Maintain a private issuance registry. It can initially be a private SQLite database, but do not rely on filenames or shell history as the issuance record.

Store privately:

- License ID.
- Customer ID.
- Customer name.
- Instance ID.
- Signing key ID.
- Issued date.
- Expiration.
- Grace end.
- Limits.
- Fingerprint.
- Output filename.
- Replaced license ID.

For rehosting, the client sends a fresh license request from the new Citadel instance. Do not add replacement metadata to the client request. The issuer decides whether the new request is a new sale, renewal, or rehost. When it is a rehost, issue the new license with `--replaces <old-license-id>` and mark the old license as replaced in the private issuance registry.

The public Citadel repository may include:

- Verifier interfaces and implementation.
- Embedded public verification keys.
- Test-only signing keys under test projects.
- Golden signed license fixtures generated from test-only keys.

It must not include production private keys.

## Implementation Plan

1. Add license domain contracts, statuses, limits, payload models, and source-generated JSON metadata.
2. Add `CitadelInstanceIdentity` and `InstalledLicenses` to the EF model and generate the migration/script.
3. Add repositories and unit-of-work properties.
4. Add `ILicenseVerifier`, embedded public key registry, and compact JWS verification.
5. Add `ILicenseStateProvider` with cache invalidation.
6. Add the license usage snapshot query and `ILicenseQuotaService`.
7. Add quota enforcement to create/enable/JIT provisioning handlers.
8. Add license commands and queries.
9. Add API endpoint records, route registration, JSON context entries, and OpenAPI/generated frontend API updates.
10. Add activity events and transition monitor.
11. Add frontend Settings > License page.
12. Add tests across verifier, persistence, handlers, API route shape, and frontend behavior.

## Tests

Add focused tests instead of only snapshot coverage.

Verifier tests:

- Valid Business license verifies.
- Unknown `kid` returns `UnknownSigningKey`.
- Tampered payload returns `Invalid`.
- `alg = none` is rejected.
- `alg = EdDSA` is rejected.
- Any algorithm other than `Ed25519` is rejected.
- Wrong `typ` is rejected.
- Wrong product, issuer, or audience is rejected.
- Unsupported schema returns `UnsupportedSchema`.
- Instance mismatch returns `InstanceMismatch`.
- Not-before future returns `NotYetValid`.
- Expired outside grace returns `Expired`.
- Expired inside grace returns `GracePeriod`.
- Missing limits fall back to Community.
- Signed limits lower than Community do not reduce effective limits.

Repository/integration tests:

- Instance ID is created once and reused.
- Installed license upsert replaces the previous license atomically.
- Raw license is persisted but never returned by `GET /api/v1/license`.
- Usage snapshot counts are database-side and match OIDC, Edge Agent platform, secret provider, custom role, active user, and total platform rows.
- Dapper AOT-friendly parameter usage is preserved.

Handler tests:

- Community blocks a second OIDC provider.
- Community blocks a second Edge Agent platform.
- Community blocks a second external secret provider.
- Community blocks custom role creation.
- Community blocks the eleventh enabled user.
- Creating a disabled user over active-user quota succeeds.
- Enabling a disabled user over active-user quota fails.
- OIDC auto-provisioning over active-user quota fails with a quota error.
- Editing, disabling, and deleting over-quota resources remain allowed.
- Valid Business license raises limits.
- Expired license falls back to Community.
- Grace-period license still applies signed limits.

API tests:

- `GET /api/v1/license` requires authentication and `ResourceType.License` read permission.
- `POST /api/v1/license` requires `ResourceType.License` write permission.
- `DELETE /api/v1/license` requires `ResourceType.License` execute permission.
- `POST /api/v1/license` installs/replaces and returns safe metadata.
- Quota errors map to 403 problem details with quota metadata.
- Route names are stable for client generation.

Frontend tests:

- License page renders Community status.
- License page renders valid Business metadata without raw license.
- Over-quota rows are visible.
- Install success invalidates and reloads license status.
- Install failure shows validation problem details.
- Create/enable flows show quota problem details.

## Acceptance Criteria

- Citadel runs normally with no license.
- Community limits are enforced for quota-increasing operations.
- Existing resources remain untouched when the installation is over quota.
- A valid signed license raises limits offline.
- Invalid, expired, unknown-key, and instance-mismatched licenses do not raise limits.
- Grace-period licenses keep signed limits until `graceUntil`.
- License install, replace, remove, and request endpoints exist under `/api/v1/license`.
- No endpoint returns the raw license.
- Quota failures return 403 problem details with limit/current/maximum metadata.
- License activity events are recorded without raw license data.
- The frontend has a Settings > License page.
- Database changes are produced through EF migration generation.
- Generated frontend API files and OpenAPI snapshots are updated.
- Unit and integration tests cover verifier, quota enforcement, API behavior, and UI behavior.
