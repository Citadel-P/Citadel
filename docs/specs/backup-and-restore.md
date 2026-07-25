# Citadel Backup and Restore

This document is the parent specification for Citadel Backup and Restore.

Implement it one slice at a time.

If no slice is explicitly requested:

1. Implement **Slice 1 only**.
2. Compare the specification against the current codebase before modifying files.
3. Reuse existing Citadel patterns instead of creating parallel abstractions.
4. Run relevant tests, formatting, OpenAPI generation, and frontend API generation.
5. Report:

   * implemented behavior;
   * migrations and generated artifacts;
   * tests added or updated;
   * deferred work;
   * any specification conflicts discovered in the codebase.
6. Do not continue automatically into the next slice.

Do not partially implement functionality belonging to later slices unless it is required to keep the current slice compilable.

---

# 1. Goal

Add first-class backup and restore support to Citadel.

The feature protects:

* Docker named volumes managed through Citadel Platforms.
* The Citadel control plane itself:

  * PostgreSQL logical database dump.
  * Persisted security and recovery assets stored outside PostgreSQL.

The feature uses Restic for:

* encrypted backup repositories;
* snapshots;
* incremental data storage;
* retention;
* repository checks;
* repository pruning;
* restore.

Backups are first-class Citadel resources.

They are not raw Automation scripts.

Automation Actions may invoke backup APIs, but Automation Actions must not gain:

* raw filesystem access;
* Docker socket access;
* direct Restic execution;
* access to backup staging files;
* access to backup credentials.

---

# 2. Citadel Alignment

Follow existing Citadel conventions.

## Backend

* Public API routes live under `/api/v1`.
* Follow the current camel-case resource route convention:

  * `/backupRepositories`
  * `/backupPolicies`
  * `/backupRuns`
  * `/backupRestoreRuns`
* Register routes from:

  * `src/Citadel.WebApi/Routes/PublicEndpoints.cs`
* Keep endpoint methods thin.
* Delegate application behavior through Mediator commands and queries.
* Split commands, handlers, queries, and results into focused files.
* Use `Result` for expected domain and validation failures.
* Use exceptions only for unexpected failures.
* Use `TimeProvider`.
* Do not use `DateTime.UtcNow`.
* PATCH endpoints use JSON Merge Patch where partial updates are required.
* Add API request and response models to:

  * `ApplicationJsonContext`
* Add polymorphic domain JSON models to:

  * `DomainJsonContext`
* Use Dapper repositories through `IUnitOfWork`.
* Preserve Dapper AOT compatibility.
* Pass anonymous parameter objects directly to Dapper.
* Do not introduce custom Dapper parameter wrapper records.
* Model schema changes in `ApplicationDbContext`.
* Generate EF migrations and generated SQL scripts.
* Do not manually write scripts under `Infrastructure/Scripts`.

## Long-running work

* Do not execute long-running backups inside the original HTTP request.
* Queue persisted runs.
* Execute runs through background workers.
* Long-running services use `IServiceScopeFactory`.
* Do not retain an injected `IUnitOfWork` for the duration of a backup.
* Use short database work items.
* Use `IDbWorkQueue` where it matches current write-serialization behavior.
* Use the same persisted run path for:

  * manual execution;
  * scheduled execution;
  * Automation-triggered execution.

## Frontend

* Use existing resource page shells.
* Use current form and table patterns.
* Use capability-driven actions.
* Use `StateIndicator`.
* Use existing tag filtering.
* List filters use `tags`, not `tagIds`.
* Do not issue one frontend coverage request per row.
* Resource coverage is loaded through existing response models or one bulk query.

---

# 3. Product Terminology

Use these UI terms:

* `Backups`
* `Backup Policy`
* `Destination`
* `Backup Run`
* `Restore`

Use these domain terms:

* `BackupRepository`
* `BackupRepositoryValidation`
* `BackupPolicy`
* `BackupRun`
* `BackupRestoreRun`

The UI calls a Restic repository a **Destination** to avoid confusion with Git repositories.

---

# 4. Fixed Product Decisions

1. Restic is the backup repository engine.
2. PostgreSQL is backed up using `pg_dump`.
3. Do not archive the live PostgreSQL Docker volume.
4. Citadel system restore is offline only.
5. Docker-volume data is processed near the source.
6. Agent and Edge Agent backup data must never pass through Core.
7. Docker-volume restore creates a new volume by default.
8. Overwrite restore is explicitly destructive.
9. Restore operations are never blocked by licensing.
10. Backups are queued operations.
11. The original run-start HTTP request returns after queuing.
12. Repository passwords must also be stored outside Citadel by the administrator.
13. Source and destination identity become immutable after the first successful policy snapshot.
14. Policy and destination deletion use archival semantics.
15. Backup events primarily belong to Backup Policy or Backup Repository resources.
16. Resource pages may show backup run history without duplicating activity events across every related resource.

---

# 5. Scope

## In scope

* Backup destinations:

  * filesystem;
  * S3-compatible.
* Destination lifecycle:

  * create;
  * update;
  * archive;
  * validate;
  * initialize;
  * check;
  * prune.
* Backup policies:

  * create;
  * update;
  * archive;
  * enable;
  * disable.
* Backup execution:

  * manual;
  * schedule;
  * Automation Action.
* Docker named-volume backup.
* Docker named-volume restore.
* Citadel system backup.
* Offline Citadel system recovery.
* Retention using `KeepLastSuccessful`.
* Run history.
* Sanitized logs.
* Warning and failure statuses.
* Repository and source operation leases.
* Cancellation.
* Crash reconciliation.
* Local, Agent, and Edge Agent Platforms.
* Backup visibility on:

  * Volume list and detail;
  * Platform detail;
  * Deployment detail;
  * Stack detail;
  * Container detail.
* Limited Platform-list summary without remote volume inspection.

## Out of scope

* Bind-mount backup.
* Arbitrary host-directory backup.
* Kubernetes persistent volumes.
* Docker Swarm-native backup.
* User-database-aware backup.
* PostgreSQL workload dumps.
* MySQL or MariaDB dumps.
* Application-consistent multi-volume backup.
* SMB or NFS mount management.
* SFTP.
* Azure Blob.
* Google Cloud Storage native integration.
* rclone.
* Cross-repository replication.
* Continuous data protection.
* Arbitrary pre-backup or post-backup shell hooks.
* Browser-based Citadel system restore.
* Restoring Citadel while Core is running.
* Container-list backup coverage.
* Cross-resource duplicated activity events.
* Stack as an independent backup source.
* Deployment as an independent backup source.
* Backup-policy webhooks.

---

# 6. Licensing

No backup-specific licence limits are required for the initial implementation.

Future licensing may restrict:

* creating a new backup policy;
* enabling a disabled backup policy;
* creating S3 destinations;
* advanced retention;
* advanced checks.

Licensing must never block:

* viewing existing backup history;
* viewing backup configuration;
* cancelling a running backup;
* restoring an existing Docker-volume snapshot;
* using the offline Citadel recovery tool.

---

# 7. High-Level Architecture

```text
                         Citadel Core
                              |
             +----------------+----------------+
             |                                 |
        CitadelSystem                    DockerVolume
             |                                 |
       Core executor                Platform connector
             |                    Local / Agent / Edge
             |                                 |
 pg_dump + recovery assets             Helper container
             |                                 |
             +--------------+------------------+
                            |
                      Restic repository
                            |
                +-----------+-----------+
                |                       |
          Filesystem / NAS        S3-compatible
```

Rules:

* Citadel Core authorizes, schedules, records, and monitors work.
* Citadel system backups execute in Core.
* Volume backups execute through the Platform connector.
* Volume backup bytes do not stream through Core.
* The execution node communicates directly with the destination.
* Repository credentials are resolved only during execution.
* Credentials are never stored in run payloads or logs.

---

# 8. Packaging

## Core image

Include:

* Restic.
* `pg_dump`.
* `pg_restore`.
* CA certificates.

## Shared helper image

Use and pin the Citadel Core image for platform-scoped backup and volume-browser helper containers:

```text
ghcr.io/citadel-p/citadel:<release-version>
```

Include:

* Restic.
* The `Citadel.VolumeHelper` executable.
* CA certificates.

Do not include:

* Docker socket.
* compilers;
* package managers;
* unnecessary administration tools.

## Recovery image or distribution

Include:

* Restic.
* `pg_restore`.
* PostgreSQL client tools.
* CA certificates.
* `Citadel.Recovery`.

## Agent and Edge Agent images

Agents orchestrate helper containers on their target Docker daemon.

Restic does not need to be installed directly in Agent or Edge Agent images when all Platform-scoped Restic operations execute inside the helper container.

## Build-time verification

Verify:

```text
restic version
pg_dump --version
pg_restore --version
```

Expose bundled versions in application diagnostics.

Do not download binaries at runtime.

---

# 9. Source Types

```csharp
public enum BackupSourceType
{
    DockerVolume,
    CitadelSystem
}
```

Source specification:

```csharp
[JsonPolymorphic(TypeDiscriminatorPropertyName = "$type")]
[JsonDerivedType(typeof(DockerVolumeBackupSource), "DockerVolume")]
[JsonDerivedType(typeof(CitadelSystemBackupSource), "CitadelSystem")]
public abstract record BackupSourceSpec;
```

```csharp
public sealed record DockerVolumeBackupSource(
    Guid PlatformId,
    string VolumeName,
    VolumeBackupConsistency Consistency)
    : BackupSourceSpec;
```

```csharp
public sealed record CitadelSystemBackupSource()
    : BackupSourceSpec;
```

```csharp
public enum VolumeBackupConsistency
{
    Live,
    StopAttachedContainers
}
```

Rules:

* `Live` is the default.
* Live backup is crash-consistent at best.
* The UI must display a warning for live backup.
* `StopAttachedContainers` improves filesystem quiescence but does not guarantee application-aware database consistency.
* Volume names are normalized and compared using Docker's effective case-sensitive name semantics.
* Bind mounts and anonymous volumes are not valid sources.

Stable source key:

```text
DockerVolume:
  <platform-id>:<volume-name>

CitadelSystem:
  citadel-system
```

---

# 10. Destination Types

```csharp
public enum BackupRepositoryType
{
    FileSystem,
    S3Compatible
}
```

```csharp
[JsonPolymorphic(TypeDiscriminatorPropertyName = "$type")]
[JsonDerivedType(typeof(FileSystemBackupRepositorySpec), "FileSystem")]
[JsonDerivedType(typeof(S3CompatibleBackupRepositorySpec), "S3Compatible")]
public abstract record BackupRepositorySpec;
```

---

# 11. Execution Context

A destination may be accessed from different execution locations.

```csharp
public enum BackupExecutionLocation
{
    Core,
    Platform
}
```

```csharp
public sealed record BackupExecutionContext(
    BackupExecutionLocation Location,
    Guid? PlatformId);
```

Validation:

* `Core` requires `PlatformId = null`.
* `Platform` requires a valid `PlatformId`.
* Citadel system backup always uses Core.
* Docker-volume backup always uses its source Platform.
* S3 reachability is evaluated from the executor used by the policy.
* Repository lifecycle status and executor reachability are separate concepts.

---

# 12. Filesystem Destination

```csharp
public sealed record FileSystemBackupRepositorySpec(
    BackupExecutionLocation Location,
    Guid? PlatformId,
    string Path)
    : BackupRepositorySpec;
```

Rules:

* Core filesystem destination:

  * `Location = Core`
  * `PlatformId = null`
  * may be used only for Citadel system backup.
* Platform filesystem destination:

  * `Location = Platform`
  * `PlatformId` required;
  * may be used only for Docker volumes on the same Platform.
* `Path` must be absolute.
* `Path` must be inside an allowlisted root.
* Reject traversal.
* Reject restricted system paths.
* Reject Docker socket paths.
* Reject Citadel source data directories.
* Reject the backup working directory.
* Resolve symbolic links where the operating system permits.
* The path is a host path visible to:

  * Core for Core destinations;
  * the Docker daemon for Platform destinations.
* A Platform path is not merely a path inside the Agent container.

Citadel does not mount NFS or SMB.

Administrators must mount network storage before configuring the destination.

---

# 13. S3-Compatible Destination

```csharp
public sealed record S3CompatibleBackupRepositorySpec(
    Uri Endpoint,
    string Bucket,
    string? Prefix,
    string? Region,
    S3BucketLookup BucketLookup,
    Guid AccessKeySecretId,
    Guid SecretKeySecretId,
    Guid? SessionTokenSecretId,
    bool AllowInsecureHttp = false)
    : BackupRepositorySpec;
```

```csharp
public enum S3BucketLookup
{
    Auto,
    Path,
    Dns
}
```

Validation:

* Endpoint must be absolute HTTP or HTTPS.
* HTTPS is required unless insecure HTTP is explicitly enabled.
* Bucket is required.
* Prefix is normalized by trimming surrounding slashes.
* Credential IDs must reference existing SecretDefinitions.
* Resolved credentials are never returned.
* Build the Restic URI server-side.
* Do not accept a raw Restic URI from the client.

Example:

```text
s3:https://minio.example.com/backups/citadel
```

The same S3 repository may have different validation results for Core and different Platforms.

---

# 14. BackupRepository

```csharp
public sealed class BackupRepository
{
    public Guid Id { get; init; }

    public string Name { get; private set; } = default!;
    public string NormalizedName { get; private set; } = default!;
    public string? Description { get; private set; }

    public BackupRepositoryType Type { get; private set; }
    public BackupRepositorySpec Spec { get; private set; } = default!;

    public Guid PasswordSecretId { get; private set; }

    public BackupRepositoryStatus Status { get; private set; }

    public DateTimeOffset? LastPrunedAt { get; private set; }
    public DateTimeOffset? LastCheckedAt { get; private set; }

    public Guid CreatedByActorId { get; init; }
    public DateTimeOffset CreatedAt { get; init; }
    public DateTimeOffset UpdatedAt { get; private set; }

    public DateTimeOffset? ArchivedAt { get; private set; }
    public long RowVersion { get; private set; }
}
```

```csharp
public enum BackupRepositoryStatus
{
    Unknown,
    Uninitialized,
    Ready
}
```

Rules:

* Name is unique case-insensitively using `NormalizedName`.
* Archived destinations are hidden from normal destination lookups.
* Archiving does not delete the Restic repository.
* Archiving does not delete historical runs.
* Archived destinations may still be used to restore existing available snapshots.
* A destination cannot be archived while:

  * an operation is active;
  * a non-archived policy references it.
* `PasswordSecretId` may be changed only while the Restic repository is uninitialized.
* Changing a SecretDefinition reference is not Restic password rotation.
* Restic password rotation is out of scope.
* Once initialized, destination identity fields that determine repository location must not be silently changed.
* Destination mutation after initialization must reject repository-location changes.
* Create a new destination resource when repository location changes.

---

# 15. BackupRepositoryValidation

Repository reachability is stored per execution context.

```csharp
public sealed class BackupRepositoryValidation
{
    public Guid BackupRepositoryId { get; init; }

    public BackupExecutionLocation Location { get; init; }
    public Guid? PlatformId { get; init; }

    public BackupRepositoryValidationStatus Status { get; private set; }

    public DateTimeOffset LastValidatedAt { get; private set; }
    public string? LastErrorCode { get; private set; }
    public string? LastErrorMessage { get; private set; }
}
```

```csharp
public enum BackupRepositoryValidationStatus
{
    Unknown,
    Ready,
    Uninitialized,
    Unavailable,
    InvalidPassword,
    InvalidConfiguration
}
```

Composite identity:

```text
BackupRepositoryId + Location + PlatformId
```

Rules:

* Validation results are advisory and executor-specific.
* A previous successful validation does not replace runtime validation.
* A backup must still fail safely if the destination becomes unavailable.
* Core validation does not prove that an Edge Agent can access an S3 endpoint.
* Platform validation executes through that Platform's Backup connector.
* Filesystem validation must match the destination's configured executor.
* S3 validation accepts an explicit execution context.

---

# 16. BackupPolicy

```csharp
public sealed class BackupPolicy
{
    public Guid Id { get; init; }

    public string Name { get; private set; } = default!;
    public string NormalizedName { get; private set; } = default!;
    public string? Description { get; private set; }

    public BackupSourceSpec Source { get; private set; } = default!;
    public Guid BackupRepositoryId { get; private set; }

    public bool Enabled { get; private set; }

    public string? Cron { get; private set; }
    public string? TimeZone { get; private set; }

    public int KeepLastSuccessful { get; private set; }
    public int TimeoutSeconds { get; private set; }
    public bool AlertOnFailure { get; private set; }

    public Guid RunAsActorId { get; private set; }

    public ResourceControlState ControlState { get; private set; }
    public Guid? CurrentRunId { get; private set; }
    public DateTimeOffset? LastScheduledRunAt { get; private set; }

    public DateTimeOffset? FirstSuccessfulRunAt { get; private set; }

    public Guid CreatedByActorId { get; init; }
    public DateTimeOffset CreatedAt { get; init; }
    public DateTimeOffset UpdatedAt { get; private set; }

    public DateTimeOffset? ArchivedAt { get; private set; }
    public long RowVersion { get; private set; }
}
```

Defaults:

```text
Enabled = true
KeepLastSuccessful = 14
TimeoutSeconds = 14400
AlertOnFailure = true
Consistency = Live
```

Validation:

* Name required.
* Name maximum 128 characters.
* Normalize name using current Citadel resource conventions.
* Cron and timezone must both be present or both absent.
* `KeepLastSuccessful` range:

  * minimum 1;
  * maximum 1000.
* Timeout range:

  * minimum 60 seconds;
  * maximum 24 hours.
* Docker-volume source requires:

  * accessible Platform;
  * existing named volume at creation time;
  * compatible destination.
* Citadel-system source requires:

  * Core filesystem destination; or
  * S3 destination.
* Platform filesystem destination must match source Platform.
* Only one Citadel-system backup may execute at a time.
* Archived policies cannot queue new runs.
* A policy cannot be changed while a run is active.
* After the first successful run:

  * `Source` is immutable;
  * `BackupRepositoryId` is immutable.
* To change source or destination after first success, create a new policy.
* `RunAsActorId` must reference an enabled actor that can execute the policy.
* Scheduled permissions are re-evaluated when the scheduled run is claimed.
* Archiving keeps historical runs and snapshot metadata.
* Archiving disables scheduling.

---

# 17. BackupRun

```csharp
public sealed class BackupRun
{
    public Guid Id { get; init; }

    public Guid BackupPolicyId { get; init; }
    public Guid BackupRepositoryId { get; init; }

    public string PolicyNameSnapshot { get; init; } = default!;
    public BackupSourceSpec SourceSnapshot { get; init; } = default!;
    public BackupRepositoryType RepositoryTypeSnapshot { get; init; }

    public BackupRunTrigger Trigger { get; init; }
    public Guid? TriggerSourceId { get; init; }

    public BackupRunStatus Status { get; private set; }

    public string? ResticSnapshotId { get; private set; }
    public string? ParentSnapshotId { get; private set; }
    public BackupSnapshotAvailability SnapshotAvailability { get; private set; }

    public long? FilesProcessed { get; private set; }
    public long? BytesProcessed { get; private set; }
    public long? BytesAdded { get; private set; }

    public IReadOnlyList<BackupRunWarning> Warnings { get; private set; } = [];

    public DateTimeOffset QueuedAt { get; init; }
    public DateTimeOffset? StartedAt { get; private set; }
    public DateTimeOffset? CompletedAt { get; private set; }

    public int? ExitCode { get; private set; }
    public string? ErrorCode { get; private set; }
    public string? ErrorMessage { get; private set; }

    public Guid TriggeredByActorId { get; init; }
}
```

```csharp
public enum BackupRunTrigger
{
    Manual,
    Schedule,
    Automation
}
```

```csharp
public enum BackupRunStatus
{
    Queued,
    Preparing,
    Running,
    ApplyingRetention,
    Succeeded,
    SucceededWithWarnings,
    Failed,
    TimedOut,
    Cancelled,
    Rejected,
    Interrupted
}
```

```csharp
public enum BackupSnapshotAvailability
{
    Pending,
    Available,
    Expired,
    Missing,
    NotCreated
}
```

```csharp
public sealed record BackupRunWarning(
    string Code,
    string Message);
```

Rules:

* `Manual` uses the current actor.
* `Schedule` uses `BackupPolicy.RunAsActorId`.
* `Automation` uses the current Automation run actor.
* For Automation:

  * `TriggerSourceId` is the Automation Action Run ID.
* Run snapshots remain after policy archival.
* Successful Restic backup:

  * stores snapshot ID;
  * sets availability to `Available`.
* Failed or cancelled before snapshot creation:

  * sets availability to `NotCreated`.
* Retention forget:

  * retains BackupRun;
  * sets availability to `Expired`.
* Repository check or restore may mark a missing snapshot as `Missing`.
* Restore is allowed only for `Available` snapshots.
* `SucceededWithWarnings` is used when:

  * snapshot succeeded but retention failed;
  * snapshot succeeded but container restart failed;
  * non-fatal cleanup failed.
* Warnings are structured and sanitized.
* Logs are persisted separately.

Never persist:

* repository password;
* S3 secret key;
* session token;
* PostgreSQL password;
* resolved internal secret values;
* raw process environment;
* temporary credential paths containing sensitive values.

---

# 18. BackupRestoreRun

```csharp
public sealed class BackupRestoreRun
{
    public Guid Id { get; init; }

    public Guid BackupRunId { get; init; }
    public Guid BackupRepositoryId { get; init; }

    public BackupRestoreStatus Status { get; private set; }

    public Guid TargetPlatformId { get; init; }
    public string TargetVolumeName { get; init; } = default!;

    public bool OverwriteExisting { get; init; }
    public bool TargetVolumeCreatedByCitadel { get; private set; }

    public IReadOnlyList<BackupAffectedContainer> AffectedContainers { get; private set; } = [];
    public IReadOnlyList<BackupRunWarning> Warnings { get; private set; } = [];

    public DateTimeOffset QueuedAt { get; init; }
    public DateTimeOffset? StartedAt { get; private set; }
    public DateTimeOffset? CompletedAt { get; private set; }

    public int? ExitCode { get; private set; }
    public string? ErrorCode { get; private set; }
    public string? ErrorMessage { get; private set; }

    public Guid TriggeredByActorId { get; init; }
}
```

```csharp
public enum BackupRestoreStatus
{
    Queued,
    Preparing,
    Running,
    Succeeded,
    SucceededWithWarnings,
    Failed,
    TimedOut,
    Cancelled,
    Rejected,
    Interrupted
}
```

```csharp
public sealed record BackupAffectedContainer(
    string DockerContainerId,
    string Name,
    ContainerStateStatus OriginalState,
    bool StopAttempted,
    bool RestartAttempted,
    bool RestartSucceeded);
```

---

# 19. Snapshot Identity

Tag each Restic snapshot using deterministic tags.

Common tags:

```text
citadel
citadel-policy:<policy-id>
citadel-run:<run-id>
citadel-source-type:<source-type>
citadel-source:<stable-source-key>
```

Docker volume:

```text
citadel-source-type:DockerVolume
citadel-source:<platform-id>:<volume-name>
```

Citadel system:

```text
citadel-source-type:CitadelSystem
citadel-source:citadel-system
```

Deterministic hostnames:

```text
citadel-core
citadel-platform-<platform-id>
```

Rules:

* Retention filters by `citadel-policy:<policy-id>`.
* Restore selects the exact stored Restic snapshot ID.
* Do not select snapshots by display name.
* Do not mix different source identities under one policy.
* Policy source immutability protects retention groups.

---

# 20. Persistence

Add tables:

```text
backuprepositories
backuprepositoryvalidations
backuppolicies
backupruns
backuprunlogs
backuprestoreruns
backuprestorerunlogs
backuprepositoryleases
backupsourceleases
```

Use lowercase schema naming.

## Repository indexes

```text
backuprepositories(normalizedname) unique
backuprepositories(status)
backuprepositories(archivedat)
```

## Validation indexes

```text
backuprepositoryvalidations(
  backuprepositoryid,
  location,
  platformid
) unique
```

## Policy indexes

```text
backuppolicies(normalizedname) unique
backuppolicies(backuprepositoryid)
backuppolicies(enabled)
backuppolicies(createdbyactorid)
backuppolicies(archivedat)
```

## Run indexes

```text
backupruns(backuppolicyid, queuedat desc)
backupruns(backuprepositoryid, status)
backupruns(resticsnapshotid)
backupruns(snapshotavailability)
```

## Restore indexes

```text
backuprestoreruns(backuprunid)
backuprestoreruns(targetplatformid, queuedat desc)
```

Rules:

* Use `jsonb` for polymorphic source/repository snapshots and warning lists.
* Historical runs keep policy and repository IDs.
* Policies and repositories use soft archival instead of physical deletion.
* Do not cascade-delete run history.
* Do not cascade-delete repository validation history unless the repository is physically purged by a future maintenance tool.
* Bulk list queries must avoid per-row repository and authorization loops.
* Include capabilities in list and detail views.

---

# 21. Backup Repository Lease

Repository operations are serialized across all Core instances.

```csharp
public sealed class BackupRepositoryLease
{
    public Guid BackupRepositoryId { get; init; }
    public Guid LeaseId { get; init; }

    public Guid OwnerInstanceId { get; init; }
    public Guid OperationId { get; init; }
    public BackupRepositoryOperationType OperationType { get; init; }

    public DateTimeOffset AcquiredAt { get; init; }
    public DateTimeOffset RenewedAt { get; private set; }
    public DateTimeOffset ExpiresAt { get; private set; }
}
```

```csharp
public enum BackupRepositoryOperationType
{
    Initialize,
    Validate,
    Backup,
    Restore,
    Forget,
    Check,
    Prune
}
```

Rules:

* Acquire atomically.
* Renew periodically.
* Release only with matching `LeaseId`.
* A lease owner must stop execution if renewal fails or ownership is lost.
* Expired leases may be replaced.
* A fixed 60-second lease without heartbeat is invalid.
* Default lease duration may be 60 seconds.
* Renew significantly before expiry.
* Restic locks remain enabled.
* Citadel lease is additional protection, not a replacement.

---

# 22. Backup Source Lease

Protect the same Docker volume from concurrent backup or restore operations.

```csharp
public sealed class BackupSourceLease
{
    public string SourceKey { get; init; } = default!;
    public Guid LeaseId { get; init; }

    public Guid OwnerInstanceId { get; init; }
    public Guid OperationId { get; init; }
    public BackupSourceOperationType OperationType { get; init; }

    public DateTimeOffset AcquiredAt { get; init; }
    public DateTimeOffset RenewedAt { get; private set; }
    public DateTimeOffset ExpiresAt { get; private set; }
}
```

```csharp
public enum BackupSourceOperationType
{
    Backup,
    Restore
}
```

Source key:

```text
<platform-id>:<volume-name>
```

Rules:

* Acquire before stopping containers or mounting the volume.
* A source cannot be backed up and restored concurrently.
* Two policies targeting the same volume cannot run concurrently even when they use different destinations.
* Source and repository leases are both required.
* Acquire leases in a consistent order to prevent deadlocks:

  1. repository;
  2. source.

---

# 23. Run Queue and Worker

Use persisted queued runs.

## Start endpoint

```text
POST /api/v1/backupPolicies/{id}/runs
```

Response:

```text
202 Accepted
BackupRunView
```

The endpoint:

1. authorizes the actor;
2. validates policy state;
3. creates a queued run;
4. updates policy control state atomically;
5. returns immediately.

The endpoint must not execute Restic.

## Cancel endpoint

```text
POST /api/v1/backupRuns/{id}/cancel
```

Cancellation targets a run, not a policy.

## Worker

Create a background worker matching existing Automation run-worker principles.

Requirements:

* Poll queued runs.
* Claim runs atomically.
* Respect `Backups.MaxParallelRuns`.
* Re-evaluate permissions when claiming.
* Reject unauthorized scheduled runs.
* Resolve a short-lived service scope per execution.
* Stream progress internally from the execution service.
* Persist logs and status updates in short DB operations.
* Release control state and leases in `finally`.

## Progress

The UI may use:

* run polling;
* run log endpoint;
* run event stream;
* SignalR updates.

Do not keep the original POST request open for the full backup.

Recommended endpoint:

```text
GET /api/v1/backupRuns/{id}/events
```

This endpoint may expose `IAsyncEnumerable<BackupExecutionEvent>` for live progress after the run has been queued.

---

# 24. Crash Reconciliation

Add a startup reconciliation service.

On startup:

* keep `Queued` runs queued;
* detect stale:

  * `Preparing`;
  * `Running`;
  * `ApplyingRetention`;
* mark stale active runs as `Interrupted`;
* mark stale restore runs as `Interrupted`;
* clear stale policy `CurrentRunId`;
* reset stale policy `ControlState`;
* release expired leases;
* identify orphan helper containers by Citadel labels;
* request cleanup through the correct connector when practical.

Do not automatically retry interrupted destructive restore operations.

Interrupted backup runs may be retried manually.

---

# 25. Application Services

Add under:

```text
src/Citadel.Application/Services/Backups
```

Interfaces:

```text
IBackupRepositoryService
IBackupRepositoryValidationService
IBackupRunQueueService
IBackupExecutionService
IBackupRestoreService
IResticProcessRunner
IResticEnvironmentBuilder
IBackupRepositoryLeaseService
IBackupSourceLeaseService
ICitadelSystemBackupService
ICitadelRecoveryAssetProvider
IBackupCoverageService
IBackupCrashReconciliationService
```

Rules:

* Do not create services that duplicate existing Mediator commands.
* Long-running services use `IServiceScopeFactory`.
* Do not retain scoped repositories across process execution.
* Prefer explicit state transitions on domain entities.

---

# 26. Restic Process Runner

```csharp
public interface IResticProcessRunner
{
    IAsyncEnumerable<ResticProcessEvent> RunAsync(
        ResticProcessCommand command,
        CancellationToken cancellationToken);
}
```

Requirements:

* Use `ProcessStartInfo.ArgumentList`.
* Never execute through a shell.
* Support cancellation.
* Support timeout.
* Kill the full process tree.
* Capture stdout and stderr separately.
* Prefer JSON output where Restic supports it.
* Tolerate unknown JSON fields.
* Return exit code.
* Extract snapshot ID.
* Limit individual persisted log entries.
* Remove ANSI sequences.
* Redact secrets before:

  * streaming;
  * logging;
  * persistence.
* Never log the complete environment.

Core Restic environment may contain:

```text
RESTIC_REPOSITORY
RESTIC_PASSWORD_FILE
AWS_ACCESS_KEY_ID
AWS_SECRET_ACCESS_KEY
AWS_SESSION_TOKEN
AWS_DEFAULT_REGION
```

Repository password:

* write to an unpredictable temporary file;
* restrict file permissions where supported;
* delete in `finally`.

---

# 27. Repository Lifecycle

## Create

Creating a destination stores configuration.

Initial status:

```text
Unknown
```

## Validate

```text
POST /api/v1/backupRepositories/{id}/validate
```

Input:

```csharp
public sealed record ValidateBackupRepositoryInput(
    BackupExecutionLocation Location,
    Guid? PlatformId);
```

For filesystem destinations, the requested context must match the configured context.

Validation checks:

* execution context is valid;
* credentials resolve;
* destination is reachable;
* repository is initialized;
* repository password opens the repository.

Persist executor-specific validation result.

## Initialize

```text
POST /api/v1/backupRepositories/{id}/initialize
```

Input includes execution context for S3.

Rules:

* Initialization is explicit.
* Acquire repository lease.
* Reject unknown non-Restic contents.
* Never overwrite an existing unknown repository.
* Run initialization on the selected executor.
* Mark repository `Ready` only after successful open validation.
* `PasswordSecretId` becomes immutable.
* Emit activity.

## Check

```text
POST /api/v1/backupRepositories/{id}/check
```

Run at the requested or configured executor.

## Prune

```text
POST /api/v1/backupRepositories/{id}/prune
```

Rules:

* separate operation;
* never automatically run after every backup;
* requires repository lease;
* no backup or restore may run concurrently.

---

# 28. Retention

MVP retention:

```csharp
public sealed record BackupRetentionPolicy(
    int KeepLastSuccessful);
```

After a successful snapshot:

1. set snapshot availability to `Available`;
2. execute Restic forget filtered by policy tag;
3. keep the latest configured successful snapshots;
4. mark forgotten run snapshots as `Expired`;
5. do not delete BackupRun rows;
6. do not run retention after failed or cancelled backup.

If snapshot succeeds but retention fails:

* backup status is `SucceededWithWarnings`;
* snapshot remains `Available`;
* add warning:

  * `backup.retention_failed`;
* raise warning alert when configured.

Do not prune after every retention operation.

Maintenance worker:

* processes Ready repositories;
* runs only when repository lease can be acquired;
* performs scheduled check and prune;
* stores maintenance timestamps;
* raises sanitized maintenance alerts.

---

# 29. Backup Connector

Add:

```csharp
public interface IBackupConnector
{
    IAsyncEnumerable<BackupExecutionEvent> ValidateRepositoryAsync(
        ValidateBackupRepositoryCommand command,
        CancellationToken cancellationToken);

    IAsyncEnumerable<BackupExecutionEvent> InitializeRepositoryAsync(
        InitializeBackupRepositoryCommand command,
        CancellationToken cancellationToken);

    IAsyncEnumerable<BackupExecutionEvent> CheckRepositoryAsync(
        CheckBackupRepositoryCommand command,
        CancellationToken cancellationToken);

    IAsyncEnumerable<BackupExecutionEvent> PruneRepositoryAsync(
        PruneBackupRepositoryCommand command,
        CancellationToken cancellationToken);

    IAsyncEnumerable<BackupExecutionEvent> BackupVolumeAsync(
        BackupVolumeCommand command,
        CancellationToken cancellationToken);

    IAsyncEnumerable<BackupExecutionEvent> RestoreVolumeAsync(
        RestoreVolumeCommand command,
        CancellationToken cancellationToken);

    Task<Result> CancelOperationAsync(
        CancelBackupOperationCommand command,
        CancellationToken cancellationToken);
}
```

Implement:

```text
LocalBackupConnector
AgentBackupConnector
EdgeBackupConnector
```

Register through the existing connector factory.

Do not create another connector-registration framework.

---

# 30. Agent and Edge Agent Contracts

Add shared protobuf contracts for:

* repository execution context;
* repository configuration;
* secret material passed ephemerally;
* operation metadata;
* progress event;
* completion result;
* cancellation.

Suggested stable command values:

```csharp
BackupRepositoryValidate = 90,
BackupRepositoryInitialize = 91,
BackupRepositoryCheck = 92,
BackupRepositoryPrune = 93,
DockerVolumeBackup = 94,
DockerVolumeRestore = 95,
BackupOperationCancel = 96
```

Rules:

* Use server-streaming for long operations.
* Events are sanitized before they leave the Agent.
* Core applies a second redaction pass.
* Backup bytes never enter protobuf messages.
* Agent disconnect cancels the helper operation for MVP.
* Helper cleanup runs after disconnect or cancellation.
* Resumable operations are out of scope.

---

# 31. Helper Container Security

The helper receives:

* source volume read-only for backup;
* target volume read-write for restore;
* repository host path when applicable;
* temporary repository password file;
* temporary S3 credentials.

Use labels:

```text
citadel.managed=true
citadel.operation=backup
citadel.backup-run-id=<run-id>
citadel.backup-policy-id=<policy-id>
```

Restore labels:

```text
citadel.managed=true
citadel.operation=restore
citadel.restore-run-id=<restore-run-id>
```

Requirements:

* No Docker socket.
* Drop unnecessary capabilities.
* Read-only root filesystem where practical.
* `no-new-privileges`.
* CPU and memory limits.
* Pinned helper image.
* No user-supplied image.
* Always remove helper in `finally`.
* Credentials are not included in container names, labels, or command arguments.
* Filesystem host path must be allowlisted by the executing Core or Agent.

---

# 32. Docker-Volume Backup

## Live backup

Flow:

1. Validate policy and source.
2. Confirm Platform is online.
3. Confirm named volume exists.
4. Acquire repository lease.
5. Acquire source lease.
6. Resolve repository credentials.
7. Start helper:

   * source volume mounted read-only.
8. Run Restic backup.
9. Apply deterministic tags.
10. Capture snapshot and statistics.
11. Remove helper and credentials.
12. Apply retention.
13. Release source and repository leases.
14. Complete run.

## Stop-attached-containers backup

Flow:

1. Discover containers mounting the volume.
2. Record their original states.
3. Stop only containers that are running.
4. Confirm stopped state.
5. Execute backup.
6. Restart only containers that were running.
7. Restart in `finally`, including backup failure paths.
8. Record restart results.

Status:

* backup failed before snapshot:

  * `Failed`;
* snapshot succeeded but restart failed:

  * `SucceededWithWarnings`;
* stopping failed:

  * `Failed`.

Do not delete or recreate attached containers.

---

# 33. Docker-Volume Restore

Restore is supported only for an `Available` Docker-volume snapshot.

## Queue endpoint

```text
POST /api/v1/backupRuns/{id}/restoreVolume
```

Returns:

```text
202 Accepted
BackupRestoreRunView
```

## Restore into new volume

Default behavior.

Flow:

1. Validate snapshot availability.
2. Validate target Platform.
3. Acquire repository lease.
4. Acquire target source lease.
5. Verify target volume does not exist.
6. Create target volume.
7. Mark `TargetVolumeCreatedByCitadel = true`.
8. Verify new volume is empty.
9. Restore exact snapshot into target volume.
10. Complete restore.
11. If restore fails:

    * remove the newly created partial volume when possible;
    * record cleanup warning if deletion fails.

Suggested default name:

```text
<original-volume>-restored-<yyyyMMdd-HHmmss>
```

## Overwrite existing volume

Rules:

* User explicitly selects overwrite.
* UI requires exact volume-name confirmation.
* Target Platform permissions are re-evaluated.
* Acquire target source lease.
* Discover attached containers.
* Record original states.
* Stop running containers.
* Mount only the selected target volume.
* Explicitly clear existing volume contents.
* Restore the selected snapshot.
* Restart containers that were previously running.

If failure occurs after contents are cleared:

* status is `Failed`;
* error is Critical;
* state that target data may be incomplete;
* do not claim the old data is intact.

If restore succeeds but container restart fails:

* status is `SucceededWithWarnings`.

Cross-platform restore:

* allowed for S3 destinations reachable from target Platform;
* allowed for Platform filesystem only on the same Platform;
* Core filesystem destination cannot restore directly to a remote Platform in MVP.

---

# 34. Citadel System Backup

Citadel system backup runs in Core.

## Staged archive

```text
<working-directory>/<run-id>/
|-- manifest.json
|-- database/
|   `-- citadel.dump
`-- recovery/
    |-- jwtsecret
    |-- secret-encryption-key
    |-- keys/
    |   |-- id_ed25519
    |   |-- id_ed25519.pub
    |   `-- dataprotection/
    `-- checksums.json
```

Do not archive all of `/app/data`.

Exclude:

* Git cache;
* Stack workspaces;
* Automation run directories;
* Deno cache;
* logs;
* backup working directory;
* configured backup destination paths;
* temporary files.

## PostgreSQL dump

Use:

```text
pg_dump
--format=custom
--no-owner
--no-privileges
```

Parse the connection string with `NpgsqlConnectionStringBuilder`.

Use ephemeral environment variables:

```text
PGHOST
PGPORT
PGDATABASE
PGUSER
PGPASSWORD
PGSSLMODE
```

Never place the PostgreSQL password in arguments or logs.

Fail clearly if:

* `pg_dump` is missing;
* the PostgreSQL client cannot dump the configured server;
* database connection fails;
* staging cannot be created.

## Recovery assets

Add:

```csharp
public interface ICitadelRecoveryAssetProvider
{
    IReadOnlyList<CitadelRecoveryAsset> GetAssets();
}
```

```csharp
public sealed record CitadelRecoveryAsset(
    string Name,
    string RelativeArchivePath,
    RecoveryAssetOrigin Origin,
    bool Required,
    string? SourcePath);
```

```csharp
public enum RecoveryAssetOrigin
{
    File,
    ExternalConfiguration,
    Missing
}
```

Account for:

```text
./data/jwtsecret
./data/secret-encryption-key
./data/keys/id_ed25519
./data/keys/id_ed25519.pub
./data/keys/dataprotection/
```

Rules:

* If an asset is externally configured:

  * record `ExternalConfiguration`;
  * do not place its plaintext value in the archive;
  * manifest warns that it must be supplied after restore.
* Required missing assets fail backup.
* Optional missing assets produce warnings.
* Calculate SHA-256 for copied recovery assets.

## Manifest

```csharp
public sealed record CitadelBackupManifest(
    int FormatVersion,
    string Product,
    string CoreInformationalVersion,
    string CompatibilityVersion,
    Guid InstanceId,
    DateTimeOffset CreatedAt,
    CitadelDatabaseManifest Database,
    IReadOnlyList<RecoveryAssetManifestEntry> Assets);
```

```csharp
public sealed record CitadelDatabaseManifest(
    string Engine,
    string DumpFile,
    string ServerVersion,
    string Sha256);
```

Manifest version:

```text
1
```

Manifest must not contain:

* passwords;
* repository credentials;
* secret plaintext;
* tokens;
* environment values.

## Restic execution

After staging:

1. run Restic backup against staging root;
2. use Citadel-system tags;
3. capture snapshot ID;
4. remove staging in `finally`;
5. run retention after snapshot success.

---

# 35. Repository Password Disaster Recovery

Every Restic repository uses a password stored through a SecretDefinition.

The UI must warn:

```text
Store this repository password outside Citadel.

A lost Citadel database cannot recover the repository password that was stored only inside Citadel.
Without the password, the Restic repository cannot be restored.
```

If Citadel generates a password:

* show it once;
* require acknowledgement;
* store it as an encrypted SecretDefinition;
* never return it again.

Restore must remain possible using an externally supplied password file.

---

# 36. Offline Citadel Recovery

Create:

```text
src/Citadel.Recovery
```

Suggested command:

```bash
citadel-recovery restore-system \
  --repository <restic-repository> \
  --snapshot <snapshot-id-or-latest> \
  --repository-password-file <path> \
  --connection-string <empty-target-postgres-connection> \
  --data-dir <citadel-data-path> \
  --confirm-server-stopped
```

MVP rules:

* Target database must be empty.
* Do not support in-place database overwrite in the initial implementation.
* Do not use `--clean` as a substitute for recreating a database.
* Do not start Citadel automatically.

Restore sequence:

1. Verify server-stopped confirmation.
2. Open Restic repository.
3. Restore snapshot to temporary directory.
4. Validate manifest structure.
5. Validate format version.
6. Validate checksums.
7. Validate required assets.
8. Validate PostgreSQL tools.
9. Verify target database is empty.
10. Run `pg_restore` into the empty target.
11. Verify restored Citadel schema can be opened.
12. Copy existing recovery asset paths to a rollback directory if they exist.
13. Atomically restore recovery assets.
14. If asset replacement fails:

    * restore previous assets from rollback directory;
    * fail recovery.
15. Clean temporary files.
16. Print next steps.
17. Operator starts Citadel.

Use:

```text
pg_restore
--no-owner
--no-privileges
```

Document default Compose recovery:

```text
1. Stop Citadel Core.
2. Keep or start the target PostgreSQL server.
3. Create an empty target database.
4. Run citadel-recovery.
5. Start Citadel Core.
6. Allow normal startup migrations to run when compatible.
```

---

# 37. Permissions

Add:

```csharp
ResourceType.BackupRepository
ResourceType.BackupPolicy
```

Add:

```csharp
SpecificPermission.Restore
```

Update all required permission locations:

* Contracts enum.
* Permission matrix.
* known specific-permission mask.
* mask serialization.
* mask deserialization.
* admin metadata.
* generated permission metadata.
* permission views.
* role UI.
* seed data.

## Capability rules

### BackupRepository

* Read:

  * view destination;
  * view safe configuration;
  * view validation status.
* Write:

  * create;
  * update;
  * archive.
* Execute:

  * validate;
  * initialize;
  * check;
  * prune.

### BackupPolicy

* Read:

  * view policy;
  * view runs;
  * view logs when logs capability allows.
* Write:

  * create;
  * update;
  * enable;
  * disable;
  * archive.
* Execute:

  * queue backup;
  * cancel own authorized backup.
* Restore:

  * queue Docker-volume restore.

Recommended permission matrix:

```text
BackupRepository:
  max level = Execute

BackupPolicy:
  max level = Execute
  Restore specific permission minimum = Read
  Logs specific permission minimum = Read
```

## Operation authorization

Queue manual backup:

```text
BackupPolicy Execute
```

Scheduled backup:

```text
RunAsActor has BackupPolicy Execute at claim time
```

Automation backup:

```text
Automation run actor has BackupPolicy Execute
```

Stop-attached-containers mode:

```text
BackupPolicy Execute
Source Platform Execute capability
```

Restore into a new volume:

```text
BackupPolicy Restore
Target Platform Write capability
```

Overwrite existing volume:

```text
BackupPolicy Restore
Target Platform Write capability
Target Platform Execute capability
```

Repository validation, initialization, check, and prune:

```text
BackupRepository Execute
```

## Default roles

Do not rely on the generic current seed behavior for the new resources.

Seed explicitly:

```text
Admin:
  BackupRepository = Execute
  BackupPolicy = Execute
  BackupPolicy specifics = Logs, Restore

Operator:
  BackupRepository = Read
  BackupPolicy = Write
  BackupPolicy specifics = Logs
  Restore not granted by default

Viewer:
  BackupRepository = Read
  BackupPolicy = Read
  BackupPolicy specifics = Logs
```

---

# 38. API

All endpoints are under `/api/v1`.

## Destinations

```text
GET    /backupRepositories
GET    /backupRepositories/{id}
POST   /backupRepositories
PATCH  /backupRepositories/{id}
DELETE /backupRepositories/{id}

POST   /backupRepositories/{id}/validate
POST   /backupRepositories/{id}/initialize
POST   /backupRepositories/{id}/check
POST   /backupRepositories/{id}/prune
```

DELETE archives the resource.

## Policies

```text
GET    /backupPolicies
GET    /backupPolicies/{id}
POST   /backupPolicies
PATCH  /backupPolicies/{id}
DELETE /backupPolicies/{id}

POST   /backupPolicies/{id}/runs
```

Filters:

```text
sourceType
platformId
repositoryId
enabled
tags
```

## Runs

```text
GET  /backupRuns
GET  /backupRuns/{id}
GET  /backupRuns/{id}/logs
GET  /backupRuns/{id}/events
POST /backupRuns/{id}/cancel
```

Filters:

```text
policyId
repositoryId
status
trigger
snapshotAvailability
createdFrom
createdTo
```

## Restore runs

```text
POST /backupRuns/{id}/restoreVolume

GET  /backupRestoreRuns
GET  /backupRestoreRuns/{id}
GET  /backupRestoreRuns/{id}/logs
GET  /backupRestoreRuns/{id}/events
POST /backupRestoreRuns/{id}/cancel
```

Reject Citadel-system restore through Web API.

---

# 39. Automation Integration

Automation Actions invoke the same HTTP API as normal users.

Automation may:

* list accessible policies;
* queue a run;
* fetch run status;
* fetch sanitized run logs.

Automation may not:

* read repository password;
* read S3 credentials;
* execute Restic;
* access staging files;
* mount volumes;
* access Docker socket.

Seed a disabled, unscheduled example.

Name:

```text
Run backup policy
```

Settings:

```text
Enabled = false
ScheduleEnabled = false
Webhook = null
```

Example shape:

```ts
const policyName = args.policyName ?? "Daily Citadel backup";

const policiesResponse =
  await citadel.backupPolicies.listBackupPolicies();

const policies = policiesResponse?.backupPolicies ?? [];

const policy = policies.find(
  (item) => item.name === policyName
);

if (!policy) {
  throw new Error(`Backup policy not found: ${policyName}`);
}

const run =
  await citadel.backupPolicies.runBackupPolicy(policy.id);

console.log(`Backup queued: ${run.id}`);
```

We must inspect the generated client and update the example to match the actual direct response shape.

Do not assume an Axios `.data` wrapper unless the generated client actually uses one.

Tag the Action with the existing `System` tag.

The example must not contain:

* credentials;
* real paths;
* real repository names;
* customer-specific IDs.

---

# 40. Backup Coverage

Coverage is a projection, not an aggregate root.

```csharp
public sealed record BackupCoverageView(
    BackupCoverageStatus Status,
    int PolicyCount,
    Guid? LastRunId,
    BackupRunStatus? LastRunStatus,
    DateTimeOffset? LastRunAt,
    DateTimeOffset? LastSuccessfulRunAt,
    DateTimeOffset? NextRunAt);
```

```csharp
public enum BackupCoverageStatus
{
    NotApplicable,
    Unprotected,
    Protected,
    Warning,
    Failed
}
```

Rules:

* `NotApplicable`:

  * no supported named volumes.
* `Unprotected`:

  * named volume exists;
  * no enabled policy.
* `Protected`:

  * enabled policy exists;
  * destination has a usable validation state for the executor;
  * latest successful available snapshot exists.
* `Warning`:

  * policy exists but disabled;
  * repository validation unavailable;
  * latest backup timed out, cancelled, interrupted, or succeeded with warnings.
* `Failed`:

  * latest backup failed.

Restore failures do not determine backup coverage.

For multiple volumes, use the worst child coverage state.

Coverage comes from policy and run persistence.

Do not infer policy coverage only from Restic snapshots.

---

# 41. Resource Integration

## Volume list

Implement full coverage.

Show:

* Protected;
* Unprotected;
* Warning;
* Failed;
* latest successful time.

Filters:

```text
Unprotected only
Failed backups only
```

Actions:

* Create policy.
* Run existing policy.
* Restore latest available snapshot.

## Volume detail

Show:

* policies targeting volume;
* recent runs;
* latest available snapshot;
* create policy;
* run backup;
* restore.

Use backup run history.

Do not require duplicated volume Activity events.

## Platform detail

Load volumes once.

Show:

* protected volume count;
* unprotected volume count;
* failed volume count;
* active policy count;
* recent backup failures;
* compatible Platform filesystem destinations.

## Platform list

Do not remotely inspect every Platform's volumes.

Show only persisted summary:

* active policy count;
* latest related backup status;
* latest backup failure time.

Do not calculate unprotected-volume counts on Platform list in MVP.

## Deployment detail

When runtime data identifies named volumes:

* list each supported named volume;
* show coverage;
* create policy;
* run policy.

Ignore bind mounts.

## Stack detail

Resolve named volumes from actual runtime mounts where available.

Show:

* named volume;
* Platform;
* coverage;
* policy actions.

Do not create a Stack backup source.

## Container detail

In mounts section:

* show coverage for named volumes;
* link to volume backup detail.

## Container list

No backup column in MVP.

---

# 42. Bulk Coverage Query

Use a single application query for a page or detail surface.

```csharp
public sealed record GetBackupCoverage(
    BackupCoverageResourceType ResourceType,
    IReadOnlyList<BackupCoverageResourceKey> Resources)
    : IQuery<Result<IReadOnlyDictionary<BackupCoverageResourceKey, BackupCoverageView>>>;
```

Supported types:

```text
Platform
Volume
Container
Deployment
Stack
```

Keys:

```text
Volume:
  PlatformId + VolumeName

Container:
  Persisted ContainerId when available

Deployment:
  DeploymentId

Stack:
  StackId

Platform:
  PlatformId
```

Rules:

* Apply authorization in bulk.
* Do not call permission evaluation once per row.
* Do not issue frontend per-row requests.
* Runtime mount discovery may happen once per loaded resource detail, not once per table cell.

---

# 43. Activity

Add:

```csharp
ActivityResourceType.BackupRepository
ActivityResourceType.BackupPolicy
```

Events:

```text
BackupRepositoryCreated
BackupRepositoryUpdated
BackupRepositoryRenamed
BackupRepositoryArchived
BackupRepositoryValidated
BackupRepositoryInitialized
BackupRepositoryPruned
BackupRepositoryChecked

BackupPolicyCreated
BackupPolicyUpdated
BackupPolicyRenamed
BackupPolicyArchived

BackupRunQueued
BackupRunStarted
BackupRunSucceeded
BackupRunSucceededWithWarnings
BackupRunFailed
BackupRunTimedOut
BackupRunCancelled
BackupRunRejected
BackupRunInterrupted
BackupRetentionFailed

BackupRestoreQueued
BackupRestoreStarted
BackupRestoreSucceeded
BackupRestoreSucceededWithWarnings
BackupRestoreFailed
BackupRestoreCancelled
BackupRestoreInterrupted
```

Requirements:

* Add concrete `ActivityEventInfo` types.
* Update event-to-resource mapping.
* Update event/info validation.
* Update source-generated JSON contexts.
* Add PlatformId to volume-related backup activities.
* Primary ResourceId:

  * Backup Policy for backup runs;
  * Backup Repository for repository maintenance.
* Do not duplicate one event for Platform, Stack, Deployment, Container, and Volume.
* Volume pages show run history separately.

Successful scheduled backups belong in Activity and run history, not Alerts.

---

# 44. Alerts

Add:

```csharp
AlertResourceType.BackupRepository
AlertResourceType.BackupPolicy
```

Alert types:

```text
BackupRunFailed
BackupRunTimedOut
BackupRunInterrupted
BackupRepositoryUnavailable
BackupRetentionFailed
BackupRepositoryMaintenanceFailed
BackupRepositoryCheckFailed
BackupRestoreFailed
BackupRestoreInterrupted
```

Rules:

* Backup run alerts respect `AlertOnFailure`.
* Repository unavailable alerts use cooldown and deduplication.
* Successful validation or backup may resolve repository-unavailable alert for the same executor.
* Successful backups do not create alerts.
* Restore overwrite failure is Critical.
* Restore-to-new-volume failure is Warning unless existing resources were affected.
* `SucceededWithWarnings` may produce Warning alert when:

  * containers failed to restart;
  * retention failed.

Alerts link to:

* Backup Policy;
* Backup Repository;
* related Platform when known.

---

# 45. Tags and Lookup

Extend:

```csharp
TaggableResourceType.BackupPolicy
LookupResourceType.BackupRepository
LookupResourceType.BackupPolicy
```

Do not tag Backup Repositories in MVP.

Lookup support:

* accessible Platforms;
* named volumes on selected Platform;
* compatible destinations;
* SecretDefinitions for:

  * repository password;
  * S3 access key;
  * S3 secret key;
  * S3 session token;
* Backup Policies accessible to Automation;
* policies covering a selected volume.

Lookups apply current permissions.

---

# 46. Security

Never persist or expose:

* Restic repository passwords;
* S3 secret keys;
* session tokens;
* PostgreSQL passwords;
* resolved internal secrets;
* external provider tokens.

Apply redaction to:

* Restic stdout;
* Restic stderr;
* `pg_dump` output;
* Agent events;
* Edge Agent envelopes;
* SignalR messages;
* run logs;
* activity metadata;
* alert metadata;
* problem details.

## Filesystem path restrictions

Reject:

```text
/var/run/docker.sock
/proc
/sys
/dev
```

Also reject:

* Citadel source data directory as destination;
* backup working directory;
* path traversal;
* non-absolute paths;
* paths outside configured roots.

## Configuration

Core:

```csharp
public sealed class BackupOptions
{
    public bool Enabled { get; init; } = true;

    public string ResticPath { get; init; } = "restic";
    public string PgDumpPath { get; init; } = "pg_dump";
    public string PgRestorePath { get; init; } = "pg_restore";

    public string WorkingDirectory { get; init; } =
        "./data/backups/work";

    public IReadOnlyList<string> AllowedCorePaths { get; init; } = [];

    public int RepositoryLeaseSeconds { get; init; } = 60;
    public int MaxParallelRuns { get; init; } = 2;
}
```

The platform helper image is shared with the volume browser and resolved by `IVolumeHelperImageResolver`.
The resolver must choose the image for the connector that will execute the helper:

```text
Local connector:      ghcr.io/citadel-p/citadel:<release-version>
Agent connector:      ghcr.io/citadel-p/citadel.agent:<release-version>
Edge Agent connector: ghcr.io/citadel-p/citadel.agent:<release-version>
```

Local Docker development may use the currently running Core container image when Core itself is running in Docker. Development installations may still opt into a specific helper image by setting `VolumeBrowser__HelperImage` explicitly, for example `VolumeBrowser__HelperImage=citadel.dev`. That override applies to every connector, so Agent and Edge Agent platforms must also be able to run that image.

Required installation settings should remain minimal.

Core `.env` example:

```env
Backups__AllowedCorePaths__0=/backups
Backups__WorkingDirectory=/app/data/backups/work
Backups__MaxParallelRuns=2
```

A Core filesystem destination such as `/backups` must be mounted persistently into the Core container.

Invalid backup configuration disables backup execution and exposes a health detail.

It must not crash unrelated Citadel functionality.

---

# 47. Failure Codes

Use stable codes:

```text
backup.repository.unavailable
backup.repository.uninitialized
backup.repository.invalid_password
backup.repository.invalid_configuration
backup.repository.busy
backup.repository.lease_lost

backup.source.busy
backup.source.volume_not_found
backup.source.platform_offline
backup.source.unsupported_connector

backup.volume.stop_failed
backup.volume.restart_failed
backup.volume.clear_failed

backup.pg_dump_failed
backup.recovery_asset_missing
backup.recovery_asset_restore_failed

backup.restic_failed
backup.retention_failed
backup.timeout
backup.cancelled
backup.interrupted

restore.snapshot_not_available
restore.snapshot_not_found
restore.target_volume_exists
restore.target_volume_create_failed
restore.target_volume_not_empty
restore.target_volume_cleanup_failed
restore.failed
restore.interrupted
```

Messages are actionable and sanitized.

---

# 48. Cancellation

Rules:

* Queued runs can be cancelled before claim.
* Active runs signal cancellation to Core, Agent, or Edge Agent.
* Kill Restic/helper process.
* Delete temporary credentials.
* Remove helper container.
* Do not run retention.
* Preserve previous snapshots.
* Restart containers stopped by `StopAttachedContainers`.
* Cancellation after snapshot commit:

  * preserve committed snapshot;
  * mark run according to actual snapshot state;
  * do not falsely report no backup exists.
* Destructive overwrite restore cancellation:

  * mark target potentially incomplete;
  * do not claim successful rollback.
* Repository and source leases are released in `finally`.

---

# 49. Tests

## Unit tests

Cover:

* destination name normalization;
* repository status transitions;
* executor-specific validation;
* source/destination compatibility;
* filesystem path allowlisting;
* traversal rejection;
* Restic URI construction;
* S3 prefix normalization;
* deterministic snapshot tags;
* deterministic hostname;
* retention filtering by policy ID;
* expired snapshot-state transitions;
* policy source immutability;
* repository identity immutability;
* schedule validation;
* RunAs actor validation;
* permission checks;
* warning state transitions;
* repository lease acquisition and heartbeat;
* source lease acquisition;
* lease loss;
* crash reconciliation;
* redaction;
* PostgreSQL environment mapping;
* recovery asset enumeration;
* manifest validation;
* checksum validation;
* container stop/restart planning;
* overwrite clear planning.

## Integration tests

Cover:

1. Initialize filesystem repository.
2. Validate from correct executor.
3. Reject incorrect executor.
4. Initialize S3 repository using MinIO.
5. Validate S3 from Core.
6. Validate S3 from Platform connector.
7. Reject invalid repository password.
8. Queue and claim manual run.
9. Reject duplicate active policy run.
10. Reject concurrent operations on same repository.
11. Reject concurrent operations on same volume.
12. Back up Docker volume.
13. Modify data.
14. Restore into new volume.
15. Verify restored files.
16. Delete partial new volume after restore failure.
17. Clear existing volume before overwrite.
18. Stop and restart only previously running containers.
19. Produce `SucceededWithWarnings` when restart fails after snapshot success.
20. Apply retention.
21. Mark forgotten snapshots `Expired`.
22. Preserve runs after policy archival.
23. Create Citadel PostgreSQL dump.
24. Include all required recovery assets.
25. Exclude caches, logs, workspaces, and backup work directory.
26. Restore into empty PostgreSQL database.
27. Restore recovery files atomically.
28. Verify secret values never appear in logs.
29. Agent backup.
30. Edge Agent streaming backup.
31. Agent disconnect cleanup.
32. Cancellation cleanup.
33. Startup stale-run reconciliation.
34. Path outside allowlist rejection.
35. Permission enforcement for run and restore.
36. Automation-triggered run uses Automation actor and trigger source ID.

## Frontend tests

Cover:

* source-dependent form fields;
* compatible destination filtering;
* executor-specific validation display;
* repository password warning;
* secret fields never expose plaintext;
* scheduled RunAs selection;
* warning status;
* expired snapshot disables Restore;
* overwrite exact-name confirmation;
* permission-driven actions;
* Volume coverage badges;
* Platform-detail coverage;
* Deployment and Stack volume coverage;
* no frontend per-row coverage requests;
* Unprotected volume filter;
* default Automation example is disabled and unscheduled.

---

# 50. Implementation Slices

## Slice 1 - Foundation

Implement only:

* enums;
* domain entities;
* normalized names;
* archival semantics;
* run state machines;
* warning models;
* snapshot availability;
* persistence tables;
* repository interfaces;
* Dapper repositories;
* EF migration;
* JSON contexts;
* permissions;
* API models and route skeletons;
* unit tests.

Do not execute Restic in Slice 1.

## Slice 2 - Backup Destinations

Implement:

* Core Restic runner;
* environment builder;
* filesystem destination;
* S3 destination;
* executor-specific validation;
* initialize;
* check;
* prune;
* repository lease and heartbeat;
* destination API;
* destination frontend;
* filesystem and MinIO integration tests.

## Slice 3 - Citadel System Backup

Implement:

* run queue;
* run worker;
* Core execution path;
* PostgreSQL dump;
* recovery asset provider;
* manifest;
* checksums;
* staging cleanup;
* manual Citadel-system backup;
* run history;
* logs;
* crash reconciliation;
* integration tests.

This slice makes Citadel able to back itself up.

## Slice 4 - Scheduling, Retention, Activity, Alerts, Automation

Implement:

* `RunAsActorId`;
* scheduler;
* scheduled claim authorization;
* retention;
* expired snapshot state;
* maintenance scheduling;
* alerts;
* activity event info mappings;
* Automation-generated client support;
* disabled unscheduled default Action;
* tests.

## Slice 5 - Docker-Volume Backup

Implement:

* backup connector interface;
* shared contracts;
* helper image;
* Local connector;
* regular Agent connector;
* Edge Agent connector;
* source lease;
* live backup;
* stop-attached-containers backup;
* Volume coverage;
* Platform-detail coverage;
* tests.

## Slice 6 - Docker-Volume Restore

Implement:

* restore queue and worker;
* restore into new volume;
* cleanup after failure;
* destructive overwrite;
* explicit volume clearing;
* cross-platform S3 restore;
* warnings;
* Restore UI;
* tests.

## Slice 7 - Offline Citadel Recovery

Implement:

* `Citadel.Recovery`;
* Restic snapshot retrieval;
* manifest validation;
* checksum validation;
* empty-database enforcement;
* PostgreSQL restore;
* atomic recovery-asset restore;
* rollback of asset replacement;
* recovery documentation;
* end-to-end disaster recovery test.

## Slice 8 - Extended Resource Integration

Implement:

* Deployment detail;
* Stack detail;
* Container detail;
* Platform-list persisted summary;
* backup links from related resources;
* additional frontend tests.

Do not add cross-resource duplicated activity events.

---

# 51. Acceptance Criteria

The feature is complete when:

* Admin can configure filesystem and S3-compatible destinations.
* Validation is stored per executor.
* Admin can initialize and check a repository.
* Repository password cannot be silently changed after initialization.
* Admin can manually queue a Citadel system backup.
* Citadel system backup contains:

  * logical PostgreSQL dump;
  * JWT secret when file-backed;
  * secret-encryption key when file-backed;
  * Agent hub keys;
  * Data Protection keys;
  * manifest;
  * checksums.
* Citadel system backup excludes caches and workspaces.
* Scheduled runs execute under an explicit RunAs actor.
* Automation can queue backup through generated API.
* Default Automation example is disabled and unscheduled.
* Snapshot identity uses deterministic Restic tags.
* Retention marks old run snapshots as Expired.
* Historical runs survive policy archival.
* Repository and source operations use renewable leases.
* Stale runs are reconciled after restart.
* Docker volumes can be backed up through:

  * Local;
  * Agent;
  * Edge Agent.
* Backup bytes never pass through Core for Agent or Edge Agent.
* A volume can be restored into a new volume.
* Failed new-volume restore cleans partial volume when possible.
* Overwrite restore explicitly clears the target.
* Overwrite restore requires exact confirmation.
* Restore permissions include target Platform checks.
* Citadel system restore works through offline recovery CLI.
* Restore uses an empty PostgreSQL database.
* Recovery assets are restored atomically.
* Secrets never appear in:

  * API responses;
  * logs;
  * events;
  * activities;
  * alerts;
  * SignalR;
  * persisted command payloads.
* Coverage is visible on supported related resources.
* Frontend does not issue one coverage request per row.
* All migrations, generated API models, generated frontend types, tests, and documentation are updated.

---

# 52. Final Design Rules

1. Backups are first-class Citadel resources.
2. Automation orchestrates backups only through the API.
3. Backup start endpoints queue runs and return immediately.
4. Cancellation targets run IDs.
5. Use Restic instead of a custom backup format.
6. Use `pg_dump`, never the live PostgreSQL data volume.
7. Include explicit control-plane recovery assets.
8. Never archive all of `/app/data`.
9. Process backup data near its source.
10. Never send Agent backup bytes through Core.
11. Use renewable repository leases.
12. Use source leases for Docker volumes.
13. Source and destination become immutable after first successful snapshot.
14. Archive policies and destinations instead of physically deleting history.
15. Store validation per execution context.
16. Restore only available snapshots.
17. Mark forgotten snapshots as expired.
18. Restore into a new volume by default.
19. Explicitly clear an existing volume before overwrite restore.
20. Citadel system restore is offline and targets an empty database.
21. Repository passwords must be preserved outside Citadel.
22. Secrets never enter logs or persisted execution payloads.
23. Keep repository check and prune separate from normal backup execution.
24. Use existing connector, permission, worker, activity, alert, JSON, migration, and frontend patterns.
25. Do not implement every slice in a single pass.
