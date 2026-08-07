using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Domain.Entities.Tags;
using System.Security.Cryptography;
using System.Text;
using Hosting.Common;

namespace Domain.Entities.SwarmServices;

public sealed class SwarmService : IAuditedEntity, IReconcilableResource
{
    private const string DockerNameFallback = "service";

    public Guid Id { get; private set; }
    public Guid PlatformId { get; private set; }
    public string Name { get; private set; }
    public string? Description { get; private set; }
    public string DockerName { get; private set; }
    public string? DockerServiceId { get; private set; }
    public SwarmServiceSpec Spec { get; private set; }
    public AutoUpdateState AutoUpdateState { get; private set; }
    public SwarmServiceHealth Health { get; private set; }
    public SwarmServiceSynchronizationState SynchronizationState { get; private set; }
    public ResourceControlState ControlState { get; private set; }
    public long? ControlStartedAt { get; private set; }
    public Guid? ControlTriggeredBy { get; private set; }
    public string DesiredSpecHash { get; private set; }
    public string? LastAppliedDesiredSpecHash { get; private set; }
    public string? LastAppliedRuntimeHash { get; private set; }
    public string? AppliedImageDigest { get; private set; }
    public long? DockerVersionIndex { get; private set; }
    public SwarmServiceOperation? CurrentOperation { get; private set; }
    public long RowVersion { get; private set; }
    public Guid CreatedByActorId { get; private set; }
    public DateTime CreatedAt { get; private set; }
    public DateTime UpdatedAt { get; private set; }
    public IReadOnlyList<TagSummary> Tags { get; private set; } = [];
    public Platform? Platform { get; private set; }
    public SwarmServiceProjection? Projection { get; private set; }

    public bool HasPendingChanges => !string.Equals(DesiredSpecHash, LastAppliedDesiredSpecHash, StringComparison.Ordinal);
    public bool HasRuntimeDrift => LastAppliedRuntimeHash is not null
        && Projection?.LiveRuntimeHash is not null
        && !string.Equals(LastAppliedRuntimeHash, Projection.LiveRuntimeHash, StringComparison.Ordinal);

    public SwarmService(
        string name,
        Guid platformId,
        Guid createdByActorId,
        SwarmServiceSpec spec,
        string? description = null,
        Guid? id = null)
    {
        Id = id ?? Guid.CreateVersion7();
        PlatformId = platformId;
        CreatedByActorId = createdByActorId;
        CreatedAt = DateTime.UtcNow;
        UpdatedAt = CreatedAt;
        Name = name.Trim();
        Description = description;
        DockerName = CreateDockerName(Name, Id);
        Spec = spec;
        DesiredSpecHash = SwarmServiceSpecHasher.Hash(spec);
        AutoUpdateState = new AutoUpdateState(DateTime.MinValue, AutoUpdateStatus.Unknown);
        Health = SwarmServiceHealth.Created;
        SynchronizationState = SwarmServiceSynchronizationState.NeverApplied;
        ControlState = ResourceControlState.Idle;
    }

    public static SwarmService FromPersistence(
        Guid id,
        Guid platformId,
        string name,
        string? description,
        string dockerName,
        string? dockerServiceId,
        SwarmServiceSpec spec,
        AutoUpdateState autoUpdateState,
        SwarmServiceHealth health,
        SwarmServiceSynchronizationState synchronizationState,
        ResourceControlState controlState,
        long? controlStartedAt,
        Guid? controlTriggeredBy,
        string desiredSpecHash,
        string? lastAppliedDesiredSpecHash,
        string? lastAppliedRuntimeHash,
        string? appliedImageDigest,
        long? dockerVersionIndex,
        SwarmServiceOperation? currentOperation,
        long rowVersion,
        Guid createdByActorId,
        DateTime createdAt,
        DateTime updatedAt,
        Platform? platform = null,
        SwarmServiceProjection? projection = null)
    {
        return new SwarmService(name, platformId, createdByActorId, spec, description, id)
        {
            DockerName = dockerName,
            DockerServiceId = dockerServiceId,
            AutoUpdateState = autoUpdateState,
            Health = health,
            SynchronizationState = synchronizationState,
            ControlState = controlState,
            ControlStartedAt = controlStartedAt,
            ControlTriggeredBy = controlTriggeredBy,
            DesiredSpecHash = desiredSpecHash,
            LastAppliedDesiredSpecHash = lastAppliedDesiredSpecHash,
            LastAppliedRuntimeHash = lastAppliedRuntimeHash,
            AppliedImageDigest = appliedImageDigest,
            DockerVersionIndex = dockerVersionIndex,
            CurrentOperation = currentOperation,
            RowVersion = rowVersion,
            CreatedAt = createdAt,
            UpdatedAt = updatedAt,
            Platform = platform,
            Projection = projection
        };
    }

    public bool UpdateSpec(SwarmServiceSpec spec)
    {
        if (LastAppliedDesiredSpecHash is not null && spec.SchedulingMode != Spec.SchedulingMode)
            return false;

        Spec = spec;
        DesiredSpecHash = SwarmServiceSpecHasher.Hash(spec);
        SynchronizationState = HasPendingChanges
            ? SwarmServiceSynchronizationState.DesiredChangesPending
            : SwarmServiceSynchronizationState.InSync;
        UpdatedAt = DateTime.UtcNow;
        return true;
    }

    public void Rename(string name)
    {
        Name = name.Trim();
        UpdatedAt = DateTime.UtcNow;
    }

    public void UpdateDescription(string? description)
    {
        Description = description;
        UpdatedAt = DateTime.UtcNow;
    }

    public bool TryPrepareOperation(
        SwarmServiceOperationKind kind,
        Guid operationId,
        Guid actorId,
        string? targetRuntimeHash = null,
        long? baseDockerVersion = null,
        long? expectedForceUpdate = null,
        string? clusterId = null)
    {
        if (ControlState == ResourceControlState.Processing
            || SynchronizationState == SwarmServiceSynchronizationState.OwnershipConflict
            || CurrentOperation?.State is SwarmServiceOperationState.OutcomeUnknown
                or SwarmServiceOperationState.OwnershipConflict)
            return false;

        var now = DateTime.UtcNow;
        CurrentOperation = new SwarmServiceOperation(
            operationId,
            kind,
            SwarmServiceOperationState.Prepared,
            baseDockerVersion,
            DesiredSpecHash,
            targetRuntimeHash,
            RowVersion,
            expectedForceUpdate,
            now,
            ClusterId: clusterId,
            ActorId: actorId);
        ControlState = ResourceControlState.Processing;
        ControlStartedAt = new DateTimeOffset(now).ToUnixTimeSeconds();
        ControlTriggeredBy = actorId;
        UpdatedAt = now;
        return true;
    }

    public void MarkOperationAttempted()
    {
        if (CurrentOperation is null || CurrentOperation.State != SwarmServiceOperationState.Prepared)
            return;

        CurrentOperation = CurrentOperation with
        {
            State = SwarmServiceOperationState.PendingAcceptance,
            AttemptedAt = DateTime.UtcNow
        };
    }

    public bool TryPrepareVersionConflictRetry(long baseDockerVersion, long? expectedForceUpdate)
    {
        if (CurrentOperation is null
            || CurrentOperation.State != SwarmServiceOperationState.PendingAcceptance
            || CurrentOperation.Kind is not (SwarmServiceOperationKind.Scale or SwarmServiceOperationKind.ForceUpdate))
        {
            return false;
        }

        CurrentOperation = CurrentOperation with
        {
            BaseDockerVersion = baseDockerVersion,
            ExpectedForceUpdate = expectedForceUpdate
        };
        DockerVersionIndex = baseDockerVersion;
        UpdatedAt = DateTime.UtcNow;
        return true;
    }

    public void MarkOperationAccepted(
        string? dockerServiceId,
        long? dockerVersion,
        IReadOnlyList<string>? warnings = null)
    {
        if (CurrentOperation is null)
            return;

        DockerServiceId = dockerServiceId ?? DockerServiceId;
        DockerVersionIndex = dockerVersion ?? DockerVersionIndex;
        CurrentOperation = CurrentOperation with
        {
            State = SwarmServiceOperationState.Accepted,
            ObservedDockerVersion = dockerVersion,
            Warnings = warnings ?? []
        };
    }

    public void CompleteOperation(
        SwarmServiceOperationState state,
        string? runtimeHash = null,
        string? appliedImageDigest = null,
        string? resultCode = null,
        string? resultMessage = null)
    {
        if (CurrentOperation is null)
            return;

        var completedAt = DateTime.UtcNow;
        CurrentOperation = CurrentOperation with
        {
            State = state,
            CompletedAt = completedAt,
            ResultCode = resultCode,
            ResultMessage = resultMessage
        };

        if (state == SwarmServiceOperationState.Completed)
        {
            if (CurrentOperation.Kind != SwarmServiceOperationKind.Delete)
            {
                LastAppliedDesiredSpecHash = CurrentOperation.TargetDesiredSpecHash;
                LastAppliedRuntimeHash = runtimeHash ?? CurrentOperation.TargetRuntimeHash;
                AppliedImageDigest = appliedImageDigest ?? AppliedImageDigest;
                SynchronizationState = SwarmServiceSynchronizationState.InSync;
            }
        }
        else if (state == SwarmServiceOperationState.OutcomeUnknown)
        {
            SynchronizationState = SwarmServiceSynchronizationState.OutcomeUnknown;
        }

        ControlState = ResourceControlState.Idle;
        ControlStartedAt = null;
        ControlTriggeredBy = null;
        UpdatedAt = completedAt;
    }

    public void ApplyObservation(SwarmServiceProjection? projection)
    {
        Projection = projection;
        if (projection is null)
        {
            Health = DockerServiceId is null
                ? SwarmServiceHealth.Created
                : SwarmServiceHealth.Unknown;
            SynchronizationState = DockerServiceId is null
                ? SwarmServiceSynchronizationState.NeverApplied
                : SwarmServiceSynchronizationState.RuntimeMissing;
            return;
        }

        DockerVersionIndex = projection.VersionIndex;
        var terminalUpdate = string.Equals(projection.UpdateState, "paused", StringComparison.OrdinalIgnoreCase)
            || string.Equals(projection.UpdateState, "rollback_paused", StringComparison.OrdinalIgnoreCase)
            || string.Equals(projection.UpdateState, "rollback_completed", StringComparison.OrdinalIgnoreCase);
        Health = projection.IsStale
            ? SwarmServiceHealth.Unknown
            : terminalUpdate
                ? SwarmServiceHealth.Failed
                : projection.DesiredTaskCount == 0 || projection.RunningTaskCount >= projection.DesiredTaskCount
                    ? SwarmServiceHealth.Healthy
                    : projection.RunningTaskCount > 0
                        ? SwarmServiceHealth.Degraded
                        : SwarmServiceHealth.Progressing;

        SynchronizationState = projection.Ownership == SwarmServiceOwnership.OwnershipConflict
            ? SwarmServiceSynchronizationState.OwnershipConflict
            : HasRuntimeDrift
                ? SwarmServiceSynchronizationState.Drifted
                : HasPendingChanges
                    ? SwarmServiceSynchronizationState.DesiredChangesPending
                    : SwarmServiceSynchronizationState.InSync;
    }

    public void SetAutoUpdateState(AutoUpdateState state) => AutoUpdateState = state;

    public void AcceptPersistedRowVersion(long rowVersion)
    {
        if (rowVersion <= RowVersion)
            throw new ArgumentOutOfRangeException(nameof(rowVersion));

        RowVersion = rowVersion;
    }

    public bool TryBeginUpdateCheck(Guid actorId, DateTimeOffset startedAt)
    {
        if (ControlState == ResourceControlState.Processing)
            return false;
        ControlState = ResourceControlState.Processing;
        ControlStartedAt = startedAt.ToUnixTimeSeconds();
        ControlTriggeredBy = actorId;
        UpdatedAt = startedAt.UtcDateTime;
        return true;
    }

    public void CompleteUpdateCheck(AutoUpdateState? state = null)
    {
        if (state is not null)
            AutoUpdateState = state;
        ControlState = ResourceControlState.Idle;
        ControlStartedAt = null;
        ControlTriggeredBy = null;
        UpdatedAt = DateTime.UtcNow;
    }
    public void AssignTags(IReadOnlyList<TagSummary> tags) => Tags = tags;

    public static string CreateDockerName(string name, Guid id)
    {
        var source = name.Trim().ToLowerInvariant();
        var builder = new StringBuilder(source.Length);
        var previousSeparator = false;

        foreach (var character in source)
        {
            var accepted = char.IsAsciiLetterOrDigit(character) || character is '_' or '-';
            var normalized = accepted ? character : '-';
            if (normalized == '-' && previousSeparator)
                continue;
            builder.Append(normalized);
            previousSeparator = normalized == '-';
        }

        var baseName = builder.ToString().Trim('-', '_');
        if (baseName.Length == 0)
            baseName = DockerNameFallback;
        if (baseName.Length > 48)
            baseName = baseName[..48].TrimEnd('-', '_');

        var stableId = id.ToString("N")[..8];
        return $"{baseName}-{stableId}";
    }
}

public sealed record SwarmServiceOperation(
    Guid Id,
    SwarmServiceOperationKind Kind,
    SwarmServiceOperationState State,
    long? BaseDockerVersion,
    string TargetDesiredSpecHash,
    string? TargetRuntimeHash,
    long TargetRowVersion,
    long? ExpectedForceUpdate,
    DateTime PreparedAt,
    DateTime? AttemptedAt = null,
    DateTime? CompletedAt = null,
    long? ObservedDockerVersion = null,
    string? ResultCode = null,
    IReadOnlyList<string>? Warnings = null,
    string? ResultMessage = null,
    string? ClusterId = null,
    Guid? ActorId = null);

public static class SwarmServiceSpecHasher
{
    public static string Hash(SwarmServiceSpec spec)
    {
        var value = new StringBuilder(1024);
        AppendImage(value, spec.Image);
        Append(value, spec.UpdateBehavior);
        Append(value, spec.SchedulingMode);
        Append(value, spec.Replicas);
        AppendSequence(value, spec.Command, preserveOrder: true);
        AppendSequence(value, spec.Arguments, preserveOrder: true);
        AppendSequence(value, spec.Environment, preserveOrder: false);
        Append(value, spec.User);
        Append(value, spec.WorkingDirectory);
        AppendHealthCheck(value, spec.HealthCheck);
        Append(value, spec.StopGracePeriodNanoseconds);
        AppendSequence(value, spec.Ports.Select(FormatPort), preserveOrder: false);
        AppendSequence(value, spec.NetworkIds, preserveOrder: false);
        AppendSequence(value, spec.Mounts.Select(FormatMount), preserveOrder: false);
        AppendSequence(value, spec.Secrets.Select(FormatSecret), preserveOrder: false);
        AppendSequence(value, spec.Configs.Select(FormatConfig), preserveOrder: false);
        AppendResources(value, spec.Resources);
        AppendSequence(value, spec.PlacementConstraints, preserveOrder: false);
        AppendRestart(value, spec.RestartPolicy);
        AppendUpdate(value, spec.UpdatePolicy);
        return Convert.ToHexStringLower(SHA256.HashData(Encoding.UTF8.GetBytes(value.ToString())));
    }

    private static void AppendImage(StringBuilder value, SwarmServiceImageInfo image)
    {
        switch (image)
        {
            case SwarmExternalImage external:
                Append(value, "External");
                Append(value, external.RegistryId);
                Append(value, external.ImageTag);
                break;
            case SwarmBuildImage build:
                Append(value, "Build");
                Append(value, build.BuildProjectId);
                Append(value, build.ResolvedImageReference);
                break;
            default:
                throw new ArgumentOutOfRangeException(nameof(image));
        }
    }

    private static void AppendHealthCheck(StringBuilder value, SwarmServiceHealthCheck? healthCheck)
    {
        if (healthCheck is null) { Append<string?>(value, null); return; }
        AppendSequence(value, healthCheck.Test, preserveOrder: true);
        Append(value, healthCheck.IntervalNanoseconds);
        Append(value, healthCheck.TimeoutNanoseconds);
        Append(value, healthCheck.Retries);
        Append(value, healthCheck.StartPeriodNanoseconds);
    }

    private static void AppendResources(StringBuilder value, SwarmServiceResources? resources)
    {
        if (resources is null) { Append<string?>(value, null); return; }
        Append(value, resources.LimitNanoCpus);
        Append(value, resources.LimitMemoryBytes);
        Append(value, resources.ReservationNanoCpus);
        Append(value, resources.ReservationMemoryBytes);
    }

    private static void AppendRestart(StringBuilder value, SwarmServiceRestartPolicy? policy)
    {
        if (policy is null) { Append<string?>(value, null); return; }
        Append(value, policy.Condition);
        Append(value, policy.DelayNanoseconds);
        Append(value, policy.MaximumAttempts);
        Append(value, policy.WindowNanoseconds);
    }

    private static void AppendUpdate(StringBuilder value, SwarmServiceUpdatePolicy? policy)
    {
        if (policy is null) { Append<string?>(value, null); return; }
        Append(value, policy.Parallelism);
        Append(value, policy.DelayNanoseconds);
        Append(value, policy.Order);
        Append(value, policy.FailureAction);
    }

    private static string FormatPort(SwarmServicePort value) =>
        $"{value.TargetPort}/{value.PublishedPort}/{value.Protocol.ToLowerInvariant()}/{value.PublishMode}";
    private static string FormatMount(SwarmServiceMount value) =>
        $"{value.Kind}/{value.Source}/{value.Target}/{value.ReadOnly}";
    private static string FormatSecret(SwarmServiceSecretReference value) =>
        $"{value.SecretId}/{value.SecretName}/{value.TargetName}";
    private static string FormatConfig(SwarmServiceConfigReference value) =>
        $"{value.ConfigId}/{value.ConfigName}/{value.TargetName}";

    private static void AppendSequence(StringBuilder builder, IEnumerable<string> values, bool preserveOrder)
    {
        var normalized = preserveOrder ? values : values.Order(StringComparer.Ordinal);
        foreach (var value in normalized)
            Append(builder, value);
        builder.Append('|');
    }

    private static void Append<T>(StringBuilder builder, T value)
    {
        var text = value?.ToString() ?? string.Empty;
        builder.Append(text.Length).Append(':').Append(text).Append(';');
    }
}

public static class SwarmServiceRuntimeHasher
{
    public static string Hash(SwarmServiceSpec spec, string resolvedImage) =>
        SwarmServiceRuntimeStateHasher.Hash(new SwarmServiceRuntimeState(
            resolvedImage,
            spec.SchedulingMode.ToString(),
            spec.Replicas,
            spec.Command,
            spec.Arguments,
            spec.Environment,
            spec.User,
            spec.WorkingDirectory,
            spec.HealthCheck?.Test ?? [],
            spec.HealthCheck?.IntervalNanoseconds,
            spec.HealthCheck?.TimeoutNanoseconds,
            spec.HealthCheck?.Retries,
            spec.HealthCheck?.StartPeriodNanoseconds,
            spec.StopGracePeriodNanoseconds,
            [.. spec.Ports.Select(static value =>
                $"{value.TargetPort}/{value.PublishedPort}/{value.Protocol.ToLowerInvariant()}/{value.PublishMode.ToString().ToLowerInvariant()}")],
            spec.NetworkIds,
            [.. spec.Mounts.Select(static value =>
                $"{value.Kind.ToString().ToLowerInvariant()}/{value.Source}/{value.Target}/{value.ReadOnly}")],
            [.. spec.Secrets.Select(static value => $"{value.SecretId}/{value.SecretName}/{value.TargetName}")],
            [.. spec.Configs.Select(static value => $"{value.ConfigId}/{value.ConfigName}/{value.TargetName}")],
            spec.Resources?.LimitNanoCpus,
            spec.Resources?.LimitMemoryBytes,
            spec.Resources?.ReservationNanoCpus,
            spec.Resources?.ReservationMemoryBytes,
            spec.PlacementConstraints,
            spec.RestartPolicy?.Condition.ToString(),
            spec.RestartPolicy?.DelayNanoseconds,
            spec.RestartPolicy?.MaximumAttempts,
            spec.RestartPolicy?.WindowNanoseconds,
            spec.UpdatePolicy?.Parallelism,
            spec.UpdatePolicy?.DelayNanoseconds,
            spec.UpdatePolicy?.Order.ToString(),
            spec.UpdatePolicy?.FailureAction.ToString()));
}
