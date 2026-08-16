---
title: "OIDC providers"
description: "Configure OpenID Connect providers for Citadel sign-in."
---

OIDC providers let users sign in to Citadel with an external identity provider such as Keycloak, Auth0, Entra ID, Google Workspace, or another OpenID Connect provider.

Citadel uses the provider only to verify who the user is. After login, Citadel still manages:

- sessions
- roles
- teams
- resource access
- permissions
- audit and activity history

Local Citadel login remains available, including the local admin account.

## When To Use OIDC

Use OIDC when your organization already manages users in a central identity provider and you want users to sign in with that account.

Good examples:

- company SSO through Entra ID
- self-hosted SSO through Keycloak
- workspace login through Google Workspace
- customer or team login through Auth0

Do not use OIDC as a replacement for Citadel permissions. The provider confirms identity; Citadel still decides what the user can see or change.

## Before You Start

Create an application/client in your OIDC provider.

Use Authorization Code Flow with PKCE.

Configure the redirect URI shown by Citadel on the provider form. It will look like:

```text
https://citadel.example.com/api/v1/authentication/oidc/<provider-id>/callback
```

The redirect URI must match exactly in the provider.

Recommended provider settings:

- Flow: Authorization Code
- PKCE: enabled
- Scopes: `openid profile email`
- ID token: enabled
- Refresh tokens: not required for Citadel

Citadel does not store provider access tokens or provider refresh tokens.

## Create A Provider

Open:

```text
Settings -> OIDC Providers
```

Create a provider with:

- `Display name`: button label shown on the login page, such as `Company SSO`
- `Name`: stable internal name, such as `company-sso`
- `Issuer URL`: provider issuer, such as `https://sso.example.com/realms/company`
- `Client ID`: client/application id from the provider
- `Client secret`: client secret, when the provider requires one
- `Scopes`: usually `openid profile email`
- `Enabled`: whether users can see and use this provider on the login page

Use `Test discovery` before enabling the provider. Citadel checks the issuer metadata and verifies that the provider exposes the required authorization, token, and JWKS endpoints.

Client secrets are encrypted at rest and are never shown again after saving.

The issuer URL must be reachable from the Citadel server, and the authorization URL returned by discovery must be reachable from users' browsers.

## Login Buttons

Enabled providers appear on the Citadel login page as buttons:

```text
Continue with Company SSO
Continue with Keycloak
```

When a user clicks a provider:

1. Citadel redirects them to the provider.
2. The provider authenticates the user.
3. The provider redirects back to Citadel.
4. Citadel validates the provider response.
5. Citadel creates its normal session.
6. The user lands in the Citadel app.

The local username/password form remains available.

## User Matching

Citadel identifies OIDC users by provider plus the provider's stable subject claim.

Citadel does not use email as the permanent identity. Email can change, but the provider subject should stay stable.

When a user signs in, Citadel checks:

```text
Provider + subject
```

If that external identity is already linked, Citadel signs in the linked local user.

## Auto-Provision Users

Auto-provisioning lets Citadel create a local user the first time an approved OIDC user signs in.

When disabled, unknown OIDC users are rejected unless email auto-linking is enabled and succeeds.

The `Default Role` picker only applies when auto-provisioning is enabled. When auto-provisioning is off, Citadel does not create new users, so there is no new user to receive that role.

Use auto-provisioning when your identity provider is already trusted to control who can access Citadel.

Recommended safe setup:

- Enable `Require verified email`
- Configure `Allowed email domains`
- Configure a `Required claim` when your provider supports groups or roles
- Assign a low-privilege default role, such as Viewer

Do not auto-provision users directly as admins.

## Email Auto-Link

Email auto-link connects a new OIDC identity to an existing Citadel user when the email address matches.

This is useful when users already exist in Citadel and you are migrating them to SSO.

For safety:

- Email auto-link is disabled by default.
- Only enable it intentionally.
- Keep `Require verified email` enabled.
- Citadel should only link when the matching local email is unique.

Never auto-link users by unverified email.

## Verified Email

When `Require verified email` is enabled, Citadel requires the provider to send:

```text
email_verified = true
```

This affects:

- auto-provisioning
- email auto-linking

Keep this enabled unless your provider does not support the claim and you have another strong gate, such as a required group claim.

## Allowed Email Domains

Allowed domains restrict who can be provisioned or linked.

Example:

```text
company.com
engineering.company.com
```

With this setting, a user with `alice@company.com` can pass the domain gate. A user with `alice@gmail.com` cannot.

Use domains as a broad safety check. For tighter control, also use a required claim.

## Required Claims

A required claim lets you restrict login to users with a provider claim value.

Example:

```text
Claim name: groups
Required values: citadel-users
```

Only users whose `groups` claim contains `citadel-users` pass the gate.

For Keycloak, common claim names include:

```text
groups
realm_access.roles
resource_access.<client-id>.roles
```

For Entra ID, common claim names include:

```text
groups
roles
```

Provider claim formats vary. Use your provider's token preview/debug tools to confirm the exact claim name and value.

## Default Role

The default role is assigned when Citadel auto-provisions a new user.

Use a low-privilege role first, such as:

```text
Viewer
```

After the user exists, an admin can assign more roles, teams, or resource-specific access from the Access page.

The default role does not automatically update existing users every time they log in.

## Teams And Roles

In the MVP, Citadel remains the source of truth for role and team membership.

Admins manage access in:

```text
Settings -> Access
```

Future versions may support claim-based role and team mapping. Until then, use OIDC for login and Citadel Access for authorization.

## Disable A Provider

Disabling a provider hides it from the login page and prevents new logins through that provider.

Existing Citadel sessions are not automatically revoked when a provider is disabled. Users with active Citadel sessions may remain signed in until their Citadel session expires or they log out.

Use disable when:

- testing provider configuration
- temporarily blocking SSO login
- replacing a provider

Use delete only when the provider and its linked identities should be removed from Citadel.

## Delete A Provider

Deleting a provider removes its configuration and external identity links.

Local Citadel users are not deleted.

After deletion, users can no longer sign in through that provider unless it is recreated and their identities are linked again.

Prefer disabling first if you are not sure.

## Rotate A Client Secret

Rotate the client secret in both places:

1. Create or rotate the secret in the OIDC provider.
2. Update the client secret in Citadel.
3. Save the provider.
4. Run `Test discovery`.
5. Test login with a non-admin account.

Citadel does not show the existing secret. Leaving the secret field empty when editing should keep the current secret.

## Troubleshooting

### Provider Does Not Appear On Login Page

Check:

- provider is enabled
- discovery test succeeds
- Citadel frontend can reach the backend
- you refreshed the login page

### Redirect URI Mismatch

The provider usually shows an error if the redirect URI does not match.

Copy the redirect URI from Citadel and paste it exactly into the provider. Watch for:

- `http` vs `https`
- trailing slashes
- wrong hostname
- missing `/api/v1`
- wrong provider id

### Login Rejected After Provider Authentication

Check:

- auto-provisioning is enabled, or the identity is already linked
- email is present when needed
- `email_verified` is true when required
- email domain is allowed
- required claim exists and contains the configured value
- the local Citadel user is enabled

### User Signs In But Has No Access

OIDC login does not grant full access by itself.

Assign access in:

```text
Settings -> Access
```

Add the user to teams or assign roles/resource access directly.

### Test Discovery Fails

Check:

- issuer URL is correct
- issuer URL is reachable from the Citadel server
- provider exposes `/.well-known/openid-configuration`
- TLS certificate is valid
- provider authorization, token, and JWKS endpoints are present

Do not enter the authorization endpoint or token endpoint as the issuer URL. Enter the issuer base URL.

## Security Notes

- Keep local admin login available as break-glass access.
- Do not assign admin access automatically from OIDC.
- Keep client secrets out of screenshots, logs, and tickets.
- Use verified email and required claims for auto-provisioning.
- Disable providers that are no longer trusted.
- Use HTTPS for Citadel and the OIDC provider.


