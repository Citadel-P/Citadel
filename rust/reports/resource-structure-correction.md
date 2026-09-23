# Resource structure correction and domain ownership

Status: implementation and validation complete.

## Scope and structure

The user requested one feature resource convention and explicitly requested that
`crates/domain` be renamed or refactored. This corrects Phase 10 and brings forward
the domain portion of Phase 11; it does not claim completion of all Phase 11 work.

- Removed Identity's `src/domain/` and `src/application/` layers. Users, teams,
  roles, service accounts, actors, profile, authentication, MFA and OIDC now have
  their own models beside commands, read models, repositories and services.
  Authorization evaluation lives in `permissions.rs`; the shared typed error is
  in `error.rs`. Session/MFA/OIDC data types are in model/command/read-model roles,
  and service-account usage ports live under `service_accounts/usage.rs`.
- Checked Deployments, Stacks, Swarm Services, Git, Builds, Backups, Automation,
  Alerts, Platforms and Resources. They already use resource/role namespaces,
  with large roles split into directories. Replaced wildcard resource exports
  encountered in Identity, Resources and Stack models with explicit exports.
  No empty per-resource service/repository files were created.
- Removed the `citadel-domain` package. Added `citadel-primitives`,
  `citadel-activities` and `citadel-licensing` with model ownership described below.
  Audit snapshots are grouped by event family, separate from the envelope,
  event information, invariants, vocabulary and change/source metadata.
- Moved identity enums, lookup vocabulary and Swarm ownership classifications
  to their feature owners. Updated imports, manifests, lockfile, test paths and
  the PowerShell deployment-validation script. No domain compatibility package
  or alias remains.
- Updated `ARCHITECTURE.md` and the authoritative external specification to remove
  the Identity structural exception and record the user-authorized phase ordering.

## Behavior and dependency decisions

Security services, persistence transactions, cryptography, ACL policies and token
handling retain their implementations. Audit role/setup/preference values are mapped
at the emission boundary to the same serialized strings as before, avoiding an
Activities → Identity → Activities dependency cycle. No database migration is needed.
Activities owns historical audit records rather than live resource aggregates.

Identity and Resources do not acquire Utoipa dependencies: server-owned vocabulary
schema descriptors preserve the previous OpenAPI names and enum values. Existing
schema derives on shared permission primitives and licensing/audit vocabulary are
retained with their moved definitions; this correction does not claim to complete
the remaining Phase 11 presentation cleanup.

`citadel-application` still contains activity/licensing services and process support.
Those remaining Phase 11 moves are documented in `ARCHITECTURE.md`; the domain models
already have their final owners. Process lifecycle auditing remains Phase 12.

## Former domain public symbols

Every former public definition is listed with its implemented destination. Narrow
primitives have no feature dependency; Activities depends on primitives and Licensing
models, while Licensing models depend only on primitives. Identity owns security
vocabulary, Discovery owns lookup vocabulary, and Swarm Services owns its ownership
classification.

| Former module and symbol | Implemented path |
|---|---|
| `domain/src/authorization.rs` · `PermissionRequirement` | `crates/primitives/src/authorization.rs` |
| `domain/src/authorization.rs` · `PermissionPolicy` | `crates/primitives/src/authorization.rs` |
| `domain/src/authorization.rs` · `SpecificPermissions` | `crates/primitives/src/authorization.rs` |
| `domain/src/authorization.rs` · `EffectivePermission` | `crates/primitives/src/authorization.rs` |
| `domain/src/authorization.rs` · `permission_policy` | `crates/primitives/src/authorization.rs` |
| `domain/src/enums.rs` · `SetupInitializationMode` | `crates/identity/src/authentication/model.rs` |
| `domain/src/enums.rs` · `LookupResourceType` | `crates/discovery/src/lookup/model.rs` |
| `domain/src/enums.rs` · `ActivityStatus` | `crates/activities/src/model/vocabulary.rs` |
| `domain/src/enums.rs` · `LicenseCapability` | `crates/licensing/src/model/vocabulary.rs` |
| `domain/src/enums.rs` · `LicenseStatus` | `crates/licensing/src/model/vocabulary.rs` |
| `domain/src/enums.rs` · `MfaPolicy` | `crates/identity/src/mfa/model.rs` |
| `domain/src/enums.rs` · `ActivityResourceType` | `crates/activities/src/model/vocabulary.rs` |
| `domain/src/enums.rs` · `ActivityEventType` | `crates/activities/src/model/vocabulary.rs` |
| `domain/src/enums.rs` · `ActorType` | `crates/identity/src/actors/model.rs` |
| `domain/src/enums.rs` · `AuthenticatedPrincipalType` | `crates/identity/src/actors/model.rs` |
| `domain/src/enums.rs` · `ResourceType` | `crates/primitives/src/permissions.rs` |
| `domain/src/enums.rs` · `SwarmServiceOwnership` | `crates/swarm-services/src/model/ownership.rs` |
| `domain/src/enums.rs` · `PermissionLevel` | `crates/primitives/src/permissions.rs` |
| `domain/src/enums.rs` · `SpecificPermission` | `crates/primitives/src/permissions.rs` |
| `domain/src/enums.rs` · `RoleType` | `crates/identity/src/roles/model.rs` |
| `domain/src/enums.rs` · `UserDateTimeFormat` | `crates/identity/src/profile/model.rs` |
| `domain/src/enums.rs` · `UserTheme` | `crates/identity/src/profile/model.rs` |
| `domain/src/lib.rs` · `is_sensitive_environment_name` | `crates/primitives/src/redaction.rs` |
| `domain/src/lib.rs` · `ActorId` | `crates/primitives/src/actor.rs` |
| `domain/src/license.rs` · `LICENSE_PRODUCT` | `crates/licensing/src/model/resource.rs` |
| `domain/src/license.rs` · `LICENSE_ISSUER` | `crates/licensing/src/model/resource.rs` |
| `domain/src/license.rs` · `LICENSE_AUDIENCE` | `crates/licensing/src/model/resource.rs` |
| `domain/src/license.rs` · `COMMUNITY_EDITION` | `crates/licensing/src/model/resource.rs` |
| `domain/src/license.rs` · `TEAM_EDITION` | `crates/licensing/src/model/resource.rs` |
| `domain/src/license.rs` · `LEGACY_BUSINESS_EDITION` | `crates/licensing/src/model/resource.rs` |
| `domain/src/license.rs` · `LEGACY_LICENSE_SCHEMA` | `crates/licensing/src/model/resource.rs` |
| `domain/src/license.rs` · `CURRENT_LICENSE_SCHEMA` | `crates/licensing/src/model/resource.rs` |
| `domain/src/license.rs` · `CitadelInstanceIdentity` | `crates/licensing/src/model/resource.rs` |
| `domain/src/license.rs` · `InstalledLicense` | `crates/licensing/src/model/resource.rs` |
| `domain/src/license.rs` · `LicenseCustomer` | `crates/licensing/src/model/resource.rs` |
| `domain/src/license.rs` · `LicensePayload` | `crates/licensing/src/model/resource.rs` |
| `domain/src/license.rs` · `VerifiedLicense` | `crates/licensing/src/model/resource.rs` |
| `domain/src/license.rs` · `LicenseVerificationResult` | `crates/licensing/src/model/resource.rs` |
| `domain/src/license.rs` · `LicenseState` | `crates/licensing/src/model/resource.rs` |
| `domain/src/activity.rs` · `BuildProjectActivitySnapshot` | `crates/activities/src/model/build.rs` |
| `domain/src/activity.rs` · `BuildSecretActivitySnapshot` | `crates/activities/src/model/build.rs` |
| `domain/src/activity.rs` · `AutomationActionActivitySnapshot` | `crates/activities/src/model/automation.rs` |
| `domain/src/activity.rs` · `BuildAgentPoolActivitySnapshot` | `crates/activities/src/model/build.rs` |
| `domain/src/activity.rs` · `IdentityResourceAccessSnapshot` | `crates/activities/src/model/identity.rs` |
| `domain/src/activity.rs` · `UserActivitySnapshot` | `crates/activities/src/model/identity.rs` |
| `domain/src/activity.rs` · `TeamActivitySnapshot` | `crates/activities/src/model/identity.rs` |
| `domain/src/activity.rs` · `RolePermissionActivitySnapshot` | `crates/activities/src/model/identity.rs` |
| `domain/src/activity.rs` · `RoleActivitySnapshot` | `crates/activities/src/model/identity.rs` |
| `domain/src/activity.rs` · `ServiceAccountResourceAccessSnapshot` | `crates/activities/src/model/identity.rs` |
| `domain/src/activity.rs` · `ServiceAccountActivitySnapshot` | `crates/activities/src/model/identity.rs` |
| `domain/src/activity.rs` · `LicenseActivitySnapshot` | `crates/activities/src/model/license.rs` |
| `domain/src/activity.rs` · `OidcProviderActivitySnapshot` | `crates/activities/src/model/identity.rs` |
| `domain/src/activity.rs` · `RegistryActivitySnapshot` | `crates/activities/src/model/registry.rs` |
| `domain/src/activity.rs` · `GitRepositoryActivitySnapshot` | `crates/activities/src/model/git.rs` |
| `domain/src/activity.rs` · `GitRepositorySyncActivitySnapshot` | `crates/activities/src/model/git.rs` |
| `domain/src/activity.rs` · `DeploymentActivitySnapshot` | `crates/activities/src/model/deployment.rs` |
| `domain/src/activity.rs` · `DeploymentResultActivitySnapshot` | `crates/activities/src/model/deployment.rs` |
| `domain/src/activity.rs` · `SwarmServiceActivitySnapshot` | `crates/activities/src/model/swarm.rs` |
| `domain/src/activity.rs` · `StackActivitySnapshot` | `crates/activities/src/model/stack.rs` |
| `domain/src/activity.rs` · `StackResultActivitySnapshot` | `crates/activities/src/model/stack.rs` |
| `domain/src/activity.rs` · `PlatformActivitySnapshot` | `crates/activities/src/model/platform.rs` |
| `domain/src/activity.rs` · `ActivitySourceResource` | `crates/activities/src/model/sources.rs` |
| `domain/src/activity.rs` · `AlertRuleActivitySnapshot` | `crates/activities/src/model/alert.rs` |
| `domain/src/activity.rs` · `ActivityChangedFieldName` | `crates/activities/src/model/changes.rs` |
| `domain/src/activity.rs` · `ActivityChangedField` | `crates/activities/src/model/changes.rs` |
| `domain/src/activity.rs` · `WebhookActivityDetails` | `crates/activities/src/model/webhooks.rs` |
| `domain/src/activity.rs` · `WebhookActivitySource` | `crates/activities/src/model/webhooks.rs` |
| `domain/src/activity.rs` · `VolumeContentDownloaded` | `crates/activities/src/model/sources.rs` |
| `domain/src/activity.rs` · `BackupPolicyActivitySnapshot` | `crates/activities/src/model/backup.rs` |
| `domain/src/activity.rs` · `ActivityEventInfo` | `crates/activities/src/model/info.rs` |
| `domain/src/activity.rs` · `ActivityEvent` | `crates/activities/src/model/event.rs` |
| `domain/src/activity.rs` · `ActivityInvariantError` | `crates/activities/src/model/event.rs` |

## Validation

- Formatting and `git diff HEAD --check`: passed.
- Strict workspace Clippy (`--locked --workspace --all-targets -- -D warnings`): passed.
- Workspace tests: 660 passed, 0 failed, 239 external tests ignored by default.
  The two new structure guards passed; all existing test names were retained.
- `cargo run --locked -p xtask -- openapi --check`: passed (404 full and 305
  public operations). Both schema documents and generated frontend contracts
  match the existing artifacts; no schema artifact was regenerated.
- Explicit PostgreSQL-backed runs: 160 passed, 0 failed in 15 suites, each using
  a separate disposable database. Of the 239 tests skipped by the default workspace
  run, 79 unrelated external tests remain unrun.

| Database-backed suite | Passed |
|---|---:|
| `identity_access` | 1 |
| `identity_authorization_differential` | 1 |
| `licenses_http` | 8 |
| `mfa_http` | 1 |
| `oidc_http` | 1 |
| `phase7_resources_http` | 1 |
| `platform_creation_http` | 13 |
| `platform_inventory_persistence` | 18 |
| `platforms_http` | 62 |
| `resource_metadata_persistence` | 2 |
| `resources_http` | 1 |
| `roles_http` | 7 |
| `service_accounts_http` | 6 |
| `teams_http` | 10 |
| `users_http` | 28 |

These cover authentication/session/MFA/OIDC semantics, administrator protection,
ACL evaluation, service-account lifecycle, safe audit persistence, licensing,
Platform registration/runtime endpoints, inventories and shared-resource metadata.
Docker protocol tests use fixtures; this is not a live workload or external secret-
provider integration certification. No HTTP/schema or persisted audit JSON behavior
change was found. Internal Rust import paths change to the owning crate/resource.

Validation disabled debug information and incremental compilation through Cargo
profile environment overrides to limit disk use. The disposable PostgreSQL container
and its databases were removed after the successful run.

## Exemptions

Removed five obsolete domain façade/umbrella exemptions from the external Phase 0/1
audits. Updated the paths of the two existing password-task exceptions to
`identity/src/authentication/service.rs`; they and the existing Container mutation
task exception remain due in Phase 12. No structural Identity exception remains,
and no new runtime or architecture exemption was added.

Helper scripts, logs and backups are outside Git in `/tmp/citadel-structure-fix`.


## Shared resource ownership correction — 2026-09-20

The user authorized replacing the broad Resources owner with explicit feature crates.
`citadel-resources` is removed, without a compatibility crate. Its owners are now:

- `citadel-tags`: Tag models, validation and `TagRepository`.
- `citadel-registries`: Registry configuration, browsing models and `RegistryRepository`.
- `citadel-bindings`: bindings, secret definitions/providers, `BindingRepository`,
  `SecretService` and `SecretProtector`. Secret-reference cleanup stays transactional.
- `citadel-discovery`: permission-filtered lookup and global search Reader contracts.

`citadel-primitives` retains actor/permission/redaction vocabulary. It does not gain
feature services or repositories. Registry projections consume Tags; Git webhook
consumers use Git directly. Platform description mutation now uses its own Platform
metadata port and PostgreSQL implementation. Server feature DTOs, HTTP state and
composition use the explicit owners. Small metadata-patch types have explicit
server-to-feature conversions, preserving missing/null/value behavior.

All 25 moved Tag/Registry/Binding repository methods preserve their transaction
bodies. Tag SQL remains callable within Registry and Git transactions, with owner-specific
error conversion. Existing audit insertion, row locking and commit boundaries remain.
No architecture exemption was added. The ownership guards reject an umbrella crate,
feature-layer HTTP concerns and the previous dependency coupling.

Validation for this correction:

- `cargo test --locked --workspace`: 661 passed, 0 failed, 239 ignored.
- 15 separately enabled PostgreSQL suites: 160 passed, 0 failed. These include
  tag concurrency, atomic metadata/secret lifecycle, HTTP authorization, lookup,
  Platform metadata/inventory and the shared authorization matrix.
- Strict workspace Clippy, all targets: passed.
- `cargo fmt --all -- --check` and full working-tree whitespace check: passed.
- `xtask openapi --check`: exact schema/generated frontend compatibility retained;
  404 full and 305 public operations. No contract baselines were regenerated.

The disposable PostgreSQL fixture was isolated from the user's running services.
This does not claim execution of live external-provider/Docker acceptance fixtures.
The remaining `citadel-application` Phase 11 cleanup and Phase 12 runtime work are unchanged.
