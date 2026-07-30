# First-Run Setup

Citadel does not ship with a default administrator username or password.

For a Docker Compose installation, prepare the production environment before
starting Core. See [Docker Compose configuration](docker-compose-configuration.md).

## Interactive Setup

Start Citadel and open its normal browser URL. A new installation redirects to
`/setup`.

Enter:

1. The administrator username.
2. The administrator email address.
3. A password containing 15 to 128 characters.
4. The same password again.

Select **Create administrator**. Citadel creates the account with the built-in
Admin role and signs you in. Setup cannot be opened again after it succeeds.

The password should be unique to Citadel and stored in a password manager.

## Two-Factor Authentication

Two-factor authentication is optional by default. It can be enabled later from
**Profile -> Security**.

To require the initial administrator to enroll before entering Citadel, set:

```env
Mfa__Policy=RequiredForAdministrators
```

The available policies are:

- `Optional`
- `RequiredForAdministrators`
- `RequiredForAllUsers`

See `docs/user/two-factor-authentication.md` for enrollment and recovery-code
instructions.

## Unattended Setup

Automated installations can initialize Citadel from a password file. The
password is not accepted directly through an environment variable.

Set all three options:

```env
Bootstrap__AdminName=citadel-admin
Bootstrap__AdminEmail=admin@example.com
Bootstrap__AdminPasswordFile=/run/secrets/citadel-admin-password
```

Mount the password file read-only into the server container:

```yaml
services:
  server:
    volumes:
      - ./secrets/citadel-admin-password:/run/secrets/citadel-admin-password:ro
```

The path must be absolute inside the container. The file must contain only the
password, may end with one newline, must not contain other line breaks, and must
not exceed 1 KiB.

If only some bootstrap options are configured, Citadel refuses to start. If
setup is already complete, Citadel ignores the bootstrap options and does not
read the file.

After the first successful start:

1. Confirm that the administrator can sign in.
2. Remove the three bootstrap settings.
3. Remove the password-file mount.
4. Delete the local password file securely.

## Recovery

Clearing browser cookies, deleting sessions, or restarting Citadel does not
reopen first-run setup. The setup endpoint is not a password-recovery feature.

Use another administrator account to restore access. Directly deleting or
editing the setup-state database row is unsupported.

Citadel is still pre-release. Databases created with an older development
baseline that seeded `admin` must be recreated with the current baseline rather
than modified manually.
