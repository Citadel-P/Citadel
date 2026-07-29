# Two-Factor Authentication

Citadel two-factor authentication adds a six-digit authenticator-app code to local password sign-in.

Citadel TOTP applies only when you sign in with a Citadel username or email and password. If you sign in with OIDC or SSO, multi-factor authentication is controlled by your identity provider.

Supported authenticator apps include:

- Microsoft Authenticator
- Google Authenticator
- Bitwarden
- 1Password
- Authy

## Before You Start

You need:

- a Citadel account that can sign in with a local password
- your current Citadel password
- an authenticator app installed on your phone or password manager

When setup is complete, Citadel shows recovery codes once. Save them somewhere secure before closing the dialog.

## Enable Two-Factor Authentication

Open:

```text
Profile -> Security -> Two-factor authentication
```

Select **Enable**.

Citadel asks for your current password before creating the authenticator setup.

After your password is accepted:

1. Open your authenticator app.
2. Add a new account.
3. Scan the QR code shown by Citadel.
4. If you cannot scan the QR code, enter the manual secret shown below it.
5. Enter the six-digit code from your authenticator app.
6. Select **Confirm**.

Citadel then enables two-factor authentication and shows recovery codes.

## Save Recovery Codes

Recovery codes let you sign in if you lose access to your authenticator app.

Citadel shows recovery codes only once. Use **Copy** or **Download**, then confirm that you saved them.

Each recovery code can be used one time. After a code is used, it cannot be used again.

Store recovery codes somewhere separate from your authenticator device, such as a password manager or another secure location.

## Sign In With Two-Factor Authentication

After two-factor authentication is enabled, local sign-in uses two steps:

1. Enter your username or email and password.
2. Enter the six-digit code from your authenticator app.

Citadel does not issue an access token or refresh session until the second step succeeds.

If you cannot access your authenticator app, select the recovery-code option on the verification screen and enter one unused recovery code.

## Mandatory Setup During Login

An administrator can require two-factor authentication by policy.

When policy requires setup, Citadel prompts you after you enter a valid username or email and password. Complete the QR-code setup and save recovery codes before entering the application.

The same policy applies immediately after the first administrator is created.
See `docs/user/first-run-setup.md` for initial installation instructions.

If the setup session expires, return to the login page and sign in again to start a new setup session.

## Regenerate Recovery Codes

Regenerate recovery codes when:

- you lost your saved recovery codes
- you think someone else may have seen them
- most of your recovery codes have already been used

Open:

```text
Profile -> Security -> Two-factor authentication
```

Select **Recovery Codes**.

Citadel asks for your current password and a current authenticator code. After confirmation, Citadel creates a new set of recovery codes and invalidates the previous set immediately.

## Disable Two-Factor Authentication

Open:

```text
Profile -> Security -> Two-factor authentication
```

Select **Disable**.

Citadel asks for:

- your current password
- either an authenticator code or one unused recovery code

Two-factor authentication cannot be disabled when the active Citadel policy requires it for your account.

## If You Lose Access

If you lose access to both your authenticator app and recovery codes, contact a Citadel administrator.

An administrator can reset your two-factor authentication. This removes your existing TOTP setup, invalidates existing recovery codes, clears outstanding MFA challenges, and signs out existing sessions.

If policy still requires two-factor authentication, your next local password login will require mandatory setup again.

## Troubleshooting

If a valid-looking code is rejected:

- Make sure you are using the current Citadel account entry in your authenticator app.
- Wait for the next six-digit code and try again.
- Check that your device clock is set automatically.
- Use an unused recovery code if you cannot generate a working authenticator code.

If recovery-code verification fails:

- Enter the code exactly as saved.
- Hyphens and spaces are optional.
- Codes are case-insensitive.
- Make sure the code was not already used.

If the setup page expires, sign in again and restart setup.
