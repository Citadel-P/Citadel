# Citadel Offline Licensing and Capability Enforcement

## Status

Implemented specification for Citadel's capability-based offline licensing
model.

The companion administrator guide is
[`docs/user/licensing.md`](../user/licensing.md).

Earlier releases used schema-1 `Business` licenses and numeric limits for
custom roles, active users, platforms, backup policies, and automation actions.
The current model replaces those limits with:

- `Community`, `Team`, and `Enterprise` editions;
- coarse product capabilities instead of database-row quotas;
- unlimited core resource counts from a licensing perspective;
- non-destructive downgrade behavior;
- backward compatibility for existing schema-1 Business licenses.

Implementation must follow the migration order in this document. Do not make an
existing valid Business license unusable during the transition.

## Goal

Citadel must remain useful without a license. Community is the complete core
product for self-hosting, evaluation, and individual operation. Paid editions
monetize how safely and collaboratively teams operate, not how many rows they
insert.

Licensing should reflect customer value:

- Team unlocks advanced collaboration, unattended operations, notification
  routing, and continuous operational safeguards.
- Enterprise will unlock organizational governance, identity lifecycle,
  high-availability operation, and formal support assurances when those features
  exist.

Licensing must be fully offline. A signed license raises the effective
capabilities for one Citadel installation without contacting Citadel services.

Licensing must also be non-destructive. Missing, invalid, expired, or removed
licenses must not delete resources, discard configuration, remove authorization
assignments, interrupt in-flight recovery, or make customer data inaccessible.

## Product Principles

The following rules are product requirements, not implementation suggestions:

1. Do not license ordinary entity counts.
2. Do not limit platforms, users, teams, stacks, deployments, builds, build
   pools, backup definitions, automation definitions, or other resource rows.
3. Do not gate authentication enforcement, MFA, security fixes, API access,
   manual rollback, manual recovery, backup restore, deletion, or access to
   customer data.
4. Do not make a connector type, orchestrator type, or infrastructure provider
   paid merely because it is new. Basic Docker, agent, edge-agent, future Swarm,
   and future Kubernetes management belong to the core product.
5. Gate coherent workflows at their ownership boundaries. Do not scatter
   edition-name checks through unrelated repositories and entity handlers.
6. Backend enforcement is authoritative. Frontend locks are explanatory and
   cannot be the only enforcement.
7. Existing paid configuration remains stored and visible after downgrade.
8. Grace-period licenses keep paid capabilities active until `graceUntil`.
9. Support terms and SLAs are contractual. Do not model support as a runtime
   capability unless software behavior genuinely depends on it.
10. Do not advertise or render an Enterprise feature before its implementation
    exists.

## Edition Model

### Community

Community is active when no valid paid license is effective.

Community includes:

- all currently supported platform and connector types;
- platforms, stacks, deployments, containers, images, networks, volumes,
  registries, Git repositories, webhooks, and builds on existing managed
  platforms;
- local authentication, MFA, and the existing OIDC implementation;
- users and teams with the built-in `Admin`, `Operator`, and `Viewer` roles;
- permission enforcement for built-in roles;
- manual deployment, build, rollback, stack drift check, and reconciliation;
- backup repositories, backup policy definitions, on-demand backup, and restore;
- automation action definitions, testing, and manual execution;
- built-in alerts and in-application notification history;
- activity history and raw customer-data access;
- all read, disable, delete, export, and recovery operations needed to leave a
  paid feature safely.

Community has no license-enforced resource-count or user-count limits.
Operational limits that protect the service, such as request size, concurrency,
timeouts, storage, and rate limits, remain technical configuration and must not
be presented as edition quotas.

### Team

Team is the first paid edition. A Team license may enable these shipped
capabilities:

| Capability | Stable key | Behavior |
| --- | --- | --- |
| `CustomAccessControl` | `custom-access-control` | Create and assign custom roles, expand custom permissions, and configure resource overrides or scoped grants. |
| `AutomatedOperations` | `automated-operations` | Enable schedules or webhooks that execute automation actions, backups, builds, deployments, or stack applies. Repository webhook reception, synchronization, and update detection remain Community; unattended execution is paid. |
| `AdvancedAlerting` | `advanced-alerting` | Create custom alert rules and configure advanced conditions such as quiet hours, cooldowns, thresholds, required matches, severity, and resource scoping. Notification channels and external delivery for seeded system rules remain Community. |
| `OperationalGuardrails` | `operational-guardrails` | Enable continuous drift monitoring, opt-in automatic reconciliation, and resource auto-update safeguards. Manual checks and updates remain Community. |
| `ElasticBuildExecution` | `elastic-build-execution` | Execute builds through external build pools that provide dedicated, remote, or ephemeral builder compute. Manual builds on existing Citadel-managed platforms remain Community. |

The Team product bundle issued by Citadel should normally contain every shipped
Team capability. Explicit signed capabilities are still used so design-partner
features and future add-ons do not require a new license format.

Team support may be sold with the subscription, but support response times are
maintained in the commercial agreement rather than enforced by Citadel Core.

A standard Team subscription may commercially include one production
installation and one non-production installation for upgrade and recovery
validation. Each installation receives a separate schema-2 license bound to
its own `instanceId`; a license payload never authorizes multiple instances.
The private issuance registry groups both licenses under the same commercial
subscription.

### Enterprise

Enterprise is a recognized edition in the license format, but it is not a
generally available product until it has material capabilities beyond Team.

Likely future Enterprise capability areas are:

- enterprise identity lifecycle: SCIM, IdP group synchronization, enforced SSO,
  and assignment-source-aware deprovisioning;
- governance: approvals, change windows, organization-wide policy distribution,
  separation of duties, immutable audit delivery, and compliance reporting;
- high availability: multiple Citadel control-plane replicas, distributed job
  ownership, a SignalR backplane, shared materialization state, failover, and
  validated control-plane recovery;
- enterprise support: production SLAs, upgrade assistance, validated release
  paths, and incident response.

SCIM, high availability, immutable audit delivery, and compliance reporting do
not currently exist in Citadel. They must not be added to
`LicenseCapability`, emitted by the issuer, shown on the License page, or listed
as available product features until their implementations and tests are merged.

An Enterprise license issued for a design partner may contain the shipped Team
capabilities. The public product must not imply that the Enterprise edition
already provides the future capabilities listed above.

## Capability Model

Use stable enum values in code and stable kebab-case strings in signed payloads:

```csharp
public enum LicenseCapability
{
    CustomAccessControl,
    AutomatedOperations,
    AdvancedAlerting,
    OperationalGuardrails,
    ElasticBuildExecution
}
```

```text
custom-access-control
automated-operations
advanced-alerting
operational-guardrails
elastic-build-execution
```

The initial implementation must not add numeric edition limits or an unlimited
sentinel.

Future capabilities require all of the following before becoming known to Core:

1. the feature is implemented;
2. backend enforcement boundaries are defined;
3. downgrade behavior is defined;
4. unit, integration, and acceptance tests exist;
5. the issuer knows the capability key;
6. product documentation can accurately describe it.

The paid edition name is presentation and commercial metadata. The signed
capability list is the enforcement authority. Core must not grant capabilities
solely because `edition` is `Team` or `Enterprise`.

### Edition And Capability Responsibilities

The runtime contract is:

- schema-2 validation recognizes `Team` and `Enterprise` as valid paid edition
  values;
- `EffectiveEdition` reports the effective signed edition for presentation and
  diagnostics;
- `EffectiveCapabilities` contains only known capabilities explicitly present
  in the signed payload while the license is effective;
- entitlement checks inspect `EffectiveCapabilities`, never the edition name;
- an Enterprise license does not implicitly inherit shipped Team capabilities;
- an Enterprise license containing a shipped Team capability may use that
  capability exactly as a Team license can;
- an Enterprise license missing a required capability must be denied even
  though its edition is commercially higher.

Every paid capability implemented in the current release belongs to the Team
product boundary. Enterprise support in the license schema is forward
compatibility and controlled design-partner infrastructure, not an implemented
Enterprise feature set.

Do not introduce a general `IsTeam`, `IsEnterprise`, minimum-edition comparison,
or edition hierarchy for feature authorization. A future Enterprise feature
must receive its own stable capability and complete the implementation
requirements above before any runtime or frontend gate uses it.

## Citadel Alignment

Follow existing Citadel conventions:

- Public API routes remain under `/api/v1`.
- Keep route registration in
  `src/Citadel.WebApi/Routes/PublicEndpoints.cs`.
- Keep endpoint methods thin under `src/Citadel.WebApi/Routes/Endpoints`.
- Put license application logic under
  `src/Citadel.Application/Features.Licensing`.
- Put license domain values under
  `src/Citadel.Domain/Entities/Licensing` or
  `src/Citadel.Domain/Contracts/Resources/Licensing`.
- Put infrastructure persistence under
  `src/Citadel.Infrastructure/Persistence`.
- Keep repository code compatible with Dapper AOT.
- Update generated frontend API files through the existing generation flow.
- Add database changes through the EF model and generated migrations only when
  the persistence schema changes.
- Use `TimeProvider` for validation and transition checks.
- Use result-based expected failures rather than exceptions for license
  validation and entitlement denial.
- Do not stage or unstage files as part of implementation unless explicitly
  requested.

## License File Format

Continue using compact JWS:

```text
BASE64URL(protected-header).BASE64URL(payload).BASE64URL(signature)
```

The license is signed but not encrypted and may be stored as:

```text
<customer>.citadel-license
```

Use Ed25519 with the fully specified JOSE identifier `alg = Ed25519`.

Protected header:

```json
{
  "alg": "Ed25519",
  "typ": "citadel-license+jws",
  "kid": "citadel-license-2026-01"
}
```

Verifier requirements:

- Accept only `alg = Ed25519`.
- Accept only `typ = citadel-license+jws`.
- Require a known `kid`.
- Verify against an embedded Citadel public key selected by `kid`.
- Reject `alg = none`, `alg = EdDSA`, HMAC, RSA, ECDSA, and unknown
  algorithms.
- Reject public keys embedded in the license.
- Reject remote key references such as `jku`, `x5u`, and equivalent headers.
- Reject unsupported critical JOSE headers.
- Do not use Citadel JWT authentication configuration for license verification.

The production private signing key must never be stored in this repository, a
Docker image, application settings, CI logs, frontend code, or non-private test
fixtures.

## Schema-2 License Payload

New licenses use schema version `2`.

```json
{
  "schema": 2,
  "product": "citadel",
  "issuer": "citadel-p",
  "audience": "citadel-core",
  "licenseId": "lic_2026_000001",
  "replacedLicenseId": "lic_2025_000123",
  "customer": {
    "id": "customer-example",
    "name": "Example Corp"
  },
  "edition": "Team",
  "instanceId": "019f0000-0000-7000-8000-000000000001",
  "issuedAt": "2026-07-12T00:00:00Z",
  "notBefore": "2026-07-12T00:00:00Z",
  "expiresAt": "2027-07-12T00:00:00Z",
  "graceUntil": "2027-07-26T00:00:00Z",
  "capabilities": [
    "custom-access-control",
    "automated-operations",
    "advanced-alerting",
    "operational-guardrails",
    "elastic-build-execution"
  ]
}
```

`replacedLicenseId` is present only for a renewal or rehost that supersedes a
previous license.

Validation rules:

- `schema` must be supported.
- `product` must equal `citadel`.
- `issuer` must equal `citadel-p`.
- `audience` must equal `citadel-core`.
- `licenseId` must be non-empty.
- `replacedLicenseId` is optional, must be non-empty when present, and must not
  equal `licenseId`.
- `edition` must equal `Team` or `Enterprise` for schema-2 paid licenses.
- `instanceId` must match the persisted Citadel instance ID.
- `issuedAt`, `notBefore`, `expiresAt`, and optional `graceUntil` must be valid
  UTC timestamps.
- `issuedAt <= notBefore`.
- `issuedAt <= expiresAt`.
- `notBefore <= expiresAt`.
- `graceUntil >= expiresAt` when present.
- `capabilities` must contain at most 64 entries.
- Capability keys are case-sensitive, non-empty, and at most 128 characters.
- Duplicate capability keys are invalid.

`replacedLicenseId` is cryptographically protected metadata, but replacement
semantics also depend on installed state. When a license is already installed,
Core must require the new payload to contain a `replacedLicenseId` that exactly
matches the installed `licenseId`. A missing or mismatched replacement ID is
rejected. When no license is installed, a signed replacement may be accepted to
support a rehost onto a new instance. The verifier validates the payload without
installed state; the install application service owns this comparison and must
perform it in the same transaction as replacement.

- Unknown capability keys are ignored for forward compatibility and surfaced
  as warnings.
- Missing known capability keys are disabled.
- A schema-2 `limits` object has no effect and must not be emitted by the issuer.

Defensive input limits:

- compact license: 64 KB;
- `kid`: 128 characters;
- `licenseId`: 128 characters;
- `replacedLicenseId`: 128 characters;
- `customer.id`: 128 characters;
- `customer.name`: 256 characters;
- JSON depth: a small explicit parser limit;
- capability count: 64.

Do not include customer email, host inventory, platform addresses, or support
contract details in the signed payload. A license is readable by anyone who can
access the file.

Input normalization:

- trim leading and trailing ASCII whitespace introduced by copy and paste;
- reject whitespace inside the compact JWS;
- store and verify the resulting exact three-segment string;
- calculate the fingerprint from that exact string;
- never decode and reserialize a segment before verification or fingerprinting.

## Schema-1 Business Compatibility

Core must continue to verify existing schema-1 `Business` licenses during the
migration period.

Compatibility behavior:

- Preserve all schema-1 signature, issuer, audience, instance, timestamp, and
  limit-value validation needed to prove that the old license is authentic.
- A schema-1 Business license in `Valid` or `GracePeriod` status maps to the
  complete set of shipped Team capabilities.
- Former numeric limits do not restrict resource counts after the capability
  model is enabled.
- A schema-1 license in `NotYetValid`, `Expired`, `Invalid`, or another inactive
  state has Community capabilities.
- Report `LicensedEdition = Business` and `EffectiveEdition = Team` while an
  active legacy license is mapped.
- Surface a warning that the license uses the legacy schema and should be
  replaced at renewal.
- The issuer must stop creating schema-1 licenses after schema-2 support is
  deployed.
- Schema-1 support must not be removed without a separately documented major
  release transition.

Deploy Core support for schemas 1 and 2 before issuing the first schema-2
license.

## Instance Identity

Continue binding offline licenses to a stable persisted instance identity:

```text
CitadelInstanceIdentity
- Id          integer primary key, constrained to 1
- InstanceId  uuid not null unique
- CreatedAt   timestamptz not null
```

Rules:

- Generate the instance ID once on first startup or first license request.
- Persist it so it survives container rebuilds and application restarts.
- Do not derive it from machine IDs, Docker host IDs, IP addresses, hostnames,
  or deployment paths.
- Do not regenerate it automatically after database restore.
- Return it from the license request endpoint.
- Use an atomic singleton insert-or-read operation.
- Keep the identity immutable.

## Persistence

Continue storing one installed license:

```text
InstalledLicenses
- Id                      int primary key, fixed value 1
- RawLicense              text, required
- Fingerprint             text, required
- InstalledAt             timestamptz, required
- InstalledByActorId      uuid, nullable
- LastValidatedAt         timestamptz, nullable
- LastValidationStatus    text, nullable
- LastValidationErrorCode text, nullable
```

The raw license remains the source of truth and must never be returned by an API
response, activity event, log message, metric, or frontend state.

The fingerprint is SHA-256 over the exact stored compact JWS after trimming only
surrounding ASCII whitespace.

No database migration is required solely for schema 2 because the signed
payload is already stored as opaque text. Remove the license-usage counting
query when no other caller requires it.

## License State

Retain these statuses:

```csharp
public enum LicenseStatus
{
    Community,
    Valid,
    GracePeriod,
    NotYetValid,
    Expired,
    Invalid,
    InstanceMismatch,
    UnsupportedSchema,
    UnknownSigningKey
}
```

Distinguish the installed edition from the currently effective edition:

```csharp
public sealed record LicenseState(
    LicenseStatus Status,
    string EffectiveEdition,
    string? LicensedEdition,
    Guid InstanceId,
    string? LicenseId,
    string? ReplacedLicenseId,
    string? CustomerId,
    string? CustomerName,
    string? Fingerprint,
    DateTimeOffset? IssuedAt,
    DateTimeOffset? NotBefore,
    DateTimeOffset? ExpiresAt,
    DateTimeOffset? GraceUntil,
    IReadOnlySet<LicenseCapability> EffectiveCapabilities,
    IReadOnlyList<string> Warnings);
```

Effective behavior:

| Status | Effective edition | Effective paid capabilities |
| --- | --- | --- |
| `Community` | Community | None |
| `Valid` schema 2 | Signed edition | Known signed capabilities |
| `GracePeriod` schema 2 | Signed edition | Known signed capabilities |
| `Valid` or `GracePeriod` schema 1 Business | Team | All shipped Team capabilities |
| `NotYetValid` | Community | None |
| `Expired` | Community | None |
| `Invalid` | Community | None |
| `InstanceMismatch` | Community | None |
| `UnsupportedSchema` | Community | None |
| `UnknownSigningKey` | Community | None |

Derive temporal state from `TimeProvider.GetUtcNow()` whenever effective state is
requested:

```text
now < notBefore
    => NotYetValid

notBefore <= now <= expiresAt
    => Valid

expiresAt < now <= graceUntil
    => GracePeriod

now > graceUntil
    => Expired

when graceUntil is absent:
    now > expiresAt => Expired
```

Static verification results may be cached. Time-dependent status must not be
cached indefinitely. Recalculate it from `TimeProvider` on every effective-state
read.

The state provider must not retain an `IUnitOfWork` or database connection in a
singleton cache. Invalidate static cached state after install and removal.

A background transition monitor must calculate the next temporal boundary from
`notBefore`, `expiresAt`, and `graceUntil`, reevaluate the effective state
within one minute of that boundary, persist the transition once, and publish a
metadata-free `LicenseStateChanged` SignalR event. This notification is required
for UI freshness only; backend entitlement checks remain authoritative.

## Entitlement Service

Replace `ILicenseQuotaService` with an entitlement-oriented service:

```csharp
public interface ILicenseEntitlementService
{
    ValueTask<LicenseState> GetOverviewAsync(CancellationToken cancellationToken);

    ValueTask<bool> IsEnabledAsync(
        LicenseCapability capability,
        CancellationToken cancellationToken);

    ValueTask<Result> EnsureEnabledAsync(
        LicenseCapability capability,
        CancellationToken cancellationToken);
}
```

Do not expose edition-name checks such as:

```csharp
if (license.Edition == "Team")
```

Callers request the capability they own. This keeps commercial packaging out of
application features and permits a signed Enterprise or design-partner license
to use the same enforcement path.

Avoid one database read per component or command. Resolve one effective license
snapshot per request and share it through the state provider. Long-running jobs
may reuse verified static payload data but must recalculate temporal status.

Remove:

- `LicenseLimit`;
- `LicenseUsageSnapshot`;
- `LicenseLimitState`;
- `LicenseQuotaViolation`;
- `LicenseQuotaExceeded`;
- `CommunityLicenseLimits`;
- `LicenseLimitKeys`;
- entity-counting license repository methods;
- quota checks from platform, user, role, backup policy, automation action, and
  OIDC provisioning handlers.

## Enforcement Boundaries

### Custom Access Control

Require `CustomAccessControl` when an operation:

- creates a custom role;
- assigns a custom role to a user or team;
- adds or expands permissions on a custom role;
- creates or expands a resource override or scoped resource grant.

Community must continue to allow:

- built-in role assignment;
- authorization evaluation for existing custom-role assignments;
- viewing and renaming existing custom roles;
- removing permissions;
- removing role assignments or resource overrides;
- deleting custom roles.

Authorization must continue to evaluate existing custom roles after downgrade.
Silently ignoring those roles could either lock administrators out or grant
broader fallback access and is therefore unsafe.

### Automated Operations

`AutomatedOperations` covers time-triggered or externally triggered execution.
It does not cover webhook reception or change detection by itself.

Require `AutomatedOperations` when an operation:

- enables or changes an automation action schedule;
- enables or changes an automation action webhook trigger;
- enables or changes a backup policy schedule;
- enables or changes a backup policy webhook trigger;
- enables or changes a webhook or schedule that automatically starts a build,
  deployment, or stack apply;
- enables `Redeploy On Build` for a deployment or stack build-image binding;
- starts a deployment or stack apply automatically after a successful build;
- accepts an automation, backup, build, deployment, or stack webhook invocation
  that will start a mutating operation;
- enqueues a run from one of those paid triggers.

Do not require it for:

- creating an automation action definition;
- testing or manually running an automation action;
- creating a backup policy definition;
- manually starting a backup;
- restoring a backup;
- receiving and authenticating a Git or repository webhook;
- fetching or synchronizing repository changes;
- detecting a new commit, image, build input, or pending release;
- recording and displaying that an update is available;
- recording a successful build artifact and updating a consumer's desired
  artifact without applying it;
- manually starting a build, deployment, or stack apply;
- viewing run history or logs.

Scheduled and webhook-generated queue items must retain trigger provenance so a
worker can distinguish them from manual runs. Check the capability when
accepting the trigger and again before starting a queued paid run.

Webhook reception must be separated from webhook-triggered execution. Community
may receive, validate, synchronize, and display a repository change. It must not
automatically build, deploy, apply, run an action, or start a backup without
`AutomatedOperations`.

### Advanced Alerting

Require `AdvancedAlerting` when an operation:

- creates a non-system alert rule;
- adds or expands conditions on a non-system alert rule;
- changes quiet hours, cooldowns, thresholds, required matches, severity, or
  resource scoping on any rule;
- assigns notification channels to a non-system alert rule.

Community must continue to provide:

- creation, testing, update, enablement, and deletion of notification channels;
- assignment of notification channels to seeded system rules;
- enablement and disablement of seeded system rules;
- built-in alert evaluation and external delivery needed for basic product
  health;
- in-app alert counts and descriptions;
- access to historical and unresolved alerts;
- resolution of existing alerts;
- visible failures for backups, builds, and deployments.

A seeded system rule is an alert rule created by Citadel's system actor. In
Community, updates to a seeded rule are limited to its enabled status and
notification-channel assignments. Changing its conditions or presentation
requires `AdvancedAlerting`.

Notification channels are not a licensed entity and have no license-enforced
count limit. All supported destination types are available in Community.

### Operational Guardrails

Require `OperationalGuardrails` when an operation enables:

- continuous stack drift monitoring;
- opt-in automatic drift reconciliation;
- deployment or stack image auto-update;
- future continuous safety checks explicitly assigned to this capability.

Do not require it for platform health, container status streaming, ordinary
resource synchronization, manual drift checks, manual reconciliation, manual
updates, or rollback.

Add new behavior to this capability only when it is an unattended operational
safeguard. Do not turn ordinary core monitoring into a paid feature.

`OperationalGuardrails` applies when Citadel initiates work because continuously
observed state changed, such as detected drift or a newly available image.
`AutomatedOperations` applies when a schedule or external webhook initiates the
work. A single trigger path must have one documented owning capability and must
not silently fall through to an unlicensed execution path.

Independent execution-target capabilities may also apply. For example, a
webhook-triggered build dispatched to an external build pool requires both
`AutomatedOperations` for the trigger and `ElasticBuildExecution` for the
execution target. A manual build on that pool requires only
`ElasticBuildExecution`.

### Elastic Build Execution

Require `ElasticBuildExecution` when an operation:

- selects an external build pool as the build execution target;
- requests, leases, provisions, starts, or resumes dedicated or ephemeral
  builder compute;
- dispatches a build to an external builder agent;
- retries a build on replacement external builder capacity.

Do not require it for:

- creating and manually running a build on an existing Citadel-managed Local,
  Agent, or Edge Agent platform;
- creating, viewing, testing, updating, disabling, or deleting an external
  build-pool definition;
- viewing external build configuration or historical runs after downgrade;
- cancelling an in-flight external build;
- allowing an in-flight external build to finish.

Build-pool definitions are not counted. The entitlement controls execution
through external build infrastructure, not the number of stored build pools.

## Downgrade and Expiry

During `GracePeriod`, paid capabilities continue without restriction and the UI
must display the upcoming cutoff.

After grace expires or a license is removed:

- do not modify paid resource records or their enabled flags;
- do not delete custom roles, assignments, schedules, webhooks, alert rules, or
  channels;
- allow in-flight operations to finish;
- reject new paid webhook invocations with a stable entitlement error;
- stop enqueueing scheduled paid runs;
- do not start paid-trigger queue items that have not begun;
- do not lease or provision external build capacity for queued builds that have
  not begun;
- allow in-flight external builds to finish or be cancelled;
- pause evaluation and delivery for non-system alert rules;
- continue evaluating seeded system rules and delivering them to configured
  notification channels;
- pause continuous guardrails and auto-update;
- continue evaluating existing custom authorization assignments;
- allow administrators to reduce or remove paid configuration;
- keep manual backup and restore available;
- keep historical logs, alerts, activities, and run records readable.

The UI must describe persisted paid configuration as `Paused by license`, not
`Disabled`, because Citadel must not rewrite the customer's configuration.

Installing a replacement license reactivates eligible persisted configuration
without requiring each resource to be edited.

## License and Entitlement Errors

Capability failures return `403 Forbidden` problem details:

```csharp
public sealed record LicenseCapabilityRequiredError(
    LicenseCapability Capability,
    LicenseStatus LicenseStatus,
    string EffectiveEdition,
    string? LicensedEdition) : Error;
```

Example:

```json
{
  "type": "https://citadel.local/problems/license-capability-required",
  "title": "License capability required",
  "status": 403,
  "detail": "Automated operations require a Team license.",
  "capability": "AutomatedOperations",
  "licenseStatus": "Community",
  "effectiveEdition": "Community",
  "licensedEdition": null
}
```

Map this error in the shared result-to-HTTP layer. Endpoints remain thin.

Do not return upgrade URLs from the backend. Product or deployment-specific
purchase links belong in frontend configuration.

License replacement conflicts return `409 Conflict` with stable problem types:

```text
license-replacement-not-yet-effective
license-replacement-mismatch
```

`license-replacement-not-yet-effective` is returned when a future-dated
license would replace a currently active paid license.

`license-replacement-mismatch` is returned when a license is already installed
and the new payload's `replacedLicenseId` is missing or does not match its
`licenseId`. Neither error may expose the raw installed or submitted license.

## API Contract

Keep the existing route group:

```text
GET    /api/v1/license
GET    /api/v1/license/entitlements
POST   /api/v1/license
DELETE /api/v1/license
GET    /api/v1/license/request
```

Keep route names:

```text
getLicense
getLicenseEntitlements
installLicense
removeLicense
getLicenseRequest
```

Authorization remains:

- every authenticated user may read `/license/entitlements`;
- `ResourceType.License` read for license state and request data;
- `ResourceType.License` write for install or replacement;
- `ResourceType.License` execute for removal;
- built-in Admin has these permissions;
- Operator and Viewer do not receive them by default.

Feature controls outside the License settings page must use the entitlements
endpoint. Do not grant broad License metadata permission merely so a user can
discover whether a workflow is enabled.

### GET `/api/v1/license/entitlements`

Returns the minimum effective state required by feature controls:

```csharp
public sealed record LicenseEntitlementsView(
    LicenseStatus Status,
    string EffectiveEdition,
    IReadOnlyList<LicenseCapabilityView> Capabilities);
```

This endpoint:

- requires authentication but no `ResourceType.License` permission;
- returns only implemented capabilities;
- does not return licensed edition, customer data, identifiers, fingerprints,
  timestamps, warnings, instance ID, or raw license data;
- uses the same effective state provider as backend enforcement and the
  administrator License page.

### GET `/api/v1/license`

Return safe metadata and effective capability state:

```csharp
public sealed record LicenseView(
    LicenseStatus Status,
    string EffectiveEdition,
    string? LicensedEdition,
    string InstanceId,
    int? LicenseSchema,
    string? LicenseId,
    string? ReplacedLicenseId,
    string? CustomerId,
    string? CustomerName,
    string? Fingerprint,
    DateTimeOffset? IssuedAt,
    DateTimeOffset? NotBefore,
    DateTimeOffset? ExpiresAt,
    DateTimeOffset? GraceUntil,
    IReadOnlyList<LicenseCapabilityView> Capabilities,
    IReadOnlyList<string> Warnings);

public sealed record LicenseCapabilityView(
    LicenseCapability Capability,
    bool Enabled);
```

Return only capabilities implemented by the running Core version. Unknown
signed capability keys appear in warnings and must not produce fictional UI
features.

Do not return:

- `RawLicense`;
- resource usage counts;
- former quota limits;
- unimplemented Enterprise capability rows.

### POST `/api/v1/license`

Accept:

```csharp
public sealed record InstallLicenseInput(string License);
```

Behavior:

- trim only surrounding ASCII whitespace;
- reject input larger than 64 KB;
- reject internal whitespace;
- verify before persistence;
- accept validly signed and instance-bound `Valid` and `GracePeriod` schema-1
  or schema-2 licenses;
- accept a `NotYetValid` license only when no currently installed license has
  `Valid` or `GracePeriod` status;
- reject a `NotYetValid` replacement of an active paid license with `409
  Conflict` and a stable `license-replacement-not-yet-effective` problem type;
- when a license is already installed, require the new payload's
  `replacedLicenseId` to match the installed `licenseId`;
- perform the installed-license read, replacement-ID comparison, and upsert in
  one transaction so concurrent replacement requests cannot bypass the check;
- allow a signed replacement with `replacedLicenseId` when no license is
  installed so an issuer-authorized rehost can be activated;
- reject malformed, unknown-key, unsupported-schema, invalid-signature, and
  instance-mismatched licenses;
- atomically replace the existing singleton license;
- invalidate cached state;
- record `LicenseInstalled` or `LicenseReplaced`;
- return the same shape as `GET /api/v1/license`.

Renewal licenses should normally use a `notBefore` that is already effective
when issued. Installing such a renewal immediately extends the active license
without a capability gap. Supporting a future paid license alongside a current
paid license requires separate active-and-pending persistence and is outside
this version of the specification.

### DELETE `/api/v1/license`

Remove the installed license and return Community state.

Deletion is idempotent. Record `LicenseRemoved` only when a license existed.
Removal applies the downgrade rules in this specification.

### GET `/api/v1/license/request`

Return only:

```csharp
public sealed record LicenseRequestView(
    string Product,
    string InstanceId,
    string CoreVersion,
    DateTimeOffset GeneratedAt);
```

Do not include secrets, user data, host identifiers, network addresses,
platform inventory, or resource counts.

## Activity Events

Retain:

```csharp
LicenseInstalled
LicenseReplaced
LicenseRemoved
LicenseEnteredGracePeriod
LicenseExpired
LicenseValidationFailed
```

Use a safe snapshot:

```csharp
public sealed record LicenseActivitySnapshot(
    int? Schema,
    string? LicenseId,
    string? ReplacedLicenseId,
    string? LicensedEdition,
    string EffectiveEdition,
    IReadOnlyList<LicenseCapability> EffectiveCapabilities,
    string? CustomerId,
    string? CustomerName,
    string? Fingerprint,
    LicenseStatus Status,
    DateTimeOffset? ExpiresAt,
    DateTimeOffset? GraceUntil);
```

Never include the raw license.

The transition monitor records an event only when persisted validation status
changes. It must not emit repeated expiry or grace-period events on every state
read. Entering an inactive state should also create one administrator-visible
warning that identifies which configured workflows are paused.

## Frontend

Keep the License page under Settings.

Replace the quota-oriented interface with:

- effective edition and status;
- licensed edition when it differs from the effective edition;
- instance ID and safe metadata;
- one row per shipped licensable capability;
- `Included`, `Not included`, or `Paused by license` state;
- an explanation of what each capability controls;
- legacy-schema warning for active Business licenses;
- install, replace, remove, and license-request actions.

Remove:

- `Quota Usage`;
- progress bars;
- `current / maximum` labels;
- over-quota labels;
- `LICENSE_LIMIT_LABELS`;
- entity-count language from confirmation dialogs and toasts.

Paid controls should be visible where discovery is useful, but disabled with a
short edition explanation before the user fills a form. Direct navigation and
API calls must still be rejected by backend entitlement checks.

Locked-feature indicators are presentation metadata:

- the icon and edition text communicate the normal minimum commercial package;
- control availability is derived from the required capability, not from the
  displayed edition;
- all currently shipped paid controls display `Team`;
- the shared indicator may support `Enterprise` structurally, but no production
  call site may display it until an Enterprise-only capability and feature are
  implemented;
- an indicator must not imply that every license with that edition contains the
  capability; the License page is authoritative for the installed license.

Use one shared frontend entitlements query with a one-hour stale interval for
feature controls. The administrator License page may additionally request the
full permission-protected license view. Explicitly invalidate the entitlements
query after install, removal, authentication changes, a
`LicenseStateChanged` transition notification, and window focus. The background
transition monitor must publish the notification close to `notBefore`,
`expiresAt`, and `graceUntil`; the one-hour interval is only a fallback.
Individual components must not independently poll either license endpoint.

When the backend returns `license-capability-required`, show its problem detail
and identify the required edition. Do not show a generic mutation failure.

## Issuer Tool

The issuer remains outside the public repository.

Responsibilities:

```text
citadel-license keys generate
citadel-license license issue
citadel-license license inspect
citadel-license license verify
```

Keep:

- encrypted PKCS#8 PEM private keys;
- SubjectPublicKeyInfo PEM public keys;
- key IDs and embedded public-key rotation;
- overwrite protection;
- private-key output suppression;
- post-generation key-pair verification;
- a private issuance registry.

Issue a Team license:

```powershell
citadel-license license issue `
  --request ./customer-license-request.json `
  --customer-id customer-example `
  --customer-name "Example Corp" `
  --subscription-ref subscription-example `
  --environment Production `
  --edition Team `
  --bundle team `
  --expires 2027-07-12 `
  --grace-days 14 `
  --key-id citadel-license-2026-01 `
  --private-key ./keys/citadel-license-2026-01.private.pem `
  --out ./licenses/example-2027.citadel-license
```

The canonical `team` bundle contains every shipped Team capability. The issuer
may also accept repeated explicit capability arguments for controlled
design-partner issuance:

```text
--capability custom-access-control
--capability automated-operations
```

Issuer requirements:

- issue schema 2 only;
- validate the request and instance ID;
- generate a unique license ID;
- validate edition and every capability key;
- reject duplicate or unknown capability keys;
- refuse unimplemented capability keys;
- refuse general Enterprise issuance until Enterprise is released;
- include `replacedLicenseId` only with an explicit `--replaces` argument;
- require the issuance registry to contain the license named by `--replaces`;
- issue ordinary renewals with an immediately effective `notBefore` so they can
  replace an active license without a capability gap;
- sign with `alg = Ed25519` and `typ = citadel-license+jws`;
- reopen and verify generated output;
- calculate and store the fingerprint;
- print only safe metadata.

The private issuance registry stores:

- commercial subscription reference and environment classification
  (`Production` or `NonProduction`);
- schema;
- license ID and replaced license ID;
- customer ID and name;
- instance ID;
- edition;
- capabilities;
- key ID;
- issue, activation, expiration, and grace timestamps;
- fingerprint;
- output filename.

The issuer may inspect and verify schema-1 licenses but must not create new
schema-1 Business licenses.

## Migration Plan

Implement in this order:

1. Add schema-2 payload contracts, `LicenseCapability`, stable key mapping, and
   Team/Enterprise edition validation.
2. Extend the verifier to accept schema 1 and schema 2.
3. Add the schema-1 Business-to-Team compatibility adapter and tests.
4. Replace quota state with `EffectiveEdition`, `LicensedEdition`, and effective
   capabilities.
5. Add `ILicenseEntitlementService`.
6. Remove platform, user, OIDC provisioning, backup-policy-count,
   automation-action-count, and custom-role-count quota checks.
7. Add capability checks at the enforcement boundaries defined above.
8. Add trigger provenance where workers need to distinguish manual and paid
   automated runs.
9. Apply downgrade behavior to schedulers, webhook entry points, alert
   evaluation, guardrail jobs, external build dispatch, and custom
   access-control mutations.
10. Add the time-boundary transition monitor and metadata-free
    `LicenseStateChanged` notification.
11. Add the minimal authenticated entitlements endpoint, update the
    administrator API view, and map stable problem details.
12. Regenerate OpenAPI and frontend API clients.
13. Replace the frontend quota UI with capability state.
14. Update the private issuer to emit schema 2.
15. Add unit, integration, API, and browser coverage.
16. Deploy the dual-schema Core before issuing schema-2 licenses.

No database migration should be generated unless implementation discovers a
real persistence change, such as missing trigger provenance. The stored license
payload itself does not require one.

## Tests

### Verifier

- Valid schema-2 Team license verifies.
- Valid schema-2 Enterprise license verifies without inventing unimplemented
  capabilities.
- Unknown `kid` returns `UnknownSigningKey`.
- Tampered payload returns `Invalid`.
- `alg = none`, `alg = EdDSA`, and every algorithm except `Ed25519` are
  rejected.
- Wrong type, product, issuer, or audience is rejected.
- Unsupported schema returns `UnsupportedSchema`.
- Instance mismatch returns `InstanceMismatch`.
- Two licenses issued under one Team subscription remain independently bound to
  their production and non-production instance IDs.
- Future not-before returns `NotYetValid`.
- Expired inside grace returns `GracePeriod`.
- Expired outside grace returns `Expired`.
- Unknown capability keys are ignored with warnings.
- Missing capability keys remain disabled.
- Duplicate capability keys are rejected.
- A schema-1 Business license remains valid and maps to Team capabilities.
- Invalid schema-1 limits do not bypass legacy validation.

### Community Baseline

- Community can create more than five platforms.
- Community can create more than ten active users.
- Community can create teams and assign built-in roles.
- Community can create more than five backup policy definitions.
- Community can create more than fifteen automation action definitions.
- Community can use existing OIDC provisioning without a user quota.
- Community can manually run automation actions.
- Community can manually run and restore backups.
- Community can receive and authenticate Git, deployment, and build webhooks,
  synchronize their source state, and display pending updates.
- Community webhooks cannot automatically build, deploy, apply, run an action,
  or start a backup.
- Community can manually build on Local, Agent, and Edge Agent platforms.

### Capability Enforcement

- Community cannot create or assign a custom role.
- Community can remove an existing custom-role assignment after downgrade.
- Existing custom-role authorization continues after downgrade.
- Community cannot enable automation schedules or automation webhooks.
- Community cannot enable backup schedules or backup webhooks.
- Community cannot configure a webhook or schedule that automatically starts a
  build, deployment, or stack apply.
- Community cannot enable `Redeploy On Build` or automatically apply a
  successful build artifact.
- Community can receive a Git webhook and record an available update without
  automatically applying it.
- Community can record a successful desired build artifact and apply it
  manually.
- Community manual automation and backup runs remain available.
- Community can create, test, update, and delete notification channels.
- Community can route seeded system rules to external channels.
- Community cannot create custom rules or change advanced rule conditions.
- Community continues receiving built-in in-app and external failure alerts.
- Community cannot enable continuous guardrails or auto-update.
- Community cannot execute a build through an external build pool.
- Community can create, inspect, test, update, disable, and delete external
  build-pool definitions without executing a build through them.
- Community can inspect and cancel existing external build work after
  downgrade.
- Team can perform every operation covered by its signed capabilities.
- A Team license missing one capability cannot use that capability.
- Direct API calls cannot bypass frontend locks.

### Transition and Recovery

- Grace-period capabilities remain enabled.
- Expiry pauses scheduled and webhook-triggered paid workflows.
- In-flight backup, restore, automation, reconciliation, and external build
  operations finish.
- Paid queue items not yet started are rejected after expiry.
- External builder capacity is not provisioned for paid queue items rejected
  after expiry.
- Manual queue items remain executable.
- Existing paid configuration is not rewritten.
- Installing a replacement license reactivates persisted configuration.
- A future-dated license cannot replace a currently active license.
- A replacement with a missing or mismatched `replacedLicenseId` is rejected
  when another license is installed.
- A signed rehost replacement may be installed on its bound new instance when
  no license is currently installed there.
- An immediately effective renewal replaces an active license without a
  Community capability gap.
- Expiry pauses non-system alert rules but preserves notification channels and
  external delivery from seeded system rules.
- Restore remains available with no license and with an invalid license.
- Historical runs, logs, alerts, and activities remain readable.
- Transition events are emitted once.

### API

- License endpoints require the correct `ResourceType.License` permissions.
- The entitlements endpoint requires authentication but not
  `ResourceType.License` permission.
- The entitlements endpoint exposes no administrative license metadata.
- `GET /api/v1/license` returns effective and licensed editions.
- Only shipped capabilities are returned.
- No license response exposes `RawLicense`.
- No license response returns former usage or limit fields.
- Capability errors map to stable 403 problem details.
- Future-dated and replacement-ID conflicts map to stable 409 problem details.
- Schema-1 and schema-2 installs return the same API shape.
- Route names remain stable for client generation.

### Frontend

- Community renders shipped capabilities as not included.
- Team renders only effective capabilities as included.
- Enterprise renders only its explicitly signed, known capabilities as
  included; the edition does not imply capability inheritance.
- Expired Team renders paid configuration as paused.
- Legacy Business displays a renewal warning and effective Team edition.
- Quota progress bars and `current / maximum` labels are absent.
- Locked controls explain the required capability before form submission.
- Current paid controls use the Team indicator; no Enterprise indicator is
  rendered without an implemented Enterprise-only capability.
- Direct navigation cannot bypass backend enforcement.
- Install, removal, window focus, and `LicenseStateChanged` invalidate the
  shared license query.

## Acceptance Criteria

- Citadel operates normally without a license.
- Community has no license-enforced platform, user, backup-policy, automation,
  or other resource-count quotas.
- Community retains core authentication, MFA, OIDC, API, manual backup, restore,
  rollback, and data-access behavior.
- Team capabilities are enforced at backend workflow boundaries.
- Webhook reception and update detection remain Community while
  webhook-triggered mutation requires `AutomatedOperations`.
- External build-pool execution requires `ElasticBuildExecution`; ordinary
  builds on existing managed platforms remain Community.
- Existing schema-1 Business licenses continue working as Team licenses.
- New licenses use schema 2 and explicit capability keys.
- Team and Enterprise licenses enable only known capabilities explicitly
  present in their signed payloads.
- No Enterprise-only runtime check or frontend indicator exists until its
  corresponding feature and capability are implemented.
- Invalid, expired, unknown-key, and instance-mismatched licenses do not enable
  paid capabilities.
- Grace-period licenses retain capabilities until `graceUntil`.
- Downgrade preserves configuration, authorization safety, history, and
  recovery access.
- No API, activity, log, or UI output exposes the raw license.
- The License page describes capabilities and contains no quota progress UI.
- SCIM, high availability, immutable audit delivery, and compliance reporting
  are not presented as implemented Enterprise features.
- Unit, integration, API, and browser tests cover entitlement enforcement,
  transition behavior, and schema-1 compatibility.
