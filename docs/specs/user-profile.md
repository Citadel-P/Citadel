# Citadel - User Profile Page

## Goal

Add a self-service profile page for the authenticated user at:

```text
/profile
```

The page allows the current user to:

- View their account information.
- Update their display name.
- Manage personal preferences.
- Change their password when supported by their authentication method.
- View and revoke their own Citadel sessions.
- View roles and team memberships as read-only information.

The profile page must not expose administrator-only user-management controls.

## 1. Citadel Alignment

Follow existing Citadel conventions:

- Public API routes are under `/api/v1`.
- Add routes in `src/Citadel.WebApi/Routes/PublicEndpoints.cs`.
- Keep endpoint classes thin under `src/Citadel.WebApi/Routes/Endpoints`.
- Put profile request and response records under `src/Citadel.WebApi/Routes/Endpoints/Resources/Identity/Profile`.
- Add application handlers under `src/Citadel.Application/Features.Identity/Profile`.
- Derive the current user from `IUserContextAccessor`.
- Never accept `UserId` or `ActorId` from the client.
- Profile endpoints require authentication but must not require administrator permissions.
- Register all request models, response models, and enums in `src/Citadel.WebApi/Routes/Endpoints/STJContext/ApplicationJsonContext.cs`.
- Regenerate frontend API files under `src/Citadel.FrontEnd/src/api/generated`.
- Do not edit generated frontend API files manually.
- Add database schema changes through `src/Citadel.Infrastructure.Migrations/EntityFramework/ApplicationDbContext.cs`.
- Generate the EF migration and SQL script.
- Do not hand-write migration scripts.
- Keep repository code compatible with Dapper AOT.
- Pass explicit anonymous objects directly to Dapper calls.
- Keep `HttpContext` access inside the Web API or Infrastructure boundary.

## 2. Current State

Citadel already has:

- JWT access tokens.
- Refresh-token cookies.
- Refresh-token records in the `RefreshTokens` table.
- Local authentication through `LoginCommand`.
- OIDC authentication through `CompleteOidcLogin`.
- `IUserContextAccessor` exposing `UserId`, `ActorId`, roles, and authentication state.
- Administrator user management under `/users`.
- A sidebar profile-menu entry pointing to `/profile`.
- Theme state stored in browser local storage through `LayoutProvider`.

Missing pieces:

- `/profile` currently falls through the generic resource route.
- The sidebar user menu displays hard-coded user information.
- The sidebar displays a hard-coded application version.
- Refresh-token records do not contain enough metadata to display active sessions.
- There is no persisted user-preference model.
- The current user cannot revoke individual sessions or other sessions.
- The current user cannot change their local password from a self-service page.

## 3. Page And Route Structure

Add a dedicated frontend route before generic `:type` resource routes:

```tsx
<Route path="profile" element={<ProfilePage />} />
```

Create:

```text
src/Citadel.FrontEnd/src/features/profile/index.tsx
src/Citadel.FrontEnd/src/features/profile/profile-tab.tsx
src/Citadel.FrontEnd/src/features/profile/preferences-tab.tsx
src/Citadel.FrontEnd/src/features/profile/security-tab.tsx
```

Use three tabs:

```text
Profile
Preferences
Security
```

This is an application page, not a resource form.

It does not need to use `ResourceForm`, but it should reuse shared Citadel components such as:

- `ContentCard`
- Tabs
- Badges
- Selects
- Dialogs
- Confirmation controls
- Form validation
- Query invalidation
- Toast handling

## 4. Sidebar Integration

Update:

```text
src/Citadel.FrontEnd/src/layout/sidebar/sidebar-dropdown.tsx
```

Requirements:

- Replace hard-coded user name and email with the current-profile response.
- Generate initials from the current display name.
- Navigate `Your Profile` to `/profile`.
- Remove or defer the `/settings` item unless that route exists.
- Keep logout behavior through `useAuthContext().logout()`.

Update:

```text
src/Citadel.FrontEnd/src/layout/sidebar/sidebar.tsx
```

Requirements:

- Replace the hard-coded version with backend-provided application metadata.
- Keep existing GitHub-link behavior.
- Display a quiet loading fallback such as `v -`.

The application-info functionality is independent of profile behavior and should be implemented as a separate small endpoint within the same task.

## 5. Profile Tab

### 5.1 Displayed Information

Show:

- Generated initials avatar.
- Display name.
- Email address.
- Authentication method.
- OIDC provider name, when applicable.
- Account creation date.
- Directly assigned roles.
- Team memberships.

For an OIDC-linked account, show:

```text
Some account information is managed by your identity provider and cannot be changed in Citadel.
```

Authentication labels:

```text
Local account
```

or:

```text
Managed by <provider display name>
```

### 5.2 Editable Information

For MVP, allow editing only:

- Display name.

Email changes remain outside this feature.

Role assignment, team assignment, resource access, account status, and permission editing remain on the administrator user page.

### 5.3 Display-Name Rules

The display name is cosmetic and must not be treated as an account identifier.

Validation:

- Required.
- Trim leading and trailing whitespace.
- Reject an empty value after trimming.
- Apply a reasonable maximum length.
- Reuse existing user-name character validation where appropriate.
- Allow multiple users to have the same display name.

Do not add a uniqueness constraint for the display name.

Email, OIDC subject, and existing account identifiers continue to identify the user.

### 5.4 Roles And Teams

The profile response must clearly distinguish:

- Roles assigned directly to the user.
- Team memberships.

Do not present team-inherited roles as directly assigned user roles.

For MVP, return direct roles only.

A complete effective-permission or inherited-role visualization is outside scope.

## 6. Authentication Capabilities

The frontend must not infer password-management behavior from OIDC fields.

The backend profile response must explicitly state whether the user can change their password.

Use:

```csharp
public sealed record CurrentProfileAuthenticationView(
    CurrentProfileAuthenticationType Type,
    string Label,
    bool CanChangePassword,
    Guid? OidcProviderId = null,
    string? OidcProviderName = null);
```

```csharp
public enum CurrentProfileAuthenticationType
{
    Local,
    Oidc
}
```

Rules for MVP:

- A local-only account has `CanChangePassword = true`.
- An account linked to an OIDC provider has `CanChangePassword = false`.
- If an account has both a local password and an OIDC login, treat it as OIDC-managed for the MVP UI.

Authentication ownership rules must be determined by the backend, not duplicated in React.

## 7. Preferences Tab

Persist preferences per user.

### 7.1 Domain Model

Add:

```csharp
public sealed class UserPreferences
{
    public Guid UserId { get; private set; }

    public string TimeZone { get; private set; } = null!;

    public UserDateTimeFormat DateTimeFormat { get; private set; }

    public UserTheme Theme { get; private set; }

    public DateTimeOffset UpdatedAt { get; private set; }
}
```

```csharp
public enum UserDateTimeFormat
{
    System,
    TwentyFourHour,
    TwelveHour
}
```

```csharp
public enum UserTheme
{
    System,
    Light,
    Dark
}
```

### 7.2 Database Schema

Table:

```text
UserPreferences
```

Schema:

- `UserId uuid not null`
- `TimeZone text not null`
- `DateTimeFormat text not null`
- `Theme text not null`
- `UpdatedAt timestamp with time zone not null`

Constraints:

- Primary key on `UserId`.
- Foreign key from `UserId` to `Users.Id`.
- Cascade delete when the user is deleted.

Do not create a preferences row until the user explicitly saves preferences.

### 7.3 Default Behavior

When no persisted row exists, return:

```json
{
  "timeZone": null,
  "dateTimeFormat": "System",
  "theme": "System",
  "isPersisted": false
}
```

The frontend should use the browser timezone as the unsaved default:

```ts
Intl.DateTimeFormat().resolvedOptions().timeZone
```

If browser resolution fails, use:

```text
UTC
```

Do not pass the browser timezone as a query parameter to the preferences endpoint.

### 7.4 Time Zone

Store an IANA timezone identifier:

```text
Europe/Paris
America/New_York
UTC
```

Validate saved values on the server.

Do not accept arbitrary strings.

The frontend should reuse or extract the existing timezone selector currently defined in:

```text
src/Citadel.FrontEnd/src/features/alerters/alert-rules/form/form.tsx
```

Do not create a second hard-coded timezone list.

The stored timezone affects presentation only.

All persisted application timestamps remain UTC.

Apply the preference initially to:

- Profile-visible timestamps.
- Session timestamps.

Shared date helpers can then adopt it for:

- Activities.
- Alerts.
- Automation runs.
- Deployment history.
- Stack history.

### 7.5 Date And Time Format

Supported values:

```text
System
TwentyFourHour
TwelveHour
```

Behavior:

- `System` uses the browser locale.
- `TwentyFourHour` forces a 24-hour clock.
- `TwelveHour` forces a 12-hour clock.

### 7.6 Theme

Supported values:

```text
System
Light
Dark
```

Current theme mode is stored through `LayoutProvider` in browser local storage.

Behavior:

1. Apply local storage as the initial fallback.
2. Load the persisted server preference.
3. Apply the server preference after the query resolves.
4. Update the UI immediately when the user changes the selection.
5. Persist the selection when the user saves.

For `System`:

- Use `prefers-color-scheme`.
- Update the theme when the operating-system preference changes.

Existing theme-color handling remains local-only.

Do not add additional appearance preferences in this feature.

## 8. Security Tab

The Security tab adapts to `Authentication.CanChangePassword`.

### 8.1 Local-Only Accounts

Show:

- Change-password form.
- Active sessions.
- Sign out all other sessions.

### 8.2 OIDC-Managed Accounts

Do not show the change-password form.

Show:

```text
Your password is managed by your identity provider.
```

Still show:

- Active Citadel sessions.
- Individual session revocation.
- Sign out all other sessions.

Revoking a Citadel session does not revoke the upstream OIDC-provider session.

## 9. Active Sessions

Reuse `RefreshToken` as the Citadel session model.

Do not add a parallel session table.

### 9.1 Refresh-Token Metadata

Extend `RefreshToken` with:

```csharp
public sealed class RefreshToken
{
    // Existing fields

    public DateTimeOffset LastSeenAt { get; private set; }

    public DateTimeOffset ExpiresAt { get; private set; }

    public string? UserAgent { get; private set; }

    public string? IpAddress { get; private set; }
}
```

If refresh-token expiry is already persisted under another field, reuse the existing field instead of adding `ExpiresAt`.

Database columns:

- `LastSeenAt timestamp with time zone not null`
- `ExpiresAt timestamp with time zone not null`, when not already present
- `UserAgent text null`
- `IpAddress text null`

Do not persist separate `Browser` and `OperatingSystem` fields for MVP.

Browser and operating-system labels should be derived from `UserAgent` when building the session response.

Parsing must be best effort.

A parsing failure must never block:

- Login.
- OIDC completion.
- Refresh.
- Session listing.

Fallback label:

```text
Unknown browser
```

### 9.2 Session Summary

Return:

```csharp
public sealed record UserSessionSummary(
    Guid Id,
    string DisplayName,
    string? UserAgent,
    string? IpAddress,
    DateTimeOffset CreatedAt,
    DateTimeOffset LastSeenAt,
    DateTimeOffset ExpiresAt,
    bool IsCurrent);
```

Example display name:

```text
Chrome on Windows
```

The refresh-token row ID may be exposed as the session-management identifier.

Never expose:

- Raw refresh-token values.
- Refresh-token hashes.
- Cookies.
- OIDC access tokens.
- OIDC refresh tokens.

### 9.3 Active-Session Definition

Only return refresh-token records that are still valid for use.

Exclude:

- Expired refresh tokens.
- Revoked refresh tokens.
- Deleted refresh tokens.
- Tokens rejected by the existing refresh-token validation rules.

If revocation is represented by deletion, only non-expired rows need to be queried.

The session list must not display stale or unusable sessions.

### 9.4 Metadata Capture

Capture request metadata when refresh tokens are created by:

- `LoginCommand`
- `CompleteOidcLogin`

Update `LastSeenAt`, `UserAgent`, and `IpAddress` after the refresh endpoint successfully validates and rotates or refreshes the token.

Use:

```csharp
HttpContext.Connection.RemoteIpAddress
```

after ASP.NET forwarded-header middleware has applied trusted-proxy configuration.

Do not parse or trust `X-Forwarded-For` manually.

Only use forwarded client IP information through the existing ASP.NET `ForwardedHeaders` and trusted-proxy configuration.

### 9.5 Application-Layer Boundary

Do not read `HttpContext` directly from the Application project.

Add an application abstraction:

```csharp
public interface IRequestSessionMetadataAccessor
{
    RequestSessionMetadata GetCurrent();
}
```

```csharp
public sealed record RequestSessionMetadata(
    string? UserAgent,
    string? IpAddress);
```

Define the interface under the Application layer.

Implement it in Web API using `IHttpContextAccessor`.

Login, OIDC completion, and refresh handlers may depend on the abstraction.

### 9.6 Current-Session Resolution

Do not assume the refresh-token cookie contains `RefreshTokens.Id`.

Resolve the current session through the existing refresh-token validation mechanism:

1. Read the refresh-token cookie.
2. Validate or hash the token using the existing authentication service.
3. Resolve the matching refresh-token database row.
4. Use the matched row's `Id` as the current session ID.

If the access token is valid but no refresh cookie is available:

- Return sessions with `IsCurrent = false`.
- Disable `Sign out all other sessions`.
- Do not guess the current session.

### 9.7 Session Revocation

Individual revoke:

- Delete the session only when it belongs to the current user.
- Prevent deletion of the current session.
- Return `404` when the session does not exist, belongs to another user, or is the current session.
- Do not perform a separate ownership check followed by an unrestricted delete.

Use one atomic repository operation:

```csharp
Task<int> DeleteOwnedSessionAsync(
    Guid sessionId,
    Guid userId,
    Guid? currentSessionId,
    CancellationToken cancellationToken);
```

The delete query must include:

- Session ID.
- Current user ID.
- Current-session exclusion.

Revoke all other sessions:

- Require a valid resolved current refresh-token session.
- Delete every refresh token for the current user except the current session.
- Return a validation error when the current session cannot be resolved.

Normal sign out continues to use:

```http
POST /api/v1/authentication/logout
```

Access tokens remain valid until their normal expiration unless Citadel already supports immediate access-token revocation.

## 10. Password Changes

### 10.1 Form

For users with `CanChangePassword = true`, show:

```text
Current password
New password
Confirm new password
```

The confirmation field is frontend-only and must match the new password before submission.

### 10.2 Backend Behavior

The password-change command must:

1. Resolve the current user from `IUserContextAccessor`.
2. Confirm that the current authentication configuration allows password changes.
3. Verify the current password through the existing user/password-hashing implementation.
4. Validate the new password through the centralized password-policy validator.
5. Update the password hash.
6. Revoke other refresh-token sessions.
7. Record a safe activity event.

Do not duplicate password length or complexity constants in this feature.

Reuse the existing centralized password policy.

### 10.3 Current-Session Behavior

When a valid current refresh-token session can be resolved:

- Keep the current refresh-token session.
- Revoke all other refresh-token sessions.

When no current refresh-token session can be resolved:

- Revoke all refresh tokens for the user.
- Allow the current access token to remain valid until normal expiration.

Password update and session revocation should occur in the same transaction where practical.

Never log:

- Current password.
- New password.
- Password hashes.
- Request-body password fields.

## 11. API Contract

Add a profile route group:

```csharp
var profile = group
    .MapGroup("/profile")
    .WithTags("Profile")
    .RequireAuthorization();
```

Routes:

```http
GET    /api/v1/profile
PATCH  /api/v1/profile
GET    /api/v1/profile/preferences
PATCH  /api/v1/profile/preferences
POST   /api/v1/profile/change-password
GET    /api/v1/profile/sessions
DELETE /api/v1/profile/sessions/{sessionId:guid}
DELETE /api/v1/profile/sessions
```

Stable OpenAPI operation names:

```text
getCurrentProfile
updateCurrentProfile
getProfilePreferences
patchProfilePreferences
changeCurrentPassword
listProfileSessions
revokeProfileSession
revokeOtherProfileSessions
```

## 12. Application Information API

Add:

```http
GET /api/v1/application/info
```

Operation name:

```text
getApplicationInfo
```

Response:

```csharp
public sealed record ApplicationInfoView(
    string Name,
    string Version,
    string InformationalVersion);
```

Behavior:

- `Name` is `Citadel`.
- `Version` is a display-safe semantic version.
- `InformationalVersion` exposes the complete generated assembly informational version.
- Source version data from Nerdbank-generated assembly metadata.
- Do not use frontend constants.
- The endpoint may require authentication because it is currently used only in the authenticated layout.
- Register `ApplicationInfoView` in `ApplicationJsonContext`.
- Add this endpoint as a normal `/api/v1/application` route, not under `/profile`.

Example display-safe version:

```text
1.0.5-preview-gabc123
```

## 13. Get Current Profile

```http
GET /api/v1/profile
```

Response:

```csharp
public sealed record CurrentProfileView(
    Guid Id,
    string DisplayName,
    string Email,
    CurrentProfileAuthenticationView Authentication,
    DateTimeOffset CreatedAt,
    IReadOnlyCollection<ResourceInfo> DirectRoles,
    IReadOnlyCollection<ResourceInfo> Teams);
```

The handler must:

- Resolve the user from `IUserContextAccessor.Current`.
- Fetch user, direct roles, teams, and OIDC linkage in as few database roundtrips as practical.
- Avoid per-role and per-team queries.
- Return only read-only role and team information.

## 14. Update Current Profile

```http
PATCH /api/v1/profile
```

Request:

```csharp
public sealed record UpdateCurrentProfileInput(
    string DisplayName);
```

Validation:

- Required.
- Trim whitespace.
- Reject empty value after trimming.
- Apply the existing reasonable name-length rules.
- Allow duplicate display names.
- Do not accept a user ID.

Behavior:

- Update `Users.Name`.
- Use optimistic concurrency if the User entity already supports row versions.
- Return the updated `CurrentProfileView`.
- Record an activity event only when the value changed.

Frontend behavior after success:

- Invalidate the profile query.
- Update the sidebar immediately.
- Show a success toast.

## 15. Get Preferences

```http
GET /api/v1/profile/preferences
```

Response:

```csharp
public sealed record UserPreferencesView(
    string? TimeZone,
    UserDateTimeFormat DateTimeFormat,
    UserTheme Theme,
    bool IsPersisted);
```

When no row exists:

- Return default enum values.
- Return `TimeZone = null`.
- Return `IsPersisted = false`.
- Do not create a database row.

## 16. Patch Preferences

```http
PATCH /api/v1/profile/preferences
```

Request:

```csharp
public sealed record PatchUserPreferencesInput(
    string? TimeZone,
    UserDateTimeFormat? DateTimeFormat,
    UserTheme? Theme);

public sealed class PatchUserPreferencesInputPatchDocument
    : JsonMergePatchDocument<PatchUserPreferencesInput>;
```

Behavior:

- Resolve the current user from the authenticated context.
- Use JSON Merge Patch semantics, consistent with other Citadel update endpoints.
- Distinguish omitted properties from explicit `null` in the endpoint layer.
- Validate the IANA timezone when `TimeZone` is provided.
- Reject explicit `null` for `TimeZone`, `DateTimeFormat`, and `Theme`; these are persisted settings and are not nullable after save.
- Upsert one row by `UserId` when at least one valid preference field is supplied.
- Set `UpdatedAt` using the current UTC time.
- Record only changed fields in activity metadata.
- Do not create an activity event when nothing changed.
- Return the persisted `UserPreferencesView`.

## 17. Change Password

```http
POST /api/v1/profile/change-password
```

Request:

```csharp
public sealed record ChangeCurrentPasswordInput(
    string CurrentPassword,
    string NewPassword);
```

Rules:

- Only accounts with `CanChangePassword = true` may use the endpoint.
- Verify the current password using the existing user method or password service.
- Validate the new password through the centralized password policy.
- Revoke all other sessions when the current session is known.
- Revoke all sessions when the current session cannot be resolved.
- Never log or return password values.

Return a validation error for accounts whose password is externally managed.

## 18. List Sessions

```http
GET /api/v1/profile/sessions
```

Response:

```csharp
public sealed record UserSessionsView(
    IReadOnlyCollection<UserSessionSummary> Sessions,
    bool CanRevokeOtherSessions);
```

Behavior:

- Return only active sessions for the authenticated user.
- Resolve the current session through existing refresh-token validation.
- Set `CanRevokeOtherSessions = false` when no current refresh session is available.
- Order sessions with the current session first, then by `LastSeenAt` descending.

## 19. Revoke One Session

```http
DELETE /api/v1/profile/sessions/{sessionId:guid}
```

Rules:

- Operate only on sessions owned by the current user.
- Prevent revocation of the current session.
- Use one atomic delete query.
- Return `404` when no row is affected.
- Record `UserSessionRevoked` only after successful deletion.

## 20. Revoke Other Sessions

```http
DELETE /api/v1/profile/sessions
```

Rules:

- Require a valid current refresh-token session.
- Delete every active refresh token for the user except the current token.
- Return a validation error when the current token cannot be resolved.
- Return the number of revoked sessions.
- Record an activity event only when at least one session was revoked.

## 21. Backend Implementation Plan

Application files:

```text
src/Citadel.Application/Features.Identity/Profile/Queries/GetCurrentProfile.cs
src/Citadel.Application/Features.Identity/Profile/Commands/UpdateCurrentProfile.cs
src/Citadel.Application/Features.Identity/Profile/Queries/GetUserPreferences.cs
src/Citadel.Application/Features.Identity/Profile/Commands/PatchUserPreferences.cs
src/Citadel.Application/Features.Identity/Profile/Commands/ChangeCurrentPassword.cs
src/Citadel.Application/Features.Identity/Profile/Queries/ListUserSessions.cs
src/Citadel.Application/Features.Identity/Profile/Commands/RevokeUserSession.cs
src/Citadel.Application/Features.Identity/Profile/Commands/RevokeOtherUserSessions.cs
```

Endpoint files:

```text
src/Citadel.WebApi/Routes/Endpoints/ApplicationInfo.cs
src/Citadel.WebApi/Routes/Endpoints/Profile.cs
src/Citadel.WebApi/Routes/Endpoints/Resources/ApplicationInfoView.cs
src/Citadel.WebApi/Routes/Endpoints/Resources/Identity/Profile/*.cs
```

Domain and persistence:

- Add `UserPreferences`.
- Add `UserDateTimeFormat`.
- Add `UserTheme`.
- Add `IUserPreferencesRepository`.
- Extend `RefreshToken` with session metadata and expiry when required.
- Extend `IRefreshTokenRepository` with session operations.
- Add the request-session metadata abstraction.
- Implement the metadata accessor in Web API.
- Add EF configuration and generated migrations.
- Add Dapper repository methods using explicit anonymous-object parameters.

Suggested refresh-token repository methods:

```csharp
Task<int> AddAsync(
    RefreshToken refreshToken,
    CancellationToken cancellationToken);

Task<int> TouchAsync(
    Guid id,
    DateTimeOffset lastSeenAt,
    string? userAgent,
    string? ipAddress,
    CancellationToken cancellationToken);

Task<IReadOnlyList<UserSessionSummary>> GetActiveSessionsAsync(
    Guid userId,
    Guid? currentSessionId,
    DateTimeOffset now,
    CancellationToken cancellationToken);

Task<int> DeleteOwnedSessionAsync(
    Guid sessionId,
    Guid userId,
    Guid? currentSessionId,
    CancellationToken cancellationToken);

Task<int> DeleteOtherTokensAsync(
    Guid userId,
    Guid keepTokenId,
    CancellationToken cancellationToken);

Task<int> DeleteAllTokensAsync(
    Guid userId,
    CancellationToken cancellationToken);
```

Do not add a separate `BelongsToUserAsync` check before deletion.

## 22. Activity

Add `User` to `ActivityResourceType` if it is not already supported.

Add activity events:

```text
UserProfileUpdated
UserPreferencesUpdated
UserPasswordChanged
UserSessionRevoked
UserOtherSessionsRevoked
```

Safe activity-info records:

```csharp
public sealed record UserProfileUpdated(
    IReadOnlyCollection<ActivityChangedField> Changes)
    : ActivityEventInfo;

public sealed record UserPreferencesUpdated(
    IReadOnlyCollection<ActivityChangedField> Changes)
    : ActivityEventInfo;

public sealed record UserPasswordChanged()
    : ActivityEventInfo;

public sealed record UserSessionRevoked(
    Guid SessionId)
    : ActivityEventInfo;

public sealed record UserOtherSessionsRevoked(
    int Count)
    : ActivityEventInfo;
```

The session ID is the refresh-token database row ID, never the token value.

Never log:

- Passwords.
- Password hashes.
- Token strings.
- Token hashes.
- Cookies.
- OIDC tokens.
- Full request bodies containing credentials.

Update serialization and frontend activity rendering where required:

```text
src/Citadel.Domain/Enums.cs
src/Citadel.Domain/Entities/Activities/ActivityEvent.cs
src/Citadel.Domain/Entities/Activities/ActivityEventInfo.cs
src/Citadel.Domain/DomainJsonContext.cs
src/Citadel.WebApi/Hubs/SignalRSerializeContext.cs
src/Citadel.WebApi/Routes/Endpoints/STJContext/ApplicationJsonContext.cs
src/Citadel.FrontEnd/src/features/activities
```

## 23. Frontend Implementation Plan

Create:

```text
src/Citadel.FrontEnd/src/features/profile
```

Use generated client hooks:

```ts
useRead('getApplicationInfo')
useRead('getCurrentProfile')
useMutate('updateCurrentProfile')
useRead('getProfilePreferences')
useMutate('patchProfilePreferences')
useMutate('changeCurrentPassword')
useRead('listProfileSessions')
useMutate('revokeProfileSession')
useMutate('revokeOtherProfileSessions')
```

### Profile Tab

Include:

- Compact identity header.
- Initials avatar.
- Editable display-name field.
- Read-only email.
- Read-only authentication information.
- Direct roles.
- Team memberships.

### Preferences Tab

Include:

- Shared searchable timezone selector.
- Date/time-format selector.
- Theme selector.
- Immediate local theme preview.
- Explicit Save action.
- Local-storage fallback before server preference loads.

### Security Tab

For password-capable accounts:

- Change-password form.
- Active-session list.

For externally managed accounts:

- Managed-password message.
- Active-session list.

Session controls:

- Mark current session clearly.
- Disable current-session revocation.
- Require confirmation before revoking another session.
- Require confirmation before signing out all other sessions.
- Disable sign-out-other-sessions when `CanRevokeOtherSessions` is false.
- Refresh the sessions query after successful revocation.

### Auth And Layout Integration

Add a shared `useCurrentProfile` hook or expose the profile query through the authentication context.

Invalidate or reset profile-dependent queries after:

- Login.
- OIDC login completion.
- Token refresh when identity changes.
- Profile update.
- Logout.

The sidebar must render profile data instead of hard-coded values.

The sidebar version must render backend application information instead of a frontend constant.

## 24. Security Requirements

- Never return password hashes.
- Never expose refresh-token strings.
- Never expose OIDC tokens.
- Never log password request bodies.
- Validate session ownership inside the delete operation.
- Return `404` for sessions belonging to another user.
- Prevent individual deletion of the current session.
- Resolve the current session through existing refresh-token validation.
- Do not assume the cookie contains a database row ID.
- Keep `HttpContext` outside the Application layer.
- Keep all persisted timestamps in UTC.
- Use `DateTimeOffset` for new operational timestamps.
- Use PostgreSQL `timestamp with time zone` following existing Citadel conventions.
- Treat user-agent and IP values as untrusted display data.
- Escape or render user-agent text as plain text.
- Do not expose administrator controls on `/profile`.
- Do not allow profile endpoints to accept `UserId` or `ActorId`.
- Ensure user-agent parsing never blocks authentication.
- Use configured forwarded-header middleware rather than manually trusting proxy headers.

## 25. Out Of Scope

Do not implement:

- Avatar uploads.
- Email changes.
- API keys.
- Personal access tokens.
- Notification preferences.
- Language selection.
- Complete effective-permission visualization.
- Inherited-role visualization.
- MFA management.
- OIDC account linking.
- OIDC account unlinking.
- Administrator user management.
- Upstream OIDC-session revocation.
- Precise geographic session location.
- Additional appearance preferences.

## 26. Recommended Implementation Order

Implement in this order:

1. Add the dedicated `/profile` frontend route.
2. Add current-profile read and display-name update APIs.
3. Replace hard-coded sidebar user information.
4. Add user-preferences persistence and APIs.
5. Integrate timezone, date format, and theme preferences.
6. Extend refresh-token session metadata.
7. Add active-session listing.
8. Add atomic individual-session revocation.
9. Add revoke-other-sessions behavior.
10. Add password-change behavior.
11. Add the application-information endpoint.
12. Add activity events.
13. Regenerate the OpenAPI frontend client.
14. Add backend and frontend tests.

## 27. Tests

### Backend Tests

Cover:

- Authenticated user can read their profile.
- Profile endpoints do not accept another user ID.
- Display name can be updated.
- Duplicate display names are allowed.
- Empty display name is rejected.
- OIDC linkage is reflected in authentication information.
- `CanChangePassword` is correct for local-only accounts.
- `CanChangePassword` is false for OIDC-linked accounts.
- Direct roles are returned.
- Team memberships are returned.
- Preferences return defaults without creating a row.
- Browser timezone is not required by the GET endpoint.
- Valid timezone can be saved.
- Invalid timezone is rejected.
- Preference upsert works.
- No activity is recorded when preferences do not change.
- Local-only user can change password.
- Wrong current password is rejected.
- OIDC-linked user cannot change password.
- Password validation uses the centralized password policy.
- Password change keeps the current refresh session when it can be resolved.
- Password change revokes all refresh sessions when no current session can be resolved.
- Session metadata is captured on local login.
- Session metadata is captured on OIDC login.
- Successful refresh updates `LastSeenAt`.
- Session listing includes only active, non-expired sessions.
- Session listing returns only the authenticated user's sessions.
- Current session is resolved through refresh-token validation.
- User can revoke another owned session.
- User cannot revoke another user's session.
- Current session cannot be individually revoked.
- Individual revocation is performed atomically.
- Revoke-all requires a valid current refresh session.
- Revoke-all keeps the current session.
- Access tokens remain valid until normal expiration.
- Activity events contain no sensitive values.

### Endpoint Tests

Cover:

- Application-info endpoint returns assembly metadata.
- Profile routes are under `/api/v1/profile`.
- Application info is not under `/profile`.
- Unauthenticated requests use existing authentication behavior.
- OpenAPI operation names remain stable.
- New request and response records are registered in STJ source generation.

### Frontend Tests

Where existing frontend testing supports them, cover:

- `/profile` renders instead of falling through to generic resource routing.
- Sidebar profile menu uses current profile data.
- Sidebar version uses backend application information.
- Display-name save invalidates and refreshes profile state.
- Duplicate display names are not blocked by client validation.
- OIDC-managed message appears for OIDC-linked users.
- Password form visibility uses `CanChangePassword`.
- Browser timezone is suggested when preferences are not persisted.
- Theme preview applies before save.
- Persisted theme survives reload.
- Current session is visually identified.
- Current session cannot be revoked.
- Revoke-session action requires confirmation.
- Sign-out-other-sessions is disabled without a current refresh session.
- Revoked sessions disappear after query refresh.

## 28. Acceptance Criteria

The feature is complete when:

1. Authenticated users can open `/profile`.
2. `/profile` does not require administrator permissions.
3. Users can update only their own display name.
4. Duplicate display names are allowed.
5. Direct roles and teams are visible but read-only.
6. Users can save timezone, date/time-format, and theme preferences.
7. Local-only users can change their password.
8. OIDC-linked users see that password management is external.
9. Users can view their active Citadel sessions.
10. Expired or invalid refresh-token rows are not shown as active sessions.
11. Users can revoke another one of their own sessions.
12. Users cannot revoke another user's session.
13. The current session cannot be individually revoked.
14. Users can revoke every other session while keeping the current session.
15. Session revocation invalidates the corresponding refresh token immediately.
16. The current session is resolved through refresh-token validation rather than cookie-ID assumptions.
17. Profile operations cannot modify another user.
18. Administrator user-management functionality remains outside `/profile`.
19. OpenAPI and generated frontend API files are updated.
20. Sidebar user information comes from the current-profile endpoint.
21. Sidebar application version comes from backend assembly metadata.
22. Backend tests cover profile, preferences, password, sessions, security, and activity behavior.
