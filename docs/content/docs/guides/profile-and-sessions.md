---
title: "Profile and sessions"
description: "Update your profile, change your password, and review browser sessions."
---

Open **Profile** from your account menu to manage your own account. You do not
need administrator access. Administrators manage other accounts through
[Access control](/docs/guides/access-control).

## Update your profile

Under **Profile**, change **Display Name** and save. The name must be unique.
Email, authentication method, direct Roles, and Team memberships are read-only
here; contact an administrator for access or account changes.

Under **Preferences**, choose a time zone and date/time format, then save.
These affect personal display, including profile and session timestamps; a job's
configured schedule and time zone remain separate. Appearance changes save
automatically.

## Change your password

For a local account, open **Security → Password**, enter the current password,
new password, and confirmation, then submit the change. Follow the length
requirement shown in the form. Other Citadel refresh sessions are revoked;
the current session is retained when Citadel can identify it.

For an account whose password is managed by an identity provider, change it
there. Citadel does not provide a local password-change form for that account.
See [OIDC providers](/docs/guides/oidc-providers).

Use **Security → Two-factor authentication** to enroll an authenticator and
save recovery codes. Follow the [two-factor guide](/docs/guides/two-factor-authentication)
before replacing or losing access to a device.

## Review active sessions

Under **Security → Active Sessions**, review the browser/device description,
IP address, last-seen time, and expiration. **Current** identifies this browser
session. Shared networks or proxies can make several sessions show the same IP.

- Select **Revoke** for a session you no longer use and confirm.
- Select **Sign Out Others** to revoke other active sessions while retaining
  the current one.
- Use the account menu's sign-out action to leave the current session.

Revocation prevents further refreshes. An already issued access token can remain
valid until expiration. This does not sign you out of an upstream OIDC provider;
manage that provider's sessions separately when needed.

If **Sign Out Others** is unavailable, sign in again so Citadel can identify the
current refresh session. If you cannot sign in at all, use an MFA recovery code
when applicable or contact another administrator. Restarting Core does not reset
your password or reopen first-run setup.
