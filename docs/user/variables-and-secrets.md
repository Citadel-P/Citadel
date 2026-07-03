# Bindings

Citadel bindings let you attach variables and secret keys to stacks and deployments without hard-coding values in compose files or deployment forms.

## Variables

Use a variable for non-sensitive values.

Examples:

- `IMAGE_TAG=1.4.2`
- `APP_ENV=production`
- `PUBLIC_URL=https://app.example.com`

Variables are visible in the UI, diffs, snapshots, and release metadata.

In a compose file, reference a variable by name:

```yaml
services:
  api:
    image: ghcr.io/example/api:${IMAGE_TAG}
    environment:
      APP_ENV: ${APP_ENV}
```

## Secrets

Use a secret for sensitive values.

Examples:

- `POSTGRES_PASSWORD`
- `API_KEY`
- `JWT_SIGNING_KEY`

Secrets have two parts in Citadel:

1. Stored secret: where the secret value comes from.
2. Secret key: the runtime name your stack or deployment uses.

For example:

```text
Stored secret: prod-db-password
Secret key: POSTGRES_PASSWORD
```

Your compose file references the secret key:

```yaml
services:
  db:
    image: postgres:16
    environment:
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
```

At deploy time, Citadel resolves `POSTGRES_PASSWORD` from the stored secret `prod-db-password`. The plaintext value is never shown in the UI after creation and is redacted from logs and activity output.

## Manual Stack Env Files

Manual stacks do not have a separate `.env` file editor.

Use the stack `Bindings` tab for values that would normally live in a local `.env` file. Reference those keys from the Compose editor with `${NAME}` placeholders.

Example:

```yaml
services:
  api:
    image: ghcr.io/example/api:${IMAGE_TAG}
    environment:
      APP_ENV: ${APP_ENV}
      API_KEY: ${API_KEY}
```

Then add `IMAGE_TAG`, `APP_ENV`, and `API_KEY` on the stack `Bindings` tab.

This keeps one source of truth for scoped values, release snapshots, diffs, permissions, and secret redaction. The stack form's `Generated Env File Path` setting only controls where Citadel writes its temporary generated env file during deploy; it is not where users define variables.

## Git Stack Env Files

Git stacks can use repository env files through the stack form's `Compose Env Files` field.

Use this for env files that are part of the Git source, such as `.env`, `compose.env`, or monorepo-specific env files committed beside the compose files. Citadel reads those files from the same resolved Git snapshot as the compose files, so the release records which env file paths were used.

Precedence is explicit:

1. Repository env files provide baseline values from Git.
2. Stack `Bindings` entries are applied after repo env files.
3. If both define the same key, the stack `Bindings` value wins.

This lets Git define shared defaults while Citadel owns deployment-specific overrides and secrets.

## Add Secret Key Vs Create Stored Secret

`Create stored secret` creates a stored secret value or external secret reference.

`Add secret key` creates a runtime secret key that points to an existing stored secret.

Usually you do both:

1. Create a stored secret named `prod-db-password`.
2. Add a secret key named `POSTGRES_PASSWORD`.
3. Select `prod-db-password` as the source for that key.
4. Reference `${POSTGRES_PASSWORD}` in your compose file.

The key and the stored secret can have the same name, but they do not have to. Keeping them separate is useful when:

- the compose file expects a specific environment name
- one stored secret is reused by multiple resources
- an external provider path/key does not match your compose environment name
- a stack or deployment needs to override a global secret key

Stored secret names must be unique. Secret keys are scoped resource bindings, so a resource can override a global secret key by using the same key name.

## External KV v2 Providers

Citadel can resolve secrets from Vault-compatible KV v2 providers, including Vault and OpenBao.

Use this when the secret value should stay in your external secret manager instead of being stored directly in Citadel.

### Create A Provider

On the global `Variables` page, create a Vault provider.

Provider fields:

- `Name`: a friendly name, for example `Production Vault`
- `Address`: the base Vault/OpenBao URL, for example `https://vault.example.com`
- `Mount Path`: the KV v2 engine mount, for example `secret`
- `Token`: a token that can read the needed KV v2 paths

Do not include `/v1` or `/data` in the mount path.

Correct:

```text
Address: https://vault.example.com
Mount Path: secret
```

Avoid:

```text
Mount Path: /v1/secret/data
```

### Create An External Secret

After creating the provider, create a secret with the external source.

Example Vault/OpenBao secret:

```text
Provider: Production Vault
Path: apps/api/prod
Key: stripe_api_key
Version: empty
```

If the Vault UI shows a full path like `secret/apps/api/prod`, use:

```text
Mount Path: secret
Path: apps/api/prod
```

Do not enter `secret/apps/api/prod` as the secret path, because Citadel already adds the provider mount path.

Citadel reads:

```text
GET https://vault.example.com/v1/secret/data/apps/api/prod
```

and uses this value from the KV v2 response:

```text
response.data.data["stripe_api_key"]
```

If you set `Version` to `3`, Citadel reads:

```text
GET https://vault.example.com/v1/secret/data/apps/api/prod?version=3
```

Leave version empty to use the latest version from the provider.

### Use The External Secret

Creating an external secret only defines where the value comes from. You still need to add a runtime secret key.

Example:

```text
External stored secret:
  Name: stripe-prod
  Provider: Production Vault
  Path: apps/api/prod
  Key: stripe_api_key

Runtime secret key:
  Name: STRIPE_API_KEY
  Secret: stripe-prod
```

Your compose file references the runtime key:

```yaml
services:
  api:
    environment:
      STRIPE_API_KEY: ${STRIPE_API_KEY}
```

At deploy time, Citadel fetches the external value and injects it as `STRIPE_API_KEY`.

## Delivery Mode

Citadel supports two delivery modes.

### Environment Variable

Environment variable delivery is supported for stacks and deployments.

Stacks receive referenced keys through Docker Compose interpolation. Deployments receive referenced keys as container environment entries.

Compose example:

```yaml
services:
  db:
    image: postgres:16
    environment:
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
```

### Mounted File

Mounted file delivery is supported for stack secret keys.

Use this when the image supports a file-based secret setting, often with a `_FILE` environment variable.

Example:

```yaml
services:
  db:
    image: postgres:16
    environment:
      POSTGRES_PASSWORD_FILE: /run/secrets/postgres_password
```

Then create a stack secret key:

```text
Name: POSTGRES_PASSWORD
Delivery: Mounted file
Target path: /run/secrets/postgres_password
```

At deploy time, Citadel writes the secret value to a generated stack secret file and mounts it read-only at the target path inside each stack service.

Mounted file delivery is not available for global entries or deployments. Native platform secrets are not supported yet.

### Postgres Mounted File Example

1. Create a stored secret:

```text
Name: postgres-prod-password
Value: <your password>
```

2. On the stack `Bindings` tab, add a secret key:

```text
Name: POSTGRES_PASSWORD
Secret: postgres-prod-password
Delivery: Mounted file
Target path: /run/secrets/postgres_password
```

3. In the compose file, point Postgres at the mounted file:

```yaml
services:
  db:
    image: postgres:16
    environment:
      POSTGRES_PASSWORD_FILE: /run/secrets/postgres_password
```

Do not also reference `${POSTGRES_PASSWORD}` in the compose file unless you intentionally want environment-variable delivery. With mounted-file delivery, Citadel mounts the secret at the target path and does not inject `POSTGRES_PASSWORD=<value>` into the generated env file.

### Safety Rules

Citadel does not show external secret values in the UI.

Citadel should not write provider tokens, raw provider responses, or resolved secret values into:

- activity events
- alert events
- apply logs
- SignalR streams
- release snapshots

If the provider token is wrong, the path is missing, or the key does not exist in the KV v2 payload, the apply fails with a redacted error.

Stack releases store a safe configuration snapshot from apply time. Variables are stored with their values because they are not sensitive. Secrets are stored only as masked metadata, such as the runtime key, stored secret name, provider name/type, external path/key/version, and delivery mode.

This means an older release still shows what configuration metadata was deployed even if a global variable or secret mapping is changed later.

### Local Secret Encryption Key

Citadel encrypts local stored secrets and external provider tokens with a dedicated secret encryption key.

Set it with:

```env
Secrets__EncryptionKey=
```

When the value is empty, Citadel generates a key and stores it in the data volume at `./data/secret-encryption-key`. Keep this file backed up with your Citadel data.

You can also provide your own base64-encoded 32-byte key:

```env
Secrets__EncryptionKey=<base64-32-byte-key>
```

Generate a custom key with PowerShell:

```powershell
[Convert]::ToBase64String([Security.Cryptography.RandomNumberGenerator]::GetBytes(32))
```

Or with OpenSSL:

```bash
openssl rand -base64 32
```

Keep this key outside source control and back it up with your Citadel data. Losing or changing the configured key or generated `./data/secret-encryption-key` file makes existing local stored secrets and provider tokens undecryptable.

This key is intentionally separate from `Jwt__Key`. JWT signing keys may be rotated without breaking stored secrets.

## Scope And Overrides

Variables and secrets can be defined globally or on a resource.

Resource values override global values with the same name.

Example:

```text
Global:
  APP_ENV=production

Stack:
  APP_ENV=staging
```

For that stack, `${APP_ENV}` resolves to `staging`.

## Where To Configure Them

Use the global `Variables` page for reusable values shared by multiple resources.

Use the stack or deployment `Bindings` tab for resource-specific values and overrides.

Resource tabs list only resource-specific entries. Global entries are inherited automatically at deploy time and remain searchable on the global `Variables` page.

Use the stack `Config` tab to reference keys in compose content, for example `${IMAGE_TAG}` or `${POSTGRES_PASSWORD}`.
