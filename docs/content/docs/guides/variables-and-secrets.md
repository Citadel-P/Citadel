---
title: "Variables and secrets"
description: "Resolve reusable variables and protected secrets into managed workloads."
---

Citadel bindings let you attach variables and secret keys to stacks and deployments without hard-coding values in compose files or deployment forms.

## Supply a secret to an application

1. Open the Stack or Deployment's **Bindings** tab and select **Create stored secret**.
   Create a local secret with a descriptive name, such as `production-api-token`.
2. Select **Add secret key**, name it `API_TOKEN`, and choose that stored secret.
   Use **Environment variable** delivery.
3. Reference `API_TOKEN: ${API_TOKEN}` under the Compose service's `environment`,
   or enter `API_TOKEN` in a Deployment's **Container Variables**.
4. Save and deploy. Verify the application's authenticated operation without
   printing the secret in logs or a terminal.

Saving a binding does not change an existing container's environment. Reapply
the consuming workload when you intend to use the new value. Use global bindings
only when the value should be inherited by multiple resources.

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

At deploy time, Citadel resolves `POSTGRES_PASSWORD` from the stored secret
`prod-db-password`. Citadel masks secret metadata and redacts resolved values
from its operation output. The application still receives plaintext; Docker
inspection, application logs, or commands inside the container can expose it.

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

Open **Settings → Bindings** and create a Vault provider.

Provider fields:

- `Name`: a friendly name, for example `Production Vault`
- `Address`: the base Vault/OpenBao URL, for example `https://vault.example.com`
- `Mount Path`: the KV v2 engine mount, for example `secret`
- `Token`: a token that can read the needed KV v2 paths

Do not include `/v1` or `/data` in the mount path.

Use **Test connection** before saving. When editing a provider, leave the token
empty to keep its stored token. Test an external secret reference as well: a valid
token does not necessarily have permission to read every path. The test reports
whether the reference resolves without displaying its value.

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

The Stack binding editor exposes **Mounted file**, but the current Rust Stack
deployment path does not deliver these bindings. A referenced secret using
this mode fails deployment with an environment-variable delivery error;
an unreferenced binding does not create a file mount.

Use **Environment variable** for Citadel-managed secret bindings. For a Swarm
application that needs file-based secrets, create a Docker Secret from the
Platform's **Secrets** page and reference it as an external secret in Compose.
This is separate from a Citadel stored-secret binding. See
[Swarm Secrets and Configs](/docs/resources/platforms/docker-swarm#manage-secrets-and-configs).

Mounted-file bindings are also unavailable for global entries and Deployments.

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

```text
Secrets__EncryptionKey=
```

When the value is empty, Citadel generates a key at
`/app/data/secret-encryption-key` in the supplied container. A custom
`CITADEL_DATA_ROOT` changes that directory. Preserve the key with the database
using [control-plane backups](/docs/operations/control-plane-recovery).

You can also provide your own base64-encoded 32-byte key:

```text
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

Keep this key outside source control and back it up with your Citadel data.
Losing or replacing it makes existing local stored secrets and provider tokens
undecryptable. Changing this key is not a way to rotate an application's password.

This key is intentionally separate from `Jwt__Key`. JWT signing keys may be rotated without breaking stored secrets.

## Rotate an application secret

For a local secret, create a replacement stored secret, then edit the runtime
secret key to reference it. Keep the runtime name stable so Compose or container
configuration does not need to change. The current editor supports editing
external references; it does not edit an existing local secret's plaintext value.

For an external secret, update its value in the provider. An unpinned reference
uses the latest version at the next apply; a reference with **Version** set
continues using that version until you edit it. Test the reference after changing
its provider, path, key, or version.

Review every consuming resource before changing a shared reference. Deploy the
affected workloads, verify their authenticated operations, then revoke the old
credential at the external service when no consumer needs it. Follow the
application's own password-rotation procedure: changing an environment variable
does not necessarily change an existing database user's password.

Stack rollback resolves secret references again. Keep a separate recovery plan
for credentials; an older release does not restore an old secret value.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Missing or unresolved binding | Runtime key spelling, resource/global scope, and whether the workload references that key. |
| A changed value is not in a running container | Save the binding and reapply the consuming workload; saving alone does not update it. |
| External secret fails to resolve | Provider connectivity, token access to the exact KV v2 path, key name, and pinned version. Test the reference as well as the provider. |
| Local secrets fail after recovery | Restore the original encryption key; generating another key cannot decrypt the database's existing values. |
| Mounted file is rejected | Use the supported delivery path described under [Delivery Mode](#delivery-mode). |

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

Use **Settings → Bindings** for reusable values shared by multiple resources.

Use the stack or deployment `Bindings` tab for resource-specific values and overrides.

Resource tabs list only resource-specific entries. Global entries are inherited automatically at deploy time and remain searchable on **Settings → Bindings**.

Use the stack `Config` tab to reference keys in compose content, for example `${IMAGE_TAG}` or `${POSTGRES_PASSWORD}`.
