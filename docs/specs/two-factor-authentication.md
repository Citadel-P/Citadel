# Citadel - Two-Factor Authentication MVP

## 1. Goal

Add TOTP-based two-factor authentication for Citadel users who authenticate with a local password.

The implementation must fit Citadel's current authentication model and remain intentionally small.

Existing conventions to preserve:

- Public API routes are under `/api/v1`.
- Local login is `POST /api/v1/authentication/login`.
- Refresh uses the existing HttpOnly `Constants.RefreshToken` cookie.
- Auth commands live under `src/Citadel.Application/Features.Identity/Auth`.
- Current-user security controls live under `src/Citadel.Application/Features.Identity/Profile`.
- Admin user controls live under `src/Citadel.Application/Features.Identity/Users`.
- Thin endpoint adapters live under `src/Citadel.WebApi/Routes/Endpoints`.
- Route registration stays in `src/Citadel.WebApi/Routes/PublicEndpoints.cs`.
- Database schema changes start in `src/Citadel.Infrastructure.Migrations/EntityFramework/ApplicationDbContext.cs`, followed by the EF migration and SQL script.
- Repository access goes through `IUnitOfWork` and Dapper repositories under `src/Citadel.Infrastructure/Persistence`.
- Request and response records must be registered in `ApplicationJsonContext`.
- Activity event info records must be registered in `DomainJsonContext` and SignalR `DerivedTypesMapping`.
- Frontend API files are generated from OpenAPI and must not be edited manually.

Supported authenticator applications include Microsoft Authenticator, Google Authenticator, Bitwarden, 1Password, and Authy.

## 2. Scope

### Included

- TOTP enrollment using a QR-code-compatible `otpauth` URI.
- TOTP verification during local password login.
- Ten single-use recovery codes.
- Enable, disable, and recovery-code regeneration from the existing Profile Security area.
- Mandatory enrollment when required by the global MFA policy.
- Administrator MFA reset from user management.
- Refresh-token revocation after MFA security changes.
- Activity events for MFA-sensitive operations.

### Excluded

- SMS or email verification codes.
- Passkeys, WebAuthn, or hardware security keys.
- Remembered or trusted devices.
- Multiple TOTP devices per user.
- Per-user, per-team, or custom-role policy exceptions.
- Step-up authentication for individual operations.
- OIDC `acr` or `amr` validation.
- Access-token authentication-method claims.
- Enrollment grace periods.
- Generic MFA provider or plugin abstractions.

## 3. Core Rules

1. Citadel must not issue an access token or refresh token until required MFA verification succeeds.
2. Citadel TOTP applies only when the user signs in with a local password.
3. OIDC login continues to rely on the identity provider's MFA policy.
4. A user with both a local password and an OIDC identity may configure Citadel TOTP for local-password login.
5. TOTP secrets must use Citadel's existing secret-protection infrastructure.
6. Recovery codes must never be stored in plaintext.
7. MFA challenges, setup sessions, recovery codes, and accepted TOTP time steps must be single-use.
8. MFA policy must be enforced by the backend, not only by the frontend.

## 4. Existing Session Issuing Logic

Citadel currently issues sessions from:

- `LoginCommandHandler` for local users.
- `CompleteOidcLoginHandler` for OIDC users.
- `RefreshTokenCommandHandler` for refresh.

Before adding MFA, extract the local session-issuing behavior into:

```csharp
public interface IAuthenticationSessionIssuer
{
    Task<string> IssueAsync(
        UserAuthInfo user,
        CancellationToken cancellationToken);
}
```

The implementation must:

- Create the access token through `IJwtService.CreateAccessToken`.
- Create the refresh token through `IJwtService.CreateRefreshToken`.
- Persist `RefreshToken.Create(...)`.
- Enforce the existing maximum of ten refresh tokens per user.
- Set the refresh cookie through `IRefreshTokenCookieService`.
- Populate `IRoleCache`.

Both normal local login and successful MFA verification must use this service. Do not duplicate session creation in MFA handlers.

OIDC login may keep its existing flow unless using the new service reduces duplication without changing behavior.

## 5. Login Contract

Replace the current response:

```csharp
public sealed record LoginResponse(string AccessToken);
```

with:

```csharp
public enum LoginNextStep
{
    Completed,
    VerifyMfa,
    EnrollMfa
}

public sealed record LoginResponse(
    string? AccessToken,
    LoginNextStep NextStep);
```

Rules:

- `Completed`: return the access token and set the normal refresh cookie.
- `VerifyMfa`: return no access token and set only the MFA challenge cookie.
- `EnrollMfa`: return no access token and set only the MFA setup cookie.

The frontend must branch on `NextStep` before treating login as authenticated.

## 6. TOTP Configuration

Use `Otp.NET` unless an equivalent dependency already exists.

Configuration:

```text
Algorithm: SHA-1
Digits: 6
Period: 30 seconds
Secret length: 20 random bytes
Encoding: Base32
Verification window: current step plus one adjacent step
```

Use the configured Citadel display or instance name as the issuer when available. Otherwise use `Citadel`.

Example:

```text
otpauth://totp/Citadel%20Production:user@example.com?secret=BASE32SECRET&issuer=Citadel%20Production&algorithm=SHA1&digits=6&period=30
```

`ITotpService` must return the matched TOTP time step when verification succeeds.

```csharp
public interface ITotpService
{
    TotpSetup CreateSetup(string issuer, string accountName);

    bool TryVerify(
        byte[] secret,
        string code,
        out long matchedTimeStep);
}
```

## 7. Secret Protection

Do not introduce a second encryption-key system.

Reuse:

- `SecretsConfiguration`.
- `Secrets:EncryptionKey`.
- Existing generated secret-key fallback.
- `ISecretValueProtector`.
- `SecretValueProtector` AES-GCM implementation.

Persist protected TOTP secrets as base64 strings, following the existing internal-secret and OIDC-client-secret pattern.

If unprotection fails:

- Log the detailed error server-side.
- Return a generic bad-request or unauthorized error.
- Never expose encryption or key details to the client.

## 8. Database Model

Add entities under:

```text
src/Citadel.Domain/Entities/Identity/MfaEntities.cs
```

Add repository interfaces to `IUnitOfWork`:

```csharp
IUserMfaRepository UserMfa { get; }
IMfaChallengeRepository MfaChallenges { get; }
```

Add Dapper implementations under:

```text
src/Citadel.Infrastructure/Persistence/MfaRepository.cs
```

Add EF mappings in `ApplicationDbContext.cs`, then generate the migration and SQL script.

### 8.1 UserMfaSettings

One row means TOTP is enabled for that user.

```text
UserMfaSettings
- UserId uuid primary key references Users(Id) on delete cascade
- ProtectedTotpSecret text not null
- LastAcceptedTimeStep bigint null
- EnabledAt timestamp not null
- CreatedAt timestamp not null
```

Do not add an `MfaEnabled` column to `Users`.

### 8.2 UserMfaRecoveryCodes

```text
UserMfaRecoveryCodes
- Id uuid primary key
- UserId uuid not null references Users(Id) on delete cascade
- CodeHash text not null
- UsedAt timestamp null
- CreatedAt timestamp not null
```

Indexes:

- Index on `UserId`.
- Unique index on `(UserId, CodeHash)`.

Store hashes as base64 strings.

### 8.3 MfaSetupSessions

```text
MfaSetupSessions
- Id uuid primary key
- UserId uuid not null references Users(Id) on delete cascade
- ProtectedTotpSecret text not null
- ExpiresAt timestamp not null
- ConsumedAt timestamp null
- CreatedAt timestamp not null
```

Rules:

- Expire after ten minutes.
- Single-use.
- Starting a new setup deletes all previous setup sessions for that user.
- Expired sessions are deleted opportunistically.
- No cleanup background service is required for the MVP.

### 8.4 MfaChallenges

```text
MfaChallenges
- Id uuid primary key
- UserId uuid not null references Users(Id) on delete cascade
- ExpiresAt timestamp not null
- FailedAttempts int not null default 0
- ConsumedAt timestamp null
- CreatedAt timestamp not null
```

Rules:

- Create only after successful password validation.
- Expire after five minutes.
- Maximum five failed attempts.
- Single-use.
- Delete expired challenges opportunistically.
- No cleanup background service is required for the MVP.

## 9. Recovery Codes

Generate ten codes when MFA is enabled or regenerated.

Format:

```text
ABCD-EFGH-IJKL
```

Rules:

- Generate with `RandomNumberGenerator`.
- Use an alphabet that avoids ambiguous characters where practical.
- Normalize by trimming, removing hyphens and spaces, and converting to uppercase invariant.
- Hash using HMAC-SHA-256.
- Derive a purpose-specific HMAC key from `Secrets:EncryptionKey`; do not use raw SHA-256.
- Compare hashes with `CryptographicOperations.FixedTimeEquals`.
- Store only the hash.
- Display plaintext codes only once.
- Regenerating codes deletes all previous recovery codes.

Suggested service:

```csharp
public interface IRecoveryCodeService
{
    IReadOnlyList<string> Generate(int count);
    string Normalize(string code);
    byte[] Hash(string normalizedCode);
}
```

## 10. Cookie Design

Add constants:

```csharp
Constants.MfaChallenge = "citadel_mfa_challenge";
Constants.MfaSetup = "citadel_mfa_setup";
```

Add cookie services similar to `RefreshTokenCookieService`:

```csharp
public interface IMfaChallengeCookieService
{
    Guid? GetCurrent();
    void Set(Guid challengeId, DateTime expiresAt);
    void Delete();
}

public interface IMfaSetupCookieService
{
    Guid? GetCurrent();
    void Set(Guid setupSessionId, DateTime expiresAt);
    void Delete();
}
```

Cookie options:

- `HttpOnly = true`.
- `Secure = true`.
- `SameSite = Strict`.
- `IsEssential = true`.
- Challenge max age: no more than five minutes.
- Setup max age: no more than ten minutes.

Do not place user IDs, setup-session IDs, or challenge IDs in frontend route state or URLs.

## 11. Endpoint Design

### 11.1 Authentication Endpoints

Under `/api/v1/authentication`:

```text
POST /api/v1/authentication/login
POST /api/v1/authentication/mfa/verify
GET  /api/v1/authentication/mfa/setup
POST /api/v1/authentication/mfa/setup/confirm
```

The three MFA endpoints are anonymous at the ASP.NET authorization layer, but require a valid HttpOnly MFA cookie and use the `strict-auth` rate-limit policy.

Suggested generated endpoint names:

```text
verifyAuthenticationMfa
getAuthenticationMfaSetup
confirmAuthenticationMfaSetup
```

### 11.2 Profile MFA Endpoints

Under `/api/v1/profile/mfa`, requiring normal authentication:

```text
GET  /api/v1/profile/mfa
POST /api/v1/profile/mfa/setup
POST /api/v1/profile/mfa/setup/confirm
POST /api/v1/profile/mfa/disable
POST /api/v1/profile/mfa/recovery-codes
```

Suggested generated endpoint names:

```text
getProfileMfaStatus
startProfileMfaSetup
confirmProfileMfaSetup
disableProfileMfa
regenerateProfileMfaRecoveryCodes
```

### 11.3 Administrator Reset

Under user management:

```text
DELETE /api/v1/users/{id:guid}/mfa
```

Use the existing `User_Update` permission pattern.

Suggested endpoint name:

```text
resetUserMfa
```

## 12. Login Flow

Update `LoginCommandHandler`:

1. Load user auth information using the existing login lookup.
2. Validate the password.
3. Apply existing missing-user and disabled-user behavior.
4. Determine whether the user has a usable local password credential.
5. Evaluate the MFA policy using the user's roles.
6. If enrollment is required and MFA is not enabled:

   - Delete previous setup sessions for the user.
   - Generate a new TOTP secret.
   - Store it as a protected `MfaSetupSession` expiring in ten minutes.
   - Set `citadel_mfa_setup` to the setup-session ID.
   - Return `NextStep = EnrollMfa` without access or refresh tokens.

7. If MFA is enabled:

   - Create an `MfaChallenge` expiring in five minutes.
   - Set `citadel_mfa_challenge` to the challenge ID.
   - Return `NextStep = VerifyMfa` without access or refresh tokens.

8. Otherwise issue the normal session and return `NextStep = Completed`.

Never issue an access or refresh token before required MFA verification or enrollment succeeds.

## 13. Mandatory Enrollment Flow

Mandatory enrollment is used only immediately after a successful password login when policy requires MFA and the user is not enrolled.

### 13.1 Read Setup

```text
GET /api/v1/authentication/mfa/setup
```

Rules:

- Read the setup-session ID from `citadel_mfa_setup`.
- Load the matching unexpired, unconsumed setup session.
- Load the target user from the session.
- Unprotect the secret.
- Return the manual secret, `otpauth` URI, and expiry.
- Do not issue tokens.

Response:

```json
{
  "secret": "BASE32SECRET",
  "otpAuthUri": "otpauth://totp/Citadel:user@example.com?secret=BASE32SECRET&issuer=Citadel",
  "expiresAt": "2026-07-18T10:10:00Z"
}
```

### 13.2 Confirm Mandatory Setup

```text
POST /api/v1/authentication/mfa/setup/confirm
```

Request:

```json
{
  "code": "123456"
}
```

Rules:

1. Read the setup-session ID from the setup cookie.
2. Validate session existence, expiry, and consumption state.
3. Verify the TOTP code against the provisional secret.
4. In one transaction:

   - Mark the setup session consumed.
   - Create `UserMfaSettings` with `LastAcceptedTimeStep` set to the matched step.
   - Delete existing recovery codes.
   - Insert ten recovery-code hashes.

5. Issue the normal access and refresh session through `IAuthenticationSessionIssuer`.
6. Clear the setup cookie.
7. Record `UserMfaEnabled`.
8. Return the access token and plaintext recovery codes once.

Response:

```json
{
  "accessToken": "...",
  "recoveryCodes": [
    "ABCD-EFGH-IJKL"
  ]
}
```

On failure, do not create MFA settings or issue tokens.

## 14. Login MFA Verification

```text
POST /api/v1/authentication/mfa/verify
```

Request:

```json
{
  "code": "123456",
  "recoveryCode": null
}
```

Exactly one of `code` or `recoveryCode` is required.

Rules:

1. Read the challenge ID from `citadel_mfa_challenge`.
2. Validate challenge existence, expiry, consumption state, and failed-attempt count.
3. Load the user's MFA settings.
4. Verify the submitted TOTP or recovery code.
5. Complete consumption atomically:

   - For TOTP, update `LastAcceptedTimeStep` only when the matched step is greater than the stored value, and consume the challenge in the same transaction.
   - For a recovery code, mark the code used and consume the challenge in the same transaction.

6. Issue the normal session through `IAuthenticationSessionIssuer` only after the transaction succeeds.
7. Clear the challenge cookie.
8. Record `UserMfaRecoveryCodeUsed` when applicable.

Invalid verification must atomically increment `FailedAttempts` while the challenge remains valid and unconsumed.

Use the same client-facing error for invalid, expired, consumed, missing, exhausted, replayed, or otherwise unusable challenges:

```text
Invalid or expired verification code.
```

Never return whether TOTP or recovery-code verification failed.

## 15. Optional Profile Enrollment

The user configures TOTP in:

```text
Profile -> Security -> Two-factor authentication
```

Place this section next to, but separate from, Change Password.

### 15.1 Start Optional Setup

```text
POST /api/v1/profile/mfa/setup
```

Request:

```json
{
  "password": "current-password"
}
```

Rules:

- Require an authenticated user with a usable local password credential.
- Having an OIDC identity link does not block setup when a local password exists.
- Validate the current password.
- Reject when MFA is already enabled.
- Delete previous setup sessions for the user.
- Generate and protect a new TOTP secret.
- Create a setup session expiring in ten minutes.
- Return the manual secret, `otpauth` URI, and expiry.

Response:

```json
{
  "secret": "BASE32SECRET",
  "otpAuthUri": "otpauth://totp/Citadel:user@example.com?secret=BASE32SECRET&issuer=Citadel",
  "expiresAt": "2026-07-18T10:10:00Z"
}
```

### 15.2 Confirm Optional Setup

```text
POST /api/v1/profile/mfa/setup/confirm
```

Request:

```json
{
  "code": "123456"
}
```

Rules:

1. Load the current user's active unexpired setup session.
2. Verify the TOTP code.
3. In one transaction:

   - Mark the setup session consumed.
   - Create `UserMfaSettings` with the matched time step.
   - Delete existing recovery codes.
   - Insert ten recovery-code hashes.

4. Revoke all other refresh tokens for the user, keeping the current session when it can be resolved through `ICurrentRefreshSessionResolver`.
5. Record `UserMfaEnabled`.
6. Return plaintext recovery codes once.

Response:

```json
{
  "enabled": true,
  "recoveryCodes": [
    "ABCD-EFGH-IJKL"
  ]
}
```

## 16. MFA Status

```text
GET /api/v1/profile/mfa
```

Response:

```json
{
  "enabled": true,
  "remainingRecoveryCodes": 8,
  "policy": "Optional",
  "canDisable": true
}
```

Rules:

- `enabled` is derived from the existence of `UserMfaSettings`.
- `remainingRecoveryCodes` counts unused recovery codes.
- `canDisable` is false when the active policy requires MFA for the current user.
- The disable handler must independently enforce the same policy.

## 17. Disable MFA

```text
POST /api/v1/profile/mfa/disable
```

Request:

```json
{
  "password": "current-password",
  "code": "123456",
  "recoveryCode": null
}
```

Exactly one of `code` or `recoveryCode` is required.

Rules:

- Require an authenticated user with a usable local password credential.
- Validate the current password.
- Reject when the active MFA policy requires MFA for this user.
- Verify TOTP or an unused recovery code.
- Apply TOTP replay prevention when TOTP is used.
- In one transaction:

  - Delete MFA settings.
  - Delete recovery codes.
  - Delete setup sessions.
  - Delete outstanding challenges.

- Revoke all other refresh tokens when the current session is resolvable; otherwise revoke all user refresh tokens.
- Record `UserMfaDisabled`.

## 18. Regenerate Recovery Codes

```text
POST /api/v1/profile/mfa/recovery-codes
```

Request:

```json
{
  "password": "current-password",
  "code": "123456"
}
```

Rules:

- Require an authenticated local-password user with MFA enabled.
- Validate the current password.
- Require TOTP; a recovery code cannot regenerate recovery codes.
- Apply TOTP replay prevention.
- In one transaction:

  - Delete all existing recovery codes.
  - Insert ten new recovery-code hashes.

- Return plaintext recovery codes once.
- Record `UserMfaRecoveryCodesRegenerated`.

## 19. Administrator Reset

```text
DELETE /api/v1/users/{id:guid}/mfa
```

Rules:

- Require `User_Update` through the existing permission system.
- Do not expose the user's TOTP secret or recovery codes.
- Delete MFA settings, recovery codes, setup sessions, and outstanding challenges.
- Delete all refresh tokens for the target user.
- Record `UserMfaResetByAdministrator` with the acting actor ID and target user ID.
- Do not require the target user's password or TOTP code.

If the policy still requires MFA for the target user, their next local-password login must enter mandatory enrollment.

## 20. Global MFA Policy

Add configuration:

```csharp
public sealed class MfaOptions
{
    public const string SectionName = "Mfa";
    public MfaPolicy Policy { get; set; } = MfaPolicy.Optional;
}

public enum MfaPolicy
{
    Optional,
    RequiredForAdministrators,
    RequiredForAllUsers
}
```

Bind in `WebApiModule.AddIOptionsFromConfiguration` with `ValidateOnStart`.

Configuration:

```text
Mfa__Policy=Optional
```

Default:

```text
Optional
```

Rules:

- `Optional`: local-password users may enable or disable MFA.
- `RequiredForAdministrators`: users with the built-in Admin role must enroll and cannot disable MFA.
- `RequiredForAllUsers`: all users with a local password must enroll and cannot disable MFA.
- The policy is evaluated only during local-password authentication.
- OIDC login does not prompt for Citadel TOTP in the MVP.
- Use roles already present in `UserAuthInfo`; do not issue duplicate role queries.

The admin settings UI must clearly state that OIDC MFA is controlled by the identity provider.

## 21. Application Structure

Suggested structure:

```text
src/Citadel.Application/Features.Identity/Mfa/
  Commands/
    StartMfaSetup.cs
    ConfirmMfaSetup.cs
    ConfirmMandatoryMfaSetup.cs
    VerifyMfaChallenge.cs
    DisableMfa.cs
    RegenerateMfaRecoveryCodes.cs
    ResetUserMfa.cs
  Queries/
    GetMfaStatus.cs
    GetMandatoryMfaSetup.cs
  Models/
    MfaModels.cs
  Services/
    TotpService.cs
    RecoveryCodeService.cs
    MfaPolicyService.cs
    AuthenticationSessionIssuer.cs
```

Keep services narrow:

- `ITotpService`: create secrets, create `otpauth` URIs, and verify TOTP.
- `IRecoveryCodeService`: generate, normalize, hash, and compare recovery codes.
- `IMfaPolicyService`: evaluate the configured policy against local auth capability and roles.
- `IAuthenticationSessionIssuer`: issue the existing access and refresh session.

Do not add a generic MFA method/provider abstraction.

## 22. Repository Operations And Atomicity

Repository methods must support conditional, atomic operations rather than read-then-write checks.

Required behaviors:

### Challenge Failure Increment

Increment only when the challenge is active:

```sql
UPDATE MfaChallenges
SET FailedAttempts = FailedAttempts + 1
WHERE Id = @id
  AND ConsumedAt IS NULL
  AND ExpiresAt > @now
  AND FailedAttempts < 5;
```

### TOTP Acceptance And Challenge Consumption

Within one transaction:

```sql
UPDATE UserMfaSettings
SET LastAcceptedTimeStep = @matchedTimeStep
WHERE UserId = @userId
  AND (
      LastAcceptedTimeStep IS NULL
      OR LastAcceptedTimeStep < @matchedTimeStep
  );
```

Then consume the active challenge conditionally. Both updates must succeed before issuing a session.

### Recovery Code And Challenge Consumption

Within one transaction:

- Mark exactly one matching unused recovery code as used.
- Consume the active challenge conditionally.
- Both operations must succeed before issuing a session.

### Setup Confirmation

Within one transaction:

- Consume the setup session conditionally.
- Create MFA settings.
- Replace recovery codes.

A concurrent second request must fail without issuing a second session or reusing a code.

## 23. Activity Events

Add values near existing user events:

```text
UserMfaEnabled
UserMfaDisabled
UserMfaVerificationFailed
UserMfaRecoveryCodeUsed
UserMfaRecoveryCodesRegenerated
UserMfaResetByAdministrator
```

Add corresponding `ActivityEventInfo` records:

```csharp
public sealed record UserMfaEnabled() : ActivityEventInfo;
public sealed record UserMfaDisabled() : ActivityEventInfo;
public sealed record UserMfaVerificationFailed() : ActivityEventInfo;
public sealed record UserMfaRecoveryCodeUsed() : ActivityEventInfo;
public sealed record UserMfaRecoveryCodesRegenerated() : ActivityEventInfo;
public sealed record UserMfaResetByAdministrator(Guid TargetUserId) : ActivityEventInfo;
```

Register them in:

- `DomainJsonContext`.
- SignalR `DerivedTypesMapping`.

Never record:

- Passwords.
- TOTP secrets.
- TOTP codes.
- Recovery codes.
- Cookie values.
- Decrypted secret material.

Use existing `RequestSessionMetadata` for request context rather than adding MFA-specific IP-address or user-agent fields.

## 24. Rate Limiting

Use the existing `strict-auth` policy for:

```text
POST /api/v1/authentication/login
POST /api/v1/authentication/mfa/verify
GET  /api/v1/authentication/mfa/setup
POST /api/v1/authentication/mfa/setup/confirm
POST /api/v1/profile/mfa/setup
POST /api/v1/profile/mfa/setup/confirm
POST /api/v1/profile/mfa/disable
POST /api/v1/profile/mfa/recovery-codes
```

The per-challenge maximum of five failed attempts remains required and is separate from IP rate limiting.

Do not add a permanent MFA-specific account lockout in the MVP.

## 25. Frontend

Update:

```text
src/Citadel.FrontEnd/src/features/auth/auth-provider.tsx
src/Citadel.FrontEnd/src/features/auth/login.tsx
src/Citadel.FrontEnd/src/features/profile/profile-form.tsx
```

Add focused components under:

```text
src/Citadel.FrontEnd/src/features/auth/mfa/
src/Citadel.FrontEnd/src/features/profile/mfa/
```

### Login Behavior

- `Completed`: apply the access token and enter the application.
- `VerifyMfa`: navigate to the MFA verification screen.
- `EnrollMfa`: navigate to the mandatory setup screen.
- Do not include username, user ID, challenge ID, or setup-session ID in the URL.

### MFA Verification Screen

Include:

- Six-digit authenticator input.
- Verify button.
- Switch to recovery-code input.
- Back-to-login action that clears local UI state.

### Profile Security

Use this layout:

```text
Profile
  Security
    Password
      Change password
    Two-factor authentication
      Status
      Enable
      Remaining recovery-code count
      Regenerate recovery codes
      Disable
```

The TOTP section is next to Change Password but remains a separate card or section.

### Optional Setup Dialog

1. Ask for the current password.
2. Display the QR code and manual secret.
3. Ask for the six-digit code.
4. Confirm setup.
5. Display recovery codes once.

### Mandatory Setup Screen

- Hide normal application navigation.
- Read setup data from `/authentication/mfa/setup`.
- Show QR code, manual secret, code input, and confirmation.
- After confirmation, show recovery codes before entering the application.
- Provide logout/cancel that returns to the login page and allows the setup cookie to expire or explicitly clears it through an existing logout path.

### Recovery Code Display

Include:

- Copy-all button.
- Download-as-text button.
- Confirmation checkbox: `I have saved my recovery codes.`
- Do not allow the dialog to be reopened to reveal the same plaintext codes.

Use regenerated OpenAPI hooks. Do not hand-edit generated API files.

## 26. Validation And Errors

TOTP code:

```text
Exactly six numeric characters
```

Recovery code:

- Case-insensitive.
- Hyphens optional.
- Whitespace ignored.
- Normalized before hashing.

Generic MFA verification error:

```text
Invalid or expired verification code.
```

Do not reveal:

- Whether a user exists.
- Whether MFA is enabled before password validation.
- Whether a recovery code was previously used.
- Whether a challenge is missing, expired, consumed, exhausted, replayed, or invalid.
- Whether TOTP secret unprotection failed.

## 27. Refresh Token Revocation

Revoke refresh tokens when:

- MFA is enabled.
- MFA is disabled.
- MFA is reset by an administrator.
- The user's password changes, preserving existing behavior if already implemented.

For authenticated profile operations:

- Keep the current refresh token only when `ICurrentRefreshSessionResolver` can identify it.
- Otherwise revoke all refresh tokens for that user.

For administrator reset:

- Always revoke all refresh tokens for the target user.

For mandatory enrollment:

- No previous session is issued by that login flow.
- Issue the first normal session only after successful setup confirmation.

Do not add access-token deny lists in the MVP.

## 28. Tests

### Unit Tests

- TOTP secret and `otpauth` URI generation.
- Valid TOTP verification.
- Invalid TOTP rejection.
- Adjacent time-window acceptance.
- TOTP time-step replay rejection.
- Recovery-code generation and normalization.
- Recovery-code HMAC hashing and comparison.
- Used recovery-code rejection.
- MFA policy for local admin, local non-admin, dual local/OIDC user, and OIDC-only user.
- Session issuer creates refresh-token rows, sets cookies, populates role cache, and enforces token cap.

### Integration Tests

1. Local user without MFA logs in normally.
2. Local user with MFA receives `VerifyMfa` and no access token.
3. MFA login does not set the refresh cookie before verification.
4. Valid TOTP completes login.
5. Invalid TOTP increments failed attempts.
6. A challenge expires after five minutes.
7. A challenge cannot be reused.
8. A TOTP time step cannot be reused.
9. A recovery code completes login.
10. A recovery code cannot be reused.
11. Concurrent MFA verification produces only one session.
12. Optional enrollment requires the current password.
13. Enabling MFA revokes other refresh tokens.
14. Disabling MFA is rejected when policy requires it.
15. Disabling MFA removes settings and recovery codes when allowed.
16. Recovery-code regeneration invalidates all old codes.
17. Administrator reset removes MFA and revokes all target-user refresh tokens.
18. Required MFA policy returns `EnrollMfa` for an unenrolled local user.
19. Mandatory enrollment setup is accessible only through a valid setup cookie.
20. Mandatory enrollment issues no session before confirmation.
21. Successful mandatory enrollment issues a session and recovery codes.
22. OIDC login does not invoke Citadel TOTP.
23. A user with both local password and OIDC may configure TOTP for local login.

### Frontend Checks

- Login branches correctly for `Completed`, `VerifyMfa`, and `EnrollMfa`.
- Profile Security shows MFA next to Change Password as a separate section.
- Optional setup follows password -> QR -> code -> recovery codes.
- Mandatory setup hides normal navigation.
- Recovery codes are not shown again after closing.
- Generated API types and hooks are used.

## 29. Acceptance Criteria

The feature is complete when:

- A user with a local password can enable TOTP from Profile -> Security.
- Optional enrollment requires the current password.
- A user with MFA cannot receive access or refresh tokens until MFA verification succeeds.
- Required users complete mandatory enrollment before receiving a session.
- Mandatory enrollment is secured by a short-lived HttpOnly setup cookie.
- Ten recovery codes are displayed once, stored only as hashes, and usable once each.
- TOTP time steps cannot be replayed.
- MFA challenges cannot be consumed twice.
- The backend prevents disabling MFA when policy requires it.
- A user can regenerate recovery codes after password and TOTP verification.
- An administrator can reset another user's MFA.
- OIDC login remains governed by the identity provider.
- Users with both local and OIDC authentication can use Citadel TOTP for local login.
- TOTP secrets use Citadel's existing secret protection.
- Refresh tokens are revoked after MFA security changes.
- MFA operations appear in activity history.
- OpenAPI, generated frontend client types, migrations, SQL scripts, and tests are updated.

## 30. Out Of Scope Reminder

Do not expand this implementation to include:

- Passkeys or WebAuthn.
- SMS or email codes.
- Remembered devices.
- Multiple TOTP credentials.
- Per-resource or per-role MFA rules beyond the three global policies.
- Step-up authentication.
- OIDC assurance-claim validation.
- Authentication-method persistence in refresh tokens.
- Generic MFA providers or plugin systems.
