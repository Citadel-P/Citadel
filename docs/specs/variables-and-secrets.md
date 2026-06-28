# Variables And Secrets

## Purpose

Citadel needs one platform-agnostic variable and secret model that works for both stacks and deployments.

The feature must support:

- global reusable variables/secrets
- resource-level overrides for stacks and deployments
- stack compose interpolation
- deployment container environment variables
- internal encrypted secrets
- future external secret providers, starting with Vault/OpenBao KV v2
- strict redaction so secret values are not persisted or streamed back to users

Citadel variables and secrets are not Docker Compose secrets, Docker Swarm secrets, or Kubernetes Secrets. Those are delivery mechanisms. Citadel should own the configuration model and let each target materializer decide how values are delivered.

## Main Files

- `src/Citadel.Domain/Entities/Stacks/StackSpec.cs`
- `src/Citadel.Domain/Entities/Deployments/DeploymentSpec.cs`
- `src/Citadel.Application/Services/ApplyStackService.cs`
- `src/Citadel.Application/Services/ApplyDeploymentService.cs`
- `src/Citadel.Contracts/src/Citadel.Hosting.DockerClient/Services/StackService.cs`
- `src/Citadel.Contracts/src/Citadel.Hosting.DockerClient/Services/DeploymentService.cs`
- `src/Citadel.Contracts/src/Citadel.Hosting.DockerClient/Models/Deployments/ApplyDeploymentSpec.cs`
- `src/Citadel.Infrastructure/Persistence`
- `src/Citadel.FrontEnd/src/features/stacks`
- `src/Citadel.FrontEnd/src/features/deployments`
- `src/Citadel.FrontEnd/src/features/variables-secrets`

## Design Rule

Users reference values by name only:

```yaml
services:
  api:
    image: my-api:${IMAGE_TAG}
    environment:
      ASPNETCORE_ENVIRONMENT: ${ASPNETCORE_ENVIRONMENT}
      API_KEY: ${API_KEY}
```

Citadel metadata decides whether `IMAGE_TAG`, `ASPNETCORE_ENVIRONMENT`, and `API_KEY` are normal variables or secrets.

Do not encode type in the key name.

Avoid:

```text
API_KEY_SECRET
IMAGE_TAG_VARIABLE
```

Prefer:

```text
API_KEY
IMAGE_TAG
```

## Core Model

### Variable

A non-sensitive key/value pair.

Examples:

- `IMAGE_TAG`
- `API_PORT`
- `PUBLIC_URL`
- `ASPNETCORE_ENVIRONMENT`
- `LOG_LEVEL`

Variables may be returned in API responses, persisted in config snapshots, shown in diffs, and included in release metadata.

### Secret

A sensitive key/value pair or an external reference.

Examples:

- `DATABASE_PASSWORD`
- `API_KEY`
- `JWT_SIGNING_KEY`
- `REGISTRY_PASSWORD`
- `GITHUB_TOKEN`
- `SSH_PRIVATE_KEY`
- `WEBHOOK_SECRET`

Secret plaintext must never be returned after creation and must never be persisted in:

- stack releases
- deployment specs
- activity events
- alert events
- apply logs
- SignalR streams
- API responses

## Scope And Precedence

Configuration entries can be defined globally or on a resource.

Initial scopes:

- Global
- Stack
- Deployment

Future scopes:

- Project
- Platform
- Team

Precedence must be deterministic:

```text
Resource scope > Global scope
```

If `API_URL` exists globally and also on a stack, the stack value wins every time.

Avoid describing this as "last one wins" because ordering should not affect resolution. Scope determines precedence.

Effective configuration for a resource is:

1. Load global entries the actor/resource is allowed to use.
2. Load resource entries.
3. Merge by normalized key name.
4. Resource entries replace global entries with the same key.
5. Validate the merged effective set.

Future scopes can be inserted later with explicit precedence, for example:

```text
Resource scope > Project scope > Team scope > Global scope
```

## Domain Shape

Use explicit configuration entries instead of raw `KEY=value` strings for new data.

Suggested shared model:

```csharp
public sealed record ConfigurationEntry(
    string Name,
    ConfigurationEntryKind Kind,
    ConfigurationScope Scope,
    Guid? ResourceId,
    string? Value,
    Guid? SecretId,
    SecretDeliveryMode? SecretDeliveryMode = null,
    string? TargetPath = null);

public enum ConfigurationEntryKind
{
    Variable,
    Secret
}
```

For variables:

```text
Kind = Variable
Scope = Global | Stack | Deployment
Value = plaintext value
SecretId = null
SecretDeliveryMode = null
TargetPath = null
```

For secrets:

```text
Kind = Secret
Scope = Global | Stack | Deployment
Value = null
SecretId = reference to secret definition
SecretDeliveryMode = required
TargetPath = required only when the delivery mode needs it
```

Use a dedicated `ConfigurationEntries` table instead of storing entries inside resource specs. Resource specs can project the effective configuration for view purposes, but the canonical variable/secret definitions should live in the configuration subsystem.

Secret plaintext must still live outside configuration entries.

`ConfigurationEntry` is the binding. It describes how a global/resource configuration key is used by Citadel.

`SecretDefinition` is the source. It describes where a secret value comes from.

The materializer is the target-specific delivery layer. It decides how a resolved variable/secret reaches Docker Compose, Docker containers, Swarm, Kubernetes, or future targets.

## Spec Changes

Stacks and deployments currently expose:

```csharp
List<string>? EnvVars
```

Citadel is still MVP, so there is no compatibility requirement to keep this shape.

Remove `EnvVars` from both:

- `StackSpec`
- `DeploymentSpec`

Replace it with structured configuration entries:

```csharp
IReadOnlyList<ConfigurationEntry>? Configuration
```

or a similarly named view/command property if the implementation chooses a more precise name.

The structured model is the only source of truth for runtime variables and secret references.

Do not keep a second legacy environment editor in the API or UI.

Patch behavior:

- patching resource configuration replaces the resource-scoped configuration entry set
- entries are validated as a collection
- duplicate names are rejected within the same scope
- resource entries may use the same name as global entries to create explicit overrides
- secret entries contain references and delivery metadata only
- resolved secret values are never written back into the resource spec

Validation by kind:

```text
Variable:
  Value required
  SecretId null
  SecretDeliveryMode null
  TargetPath null

Secret:
  Value null
  SecretId required
  SecretDeliveryMode required
```

## Secret Storage

Secret storage is where the value comes from. It is separate from how a resource uses the secret.

Initial providers:

- `InternalEncrypted`

Next provider:

- `VaultCompatibleKvV2`

Future providers:

- AWS Secrets Manager
- Azure Key Vault
- GCP Secret Manager
- 1Password
- Doppler

Suggested model:

```csharp
public sealed class SecretDefinition
{
    public Guid Id { get; init; }
    public string Name { get; init; } = default!;
    public SecretProviderType ProviderType { get; init; }
    public Guid? ProviderId { get; init; }
    public string? ExternalPath { get; init; }
    public string? ExternalKey { get; init; }
    public int? ExternalVersion { get; init; }
    public DateTimeOffset CreatedAt { get; init; }
    public DateTimeOffset UpdatedAt { get; init; }
}
```

Internal encrypted secret values should be stored separately from the definition and never returned by normal view queries.

Example:

```text
SecretDefinition:
  Name = STRIPE_API_KEY
  ProviderType = VaultCompatibleKvV2
  ExternalPath = apps/api/prod
  ExternalKey = stripe_api_key

ConfigurationEntry:
  Name = STRIPE_API_KEY
  Kind = Secret
  Scope = Stack
  SecretId = <SecretDefinition.Id>
  SecretDeliveryMode = EnvironmentVariable
```

The `SecretDefinition` answers "where does the value come from?"

The `ConfigurationEntry` answers "which key should this resource expose and how should it be delivered?"

## Secret Providers

Suggested provider enum:

```csharp
public enum SecretProviderType
{
    InternalEncrypted,
    VaultCompatibleKvV2
}
```

For Vault/OpenBao KV v2, use the generic `VaultCompatibleKvV2` name.

Provider config:

```csharp
public sealed class SecretProvider
{
    public Guid Id { get; init; }
    public string Name { get; init; } = default!;
    public SecretProviderType Type { get; init; }
    public string BaseUrl { get; init; } = default!;
    public string MountPath { get; init; } = default!;
    public SecretProviderAuthType AuthType { get; init; }
    public Guid? AuthSecretId { get; init; }
    public DateTimeOffset CreatedAt { get; init; }
    public DateTimeOffset UpdatedAt { get; init; }
}
```

For MVP, support token auth only. The provider token must be stored as an internal encrypted secret.

Vault KV v2 read shape:

```text
GET /v1/{mountPath}/data/{externalPath}?version={version}
Header: X-Vault-Token: <token>
Value: response.data.data[externalKey]
```

Do not log the request auth header or response body.

## Resolver

Deployment and stack apply should call a shared resolver before materialization.

```csharp
public interface IConfigurationResolver
{
    Task<ResolvedConfiguration> ResolveAsync(
        ResourceConfigurationScope scope,
        Guid resourceId,
        CancellationToken cancellationToken);
}

public sealed class ResolvedConfiguration
{
    public IReadOnlyDictionary<string, string> Variables { get; init; } = new Dictionary<string, string>();
    public IReadOnlyList<ResolvedSecret> Secrets { get; init; } = [];
    public IReadOnlyList<string> RedactionValues { get; init; } = [];
}

public sealed class ResolvedSecret
{
    public string Name { get; init; } = default!;
    public string Value { get; init; } = default!;
    public SecretDeliveryMode DeliveryMode { get; init; }
    public string? TargetPath { get; init; }
}
```

The resolver should not know about Docker Compose, Docker containers, Swarm, or Kubernetes. It only resolves names to values and returns redaction values.

The separation must stay clear:

```text
Variable/Secret model = Citadel domain
Provider = where a secret value comes from
Materializer = how the target runtime receives it
```

Resolver steps:

1. Load global entries available to the resource.
2. Load resource-scoped entries.
3. Merge entries by key using `Resource scope > Global scope`.
4. Resolve variables directly.
5. Resolve internal and external secrets through the provider abstraction.
6. Return resolved values plus redaction values.

## Delivery Modes

Suggested enum:

```csharp
public enum SecretDeliveryMode
{
    EnvironmentVariable,
    MountedFile,
    NativePlatformSecret
}
```

MVP support:

- `EnvironmentVariable` for deployments
- `EnvironmentVariable` for stacks, materialized by the Docker Compose backend through a generated compose env file

Future support:

- mounted file
- Docker Compose secret wiring as a materializer implementation detail
- Docker Swarm secrets
- Kubernetes Secret resources

## Stack Behavior

Stacks should use the shared resolver before `StackApplyCommand` is created.

Manual stack flow:

1. Load current stack spec.
2. Resolve effective variables and secrets for the stack.
3. Inject Citadel labels into compose content.
4. Build an environment list for Docker Compose.
5. Pass the environment list to the existing generated env file path.
6. Redact secret values from stack apply output.
7. Delete generated files containing secrets after apply.

Git stack flow:

1. Materialize Git source at the resolved commit.
2. Resolve effective variables and secrets for the stack.
3. Merge repo env files and Citadel-generated values.
4. Citadel-generated values should take precedence over repo env file values.
5. Pass resolved values to the generated env file path.
6. Redact secret values from output.
7. Do not persist plaintext secrets in `StackReleaseSource`, release snapshots, or activity events.

For Docker Compose, users should still write `${NAME}` placeholders. Citadel should generate the env file and let Docker Compose perform interpolation.

The stack config tab is where users reference variables in compose content. The stack Variables & Secrets tab is where users view inherited global entries and define stack overrides.

## Deployment Behavior

Deployments do not use Docker Compose interpolation. They create a Docker container directly.

Deployment flow:

1. Load current deployment spec.
2. Resolve effective variables and secrets for the deployment.
3. Convert resolved entries to Docker container environment entries.
4. Build `ApplyDeploymentCommand`.
5. Apply the deployment through the existing deployment connector.
6. Redact secret values from apply errors, activity events, alert events, logs, and SignalR streams.

For deployments, resolved values should be materialized as:

```text
KEY=value
```

and passed to:

```csharp
ApplyDeploymentCommand.EnvironmentVariables
```

Important: the persisted `DeploymentSpec` must contain only variable values and secret references. It must not contain resolved secret values.

Example user configuration:

```text
ASPNETCORE_ENVIRONMENT = Production
DATABASE_PASSWORD = secret reference
```

Runtime container env:

```text
ASPNETCORE_ENVIRONMENT=Production
DATABASE_PASSWORD=<resolved secret value>
```

Persisted deployment config:

```json
{
  "configuration": [
    {
      "name": "ASPNETCORE_ENVIRONMENT",
      "kind": "Variable",
      "value": "Production"
    },
    {
      "name": "DATABASE_PASSWORD",
      "kind": "Secret",
      "secretId": "...",
      "secretDeliveryMode": "EnvironmentVariable"
    }
  ]
}
```

API responses should mask secret references:

```json
{
  "name": "DATABASE_PASSWORD",
  "kind": "Secret",
  "value": "********",
  "secretId": "...",
  "secretDeliveryMode": "EnvironmentVariable"
}
```

## Redaction

Add one shared redaction service used by both stack and deployment apply.

Hard rule:

```text
Never pass raw command output directly to SignalR, activity events, alert events, logs, or persisted apply output.
```

All stack/deployment output must pass through the redaction service first.

Responsibilities:

- accept resolved secret values
- redact exact secret values from text
- redact from stdout/stderr/progress messages
- redact from activity event info
- redact from alert event info
- redact from exception/error messages before sending to clients
- provide redacted strings before any output is streamed or persisted

Do not log resolved secret values.

Redaction should be best-effort for text streams, but persistence rules must be strict: never persist plaintext secret values intentionally.

## Access Control

Global secrets need explicit use controls.

A user who can attach a global secret to a stack or deployment cannot see the plaintext in Citadel, but they can deploy a workload that consumes it. That is still meaningful access.

MVP options:

- require a specific `Secret_Use` permission to reference global secrets
- or require explicit assignment of global secrets to allowed resources/scopes

Recommended MVP:

```text
Secret_ViewMetadata
Secret_Create
Secret_Update
Secret_Delete
Secret_Use
```

Rules:

- viewing secret metadata does not reveal plaintext
- creating/rotating a secret accepts plaintext write-only
- using a global secret on a resource requires `Secret_Use`
- applying a resource resolves only secrets the resource is allowed to use
- removing `Secret_Use` or assignment should cause future applies to fail safely

Failure messages must be safe:

```text
Secret DATABASE_PASSWORD is not available to this stack.
```

Do not reveal provider tokens or secret values in authorization errors.

## UI

Citadel should expose variables/secrets in two places:

- a global `Variables & Secrets` page
- a `Variables & Secrets` tab on resources that support configuration injection

The stack/deployment `Config` tab should reference keys, but it should not be the primary place to manage variable/secret definitions.

Use shared components where possible, for example:

```text
VariablesSecretsTable
VariableSecretEditorDialog
EffectiveConfigurationView
```

### Global Page

Add a global page:

```text
Variables & Secrets
```

Purpose:

- define reusable variables
- define reusable secrets
- configure external secret references
- later configure secret providers
- show where entries are used or overridden

Minimum columns:

- Name
- Kind
- Provider
- Value / Reference
- Used by
- Updated
- Actions

Global variables are visible according to normal permissions.

Global secrets never expose plaintext. The UI should allow entering plaintext only when creating or rotating an internal secret.

### Stack Variables & Secrets Tab

Add a stack page tab:

```text
Stack
  Config
  Containers
  Releases
  Activity
  Variables & Secrets
```

The tab shows the effective stack configuration:

- global entries inherited by the stack
- stack-scoped entries
- stack overrides of global entries

The stack `Config` tab remains the place where the user edits compose content and references variables with `${NAME}`.

Stack copy:

- title: `Variables & Secrets`
- description: `Values available to Docker Compose at deploy time. Reference them in compose files as ${NAME}.`
- global helper: `Inherited from global configuration.`
- override helper: `Stack values override global values with the same name.`
- secret helper: `Secrets resolve only during deploy and are never shown in plaintext.`

Recommended table columns:

- Name
- Kind
- Effective Source
- Value / Reference
- Delivery
- Actions

Source badges:

- `Global`
- `Stack`
- `Override`

Behavior:

- inherited global entries are read-only from the stack tab
- stack entries can be added/edited/deleted from the stack tab
- clicking `Override` on a global entry creates a stack-scoped entry with the same name
- deleting a stack override reveals the global value again
- the effective value preview should clearly show which value will be used at deploy time

The stack form should no longer show `Env File Path` as a primary user field for Citadel-managed variables. The generated env file path is an implementation detail. If an advanced override remains useful, keep it under an advanced deploy/materialization setting, not in the main config.

### Deployment Variables & Secrets Tab

Add the same tab to deployment pages:

```text
Deployment
  Config
  Activity
  Variables & Secrets
```

Deployment copy:

- title: `Variables & Secrets`
- description: `Values injected into the container environment at deploy time.`
- global helper: `Inherited from global configuration.`
- override helper: `Deployment values override global values with the same name.`

Deployment behavior:

- inherited global entries are read-only from the deployment tab
- deployment entries can be added/edited/deleted from the deployment tab
- clicking `Override` on a global entry creates a deployment-scoped entry with the same name
- deleting a deployment override reveals the global value again

### Config Tab Integration

The stack config tab should help users reference variables/secrets without managing them inline.

Stack config tab:

- Monaco autocomplete should suggest effective variable/secret names for `${NAME}` expressions.
- Validation should warn when compose references `${NAME}` and no effective entry exists.
- Validation should warn when duplicate or unsupported interpolation syntax is likely to fail.
- Missing variables should link to the stack `Variables & Secrets` tab.

Deployment config tab:

- remove the current raw `Environment` editor
- show a compact summary card instead:
  - number of effective variables
  - number of effective secrets
  - link to `Variables & Secrets`
- if useful, show a read-only preview of runtime env names, not secret values

### Component Behavior

Variables:

- inline add/edit/delete for resource-scoped entries
- global edits happen on the global page
- values are visible in diffs and snapshots

Secrets:

- internal secret create flow opens a dialog for entering the value
- existing internal secret values display as `********`
- external secret entries show provider/path/key/version metadata
- generated diffs show secret metadata changes, never plaintext values

For MVP, supported delivery modes in UI:

- Stack: `EnvironmentVariable` displayed as `Compose env`
- Deployment: `EnvironmentVariable` displayed as `Container env`

Disable unsupported delivery modes instead of showing options that do nothing.

### Dirty State And Preview Changes

Resource-scoped entries are part of resource configuration. Any add/update/delete should:

- show a confirmation or save flow consistent with the resource page
- appear in resource configuration diffs where relevant
- preserve masked secret values in the diff
- avoid adding unrelated default fields to the diff

Global entries should use the global page's own create/update/delete flow and should not make every resource form dirty.

## Provider UI

Add provider management after the internal provider is working. This can live under the global `Variables & Secrets` area.

Minimum fields:

- name
- type
- base URL
- mount path
- auth type
- token
- test connection

Provider test endpoints must not return secret values.

## Activity And Release Snapshots

Snapshots should show configuration metadata safely.

Allowed:

- variable names and values
- secret names
- secret scope and effective source
- secret provider name/type
- external path/key
- delivery mode
- masked placeholders

Not allowed:

- internal secret value
- external secret value
- Vault token
- provider auth headers

Stack releases and deployment snapshots should contain enough metadata to understand what was deployed without exposing plaintext.

For global entries, snapshots should record the effective metadata used at apply time. This is important because a later global variable edit should not rewrite the meaning of an older release snapshot.

Snapshot rules:

- variable values are copied into snapshots
- secret values are never copied into snapshots
- secret references and provider metadata are copied safely
- external secret versions should be copied when pinned
- unpinned external secrets should record that they were unpinned

## Alerts

Add minimal alert events only when useful:

- secret resolution failed
- secret provider authentication failed
- secret provider unavailable

Avoid noisy alerts for successful secret resolution.

Failures should include:

- resource name
- resource type
- secret name
- provider name/type when applicable
- safe reason

Failures must not include secret values or auth tokens.

## Validation

Validate configuration keys:

- required
- unique per scope
- shell/env compatible name for MVP: `^[A-Za-z_][A-Za-z0-9_]*$`
- no empty values for variables unless explicitly allowed
- secret must reference a valid secret definition
- delivery mode must be supported by the target resource type
- global and resource entries can share the same name only when the resource entry is an explicit override

For stacks, optional compose validation can warn when:

- compose references `${NAME}` but no effective Citadel variable/secret or repo env file defines it
- Citadel defines a variable/secret that is not referenced by compose

For deployments, validation should ensure every configured entry can be converted to a Docker env entry.

## Implementation Slices

### Slice 1: Internal Variables And Secrets

- Add domain models for global/resource configuration entries and internal secret definitions.
- Add encrypted secret value persistence.
- Add resolver for internal variables/secrets.
- Implement deterministic merge precedence: `Resource scope > Global scope`.
- Add authorization checks for global secret use.
- Wire stack apply to resolver.
- Wire deployment apply to resolver.
- Add redaction service.
- Add tests for non-leak behavior.

### Slice 2: UI

- Add global `Variables & Secrets` page.
- Add stack `Variables & Secrets` tab.
- Add deployment `Variables & Secrets` tab.
- Add effective configuration table with `Global`, `Stack`/`Deployment`, and `Override` badges.
- Add override/remove override flows.
- Add config-tab summaries and compose autocomplete/validation.
- Mask existing secrets.

### Slice 3: Vault-Compatible KV v2

- Add secret provider entity.
- Add token auth stored as an internal secret.
- Add Vault/OpenBao KV v2 provider client.
- Add test connection/test secret endpoints.
- Add UI for provider management.
- Add integration/unit tests with mocked provider responses.

### Slice 4: Safer Delivery Modes

- Mounted files.
- Native Docker/Swarm/Kubernetes secret materialization.
- Secret rotation workflow.

## Tests

Backend tests should cover:

- global variables are available to stacks and deployments
- resource variables override global variables with the same name
- deleting a resource override reveals the global value again
- duplicate names are rejected within the same scope
- global secret use requires permission or resource assignment
- deployment apply resolves variables into container env
- deployment apply resolves secrets into runtime env without persisting plaintext
- stack apply resolves variables/secrets into generated compose env file
- stack release snapshot masks secret references
- deployment activity events mask secrets
- stack activity events mask secrets
- apply errors redact secret values
- raw apply output cannot be streamed or persisted before redaction
- duplicate key validation
- invalid key validation
- variable entries reject secret delivery fields
- secret entries require secret delivery fields
- Vault provider reads KV v2 path/key/version correctly
- Vault auth failures return safe errors

Frontend tests are recommended for:

- masked secret display
- effective source badges
- override/create/delete override flows
- global page create/edit/delete
- preview diff masking
- stack compose autocomplete from effective variables/secrets
- missing variable validation in compose editor

## Non-Goals For First Slice

- Docker Compose `secrets:` as the primary user model
- Swarm native secrets
- Kubernetes Secrets
- Vault dynamic secrets
- leasing/renewal
- automated rotation
- provider-level RBAC beyond existing Citadel permissions
- project/team/platform inheritance

## Open Decisions

- Whether global secret use is controlled by a `Secret_Use` permission only, explicit resource assignment only, or both.
- Whether stack repo env files or Citadel variables win on duplicate keys. Recommended: Citadel variables win.
- Whether deployment secret env values should be visible in Docker inspect. Environment variable delivery implies yes; safer mounted-file/native delivery can come later.
