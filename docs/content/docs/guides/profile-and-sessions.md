---
title: "Profile and sessions"
description: "Update your profile, change your password, and review browser sessions."
---

Open **Profile** from your account menu to manage your own account. You do not
need administrator access. Administrators manage other accounts through
[Access control](/docs/guides/access-control).

## Update your profile

Under **Profile**, change **Display Name** and save. The name must be unique.
Email is read-only. The header shows your authentication method, direct Roles,
and Team memberships. Roles and Teams show the first item and the number of
additional items; hover, focus, or tap the summary to view the full list.
Contact an administrator for access or account changes.

Under **Preferences**, choose a time zone and date/time format, then save.
These affect personal display, including profile and session timestamps; a job's
configured schedule and time zone remain separate. Appearance changes save
automatically.

## Change your password

For a local account, open **Security → Password** and select **Change Password**.
In the dialog, enter the current password, new password, and confirmation, then
submit the change. Select **Cancel** to close the dialog and clear the fields.
Follow the length requirement shown in the form. Other Citadel refresh sessions are revoked;
the current session is retained when Citadel can identify it.

For an account whose password is managed by an identity provider, change it
there. Citadel does not provide a local password-change form for that account.
See [OIDC providers](/docs/guides/oidc-providers).

Under **Security → Two-factor authentication**, select **Manage two-factor
authentication** to enroll an authenticator or manage recovery codes in a dialog.
The badge beside the heading shows whether MFA is enabled or disabled. It shows
**Unavailable** if the status cannot be read or the account cannot use local-password MFA.
If the status cannot be loaded, the management dialog offers **Retry**.
Follow the [two-factor guide](/docs/guides/two-factor-authentication)
before replacing or losing access to a device.

## Review active sessions

Under **Security → Active Sessions**, expand **Manage active sessions** to review
the browser/device description,
IP address, last-seen time, and expiration. **Current** identifies this browser
session. Shared networks or proxies can make several sessions show the same IP.
The section is collapsed by default.
The list loads once when the profile opens, and the heading shows the loaded
session count while collapsed. Reopening the section reuses the cached list.
Select **Refresh** in the expanded panel's header to check for new sessions. Password, MFA,
and session revocation changes also refresh the list.

- Select **Revoke** for a session you no longer use and confirm.
- Select **Sign Out Others** to revoke other active sessions while retaining
  the current one.
- Use the account menu's sign-out action to leave the current session.

Revocation invalidates both the refresh session and its browser access tokens.
Affected realtime connections close when the revocation commits; other active
sessions can reconnect. Password changes and MFA resets apply the same revocation
rules. Revoking a Citadel session does not sign you out of an upstream OIDC
provider; manage that provider's sessions separately when needed.

**Sign Out Others** is disabled when there are no other sessions to revoke.
If other sessions are listed but the action is unavailable, sign in again so
Citadel can identify the current refresh session. If you cannot sign in at all, use an MFA recovery code
when applicable or contact another administrator. Restarting Core does not reset
your password or reopen first-run setup.
