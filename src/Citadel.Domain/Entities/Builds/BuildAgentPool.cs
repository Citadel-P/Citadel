using Domain.Entities.Tags;

namespace Domain.Entities.Builds;

public sealed class BuildAgentPool(
    string name,
    string? description,
    bool enabled,
    BuildAgentPoolProviderSpec providerSpec,
    int maxActiveBuilders,
    int queueTimeoutSeconds,
    int provisioningTimeoutSeconds,
    int registrationTimeoutSeconds,
    int heartbeatTimeoutSeconds,
    int cleanupTimeoutSeconds,
    int maximumInstanceLifetimeSeconds,
    int failureRetentionMinutes,
    Guid createdByActorId,
    BuildAgentPoolValidationStatus lastValidationStatus = BuildAgentPoolValidationStatus.NotTested,
    string? lastValidationMessage = null,
    DateTimeOffset? lastValidatedAt = null,
    ResourceControlState controlState = ResourceControlState.Idle,
    Guid? controlTriggeredBy = null,
    long? controlStartedAt = null,
    DateTimeOffset? createdAt = null,
    DateTimeOffset? updatedAt = null,
    DateTimeOffset? archivedAt = null,
    long rowVersion = 0) : IReconcilableResource
{
    public const int DefaultMaxActiveBuilders = 1;
    public const int DefaultQueueTimeoutSeconds = 3600;
    public const int DefaultProvisioningTimeoutSeconds = 600;
    public const int DefaultRegistrationTimeoutSeconds = 300;
    public const int DefaultHeartbeatTimeoutSeconds = 90;
    public const int DefaultCleanupTimeoutSeconds = 600;
    public const int DefaultMaximumInstanceLifetimeSeconds = 7200;
    public const int DefaultFailureRetentionMinutes = 0;

    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = BuildProject.NormalizeName(name);
    public string NormalizedName { get; private set; } = BuildProject.ToNormalizedName(name);
    public string? Description { get; private set; } = BuildProject.NormalizeOptional(description);
    public bool Enabled { get; private set; } = enabled;
    public BuildAgentPoolProviderSpec ProviderSpec { get; private set; } = providerSpec;
    public BuildAgentPoolProvider Provider => ProviderSpec.Provider;
    public int MaxActiveBuilders { get; private set; } = maxActiveBuilders;
    public int QueueTimeoutSeconds { get; private set; } = queueTimeoutSeconds;
    public int ProvisioningTimeoutSeconds { get; private set; } = provisioningTimeoutSeconds;
    public int RegistrationTimeoutSeconds { get; private set; } = registrationTimeoutSeconds;
    public int HeartbeatTimeoutSeconds { get; private set; } = heartbeatTimeoutSeconds;
    public int CleanupTimeoutSeconds { get; private set; } = cleanupTimeoutSeconds;
    public int MaximumInstanceLifetimeSeconds { get; private set; } = maximumInstanceLifetimeSeconds;
    public int FailureRetentionMinutes { get; private set; } = failureRetentionMinutes;
    public BuildAgentPoolValidationStatus LastValidationStatus { get; private set; } = lastValidationStatus;
    public string? LastValidationMessage { get; private set; } = BuildProject.NormalizeOptional(lastValidationMessage);
    public DateTimeOffset? LastValidatedAt { get; private set; } = lastValidatedAt?.ToUniversalTime();
    public ResourceControlState ControlState { get; private set; } = controlState;
    public Guid? ControlTriggeredBy { get; private set; } = controlTriggeredBy;
    public long? ControlStartedAt { get; private set; } = controlStartedAt;
    public Guid CreatedByActorId { get; private set; } = createdByActorId;
    public DateTimeOffset CreatedAt { get; private set; } = (createdAt ?? DateTimeOffset.UtcNow).ToUniversalTime();
    public DateTimeOffset UpdatedAt { get; private set; } = (updatedAt ?? DateTimeOffset.UtcNow).ToUniversalTime();
    public DateTimeOffset? ArchivedAt { get; private set; } = archivedAt?.ToUniversalTime();
    public long RowVersion { get; private set; } = rowVersion;
    public IReadOnlyList<TagSummary> Tags { get; private set; } = [];

    public void AssignTags(IReadOnlyList<TagSummary> tags) => Tags = tags;

    public void Rename(string name)
    {
        Name = BuildProject.NormalizeName(name);
        NormalizedName = BuildProject.ToNormalizedName(name);
        Touch();
    }

    public void Update(
        string? description,
        bool? enabled,
        BuildAgentPoolProviderSpec? providerSpec,
        int? maxActiveBuilders,
        int? queueTimeoutSeconds,
        int? provisioningTimeoutSeconds,
        int? registrationTimeoutSeconds,
        int? heartbeatTimeoutSeconds,
        int? cleanupTimeoutSeconds,
        int? maximumInstanceLifetimeSeconds,
        int? failureRetentionMinutes)
    {
        Description = BuildProject.NormalizeOptional(description);
        if (enabled.HasValue) Enabled = enabled.Value;
        if (providerSpec is not null) ProviderSpec = providerSpec;
        if (maxActiveBuilders.HasValue) MaxActiveBuilders = maxActiveBuilders.Value;
        if (queueTimeoutSeconds.HasValue) QueueTimeoutSeconds = queueTimeoutSeconds.Value;
        if (provisioningTimeoutSeconds.HasValue) ProvisioningTimeoutSeconds = provisioningTimeoutSeconds.Value;
        if (registrationTimeoutSeconds.HasValue) RegistrationTimeoutSeconds = registrationTimeoutSeconds.Value;
        if (heartbeatTimeoutSeconds.HasValue) HeartbeatTimeoutSeconds = heartbeatTimeoutSeconds.Value;
        if (cleanupTimeoutSeconds.HasValue) CleanupTimeoutSeconds = cleanupTimeoutSeconds.Value;
        if (maximumInstanceLifetimeSeconds.HasValue) MaximumInstanceLifetimeSeconds = maximumInstanceLifetimeSeconds.Value;
        if (failureRetentionMinutes.HasValue) FailureRetentionMinutes = failureRetentionMinutes.Value;
        Touch();
    }

    public void ApplyValidation(BuildAgentPoolValidationStatus status, string? message, DateTimeOffset now)
    {
        LastValidationStatus = status;
        LastValidationMessage = BuildProject.NormalizeOptional(message);
        LastValidatedAt = now.ToUniversalTime();
        Touch(now);
    }

    public void MarkProcessing(Guid controlTriggeredBy, DateTimeOffset now)
    {
        ControlTriggeredBy = controlTriggeredBy;
        ControlState = ResourceControlState.Processing;
        ControlStartedAt = now.ToUniversalTime().ToUnixTimeSeconds();
        Touch(now);
    }

    public void MarkIdle(DateTimeOffset now)
    {
        ControlState = ResourceControlState.Idle;
        ControlTriggeredBy = null;
        ControlStartedAt = null;
        Touch(now);
    }

    public void Archive(DateTimeOffset now)
    {
        Enabled = false;
        ArchivedAt ??= now.ToUniversalTime();
        Touch(now);
    }

    public BuildAgentPoolSnapshot ToSnapshot()
        => ProviderSpec switch
        {
            AwsEc2BuildAgentPoolProviderSpec aws => new BuildAgentPoolSnapshot(
                Id,
                Name,
                Description,
                Enabled,
                Provider,
                ProviderSpec,
                aws.Architecture,
                aws.Region,
                aws.InstanceType,
                MaxActiveBuilders,
                QueueTimeoutSeconds,
                ProvisioningTimeoutSeconds,
                RegistrationTimeoutSeconds,
                HeartbeatTimeoutSeconds,
                CleanupTimeoutSeconds,
                MaximumInstanceLifetimeSeconds,
                FailureRetentionMinutes,
                LastValidationStatus,
                LastValidationMessage,
                LastValidatedAt),
            SelfManagedVmBuildAgentPoolProviderSpec vm => new BuildAgentPoolSnapshot(
                Id,
                Name,
                Description,
                Enabled,
                Provider,
                ProviderSpec,
                vm.Architecture,
                vm.ConnectionMode == BuildAgentPoolConnectionMode.EdgeAgent
                    ? $"edge-build-pool://{Id:D}"
                    : vm.Endpoint ?? string.Empty,
                $"static-vm x{vm.MaxWorkers}",
                MaxActiveBuilders,
                QueueTimeoutSeconds,
                ProvisioningTimeoutSeconds,
                RegistrationTimeoutSeconds,
                HeartbeatTimeoutSeconds,
                CleanupTimeoutSeconds,
                MaximumInstanceLifetimeSeconds,
                FailureRetentionMinutes,
                LastValidationStatus,
                LastValidationMessage,
                LastValidatedAt),
            _ => new BuildAgentPoolSnapshot(
                Id,
                Name,
                Description,
                Enabled,
                Provider,
                ProviderSpec,
                CpuArchitecture.Amd64,
                string.Empty,
                string.Empty,
                MaxActiveBuilders,
                QueueTimeoutSeconds,
                ProvisioningTimeoutSeconds,
                RegistrationTimeoutSeconds,
                HeartbeatTimeoutSeconds,
                CleanupTimeoutSeconds,
                MaximumInstanceLifetimeSeconds,
                FailureRetentionMinutes,
                LastValidationStatus,
                LastValidationMessage,
                LastValidatedAt)
        };

    public void Validate()
    {
        BuildProject.ValidateName(Name, "Build pool");
        if (Description?.Length > 600) throw new ArgumentException("Build pool description cannot exceed 600 characters.", nameof(Description));
        if (MaxActiveBuilders is < 1 or > 100) throw new ArgumentException("Max active builders must be between 1 and 100.", nameof(MaxActiveBuilders));
        if (QueueTimeoutSeconds is < 60 or > 86_400) throw new ArgumentException("Queue timeout must be between 60 and 86400 seconds.", nameof(QueueTimeoutSeconds));
        if (ProvisioningTimeoutSeconds is < 60 or > 7_200) throw new ArgumentException("Provisioning timeout must be between 60 and 7200 seconds.", nameof(ProvisioningTimeoutSeconds));
        if (RegistrationTimeoutSeconds is < 30 or > 3_600) throw new ArgumentException("Registration timeout must be between 30 and 3600 seconds.", nameof(RegistrationTimeoutSeconds));
        if (HeartbeatTimeoutSeconds is < 30 or > 600) throw new ArgumentException("Heartbeat timeout must be between 30 and 600 seconds.", nameof(HeartbeatTimeoutSeconds));
        if (CleanupTimeoutSeconds is < 60 or > 7_200) throw new ArgumentException("Cleanup timeout must be between 60 and 7200 seconds.", nameof(CleanupTimeoutSeconds));
        if (MaximumInstanceLifetimeSeconds is < 300 or > 86_400) throw new ArgumentException("Maximum instance lifetime must be between 300 and 86400 seconds.", nameof(MaximumInstanceLifetimeSeconds));
        if (FailureRetentionMinutes is < 0 or > 1_440) throw new ArgumentException("Failure retention must be between 0 and 1440 minutes.", nameof(FailureRetentionMinutes));
        ValidateProviderSpec(ProviderSpec);
    }

    public static BuildAgentPool FromPersistence(
        Guid id,
        string name,
        string normalizedName,
        string? description,
        bool enabled,
        BuildAgentPoolProviderSpec providerSpec,
        int maxActiveBuilders,
        int queueTimeoutSeconds,
        int provisioningTimeoutSeconds,
        int registrationTimeoutSeconds,
        int heartbeatTimeoutSeconds,
        int cleanupTimeoutSeconds,
        int maximumInstanceLifetimeSeconds,
        int failureRetentionMinutes,
        BuildAgentPoolValidationStatus lastValidationStatus,
        string? lastValidationMessage,
        DateTimeOffset? lastValidatedAt,
        ResourceControlState controlState,
        Guid? controlTriggeredBy,
        long? controlStartedAt,
        Guid createdByActorId,
        DateTimeOffset createdAt,
        DateTimeOffset updatedAt,
        DateTimeOffset? archivedAt,
        long rowVersion)
        => new(
            name,
            description,
            enabled,
            providerSpec,
            maxActiveBuilders,
            queueTimeoutSeconds,
            provisioningTimeoutSeconds,
            registrationTimeoutSeconds,
            heartbeatTimeoutSeconds,
            cleanupTimeoutSeconds,
            maximumInstanceLifetimeSeconds,
            failureRetentionMinutes,
            createdByActorId,
            lastValidationStatus,
            lastValidationMessage,
            lastValidatedAt,
            controlState,
            controlTriggeredBy,
            controlStartedAt,
            createdAt,
            updatedAt,
            archivedAt,
            rowVersion)
        {
            Id = id,
            NormalizedName = normalizedName
        };

    private static void ValidateProviderSpec(BuildAgentPoolProviderSpec providerSpec)
    {
        switch (providerSpec)
        {
            case AwsEc2BuildAgentPoolProviderSpec aws:
                if (string.IsNullOrWhiteSpace(aws.Region)) throw new ArgumentException("AWS region is required.", nameof(ProviderSpec));
                if (string.IsNullOrWhiteSpace(aws.InstanceType)) throw new ArgumentException("AWS instance type is required.", nameof(ProviderSpec));
                if (string.IsNullOrWhiteSpace(aws.AmiId)) throw new ArgumentException("AWS AMI ID is required.", nameof(ProviderSpec));
                if (aws.RootVolumeSizeGb is < 8 or > 16_384) throw new ArgumentException("AWS root volume size must be between 8 and 16384 GB.", nameof(ProviderSpec));
                if (string.IsNullOrWhiteSpace(aws.SubnetId)) throw new ArgumentException("AWS subnet ID is required.", nameof(ProviderSpec));
                if (aws.SecurityGroupIds.Count == 0) throw new ArgumentException("At least one AWS security group is required.", nameof(ProviderSpec));
                break;
            case SelfManagedVmBuildAgentPoolProviderSpec vm:
                switch (vm.ConnectionMode)
                {
                    case BuildAgentPoolConnectionMode.InboundAgent:
                        if (string.IsNullOrWhiteSpace(vm.Endpoint))
                            throw new ArgumentException("Self-managed VM endpoint is required.", nameof(ProviderSpec));
                        break;
                    case BuildAgentPoolConnectionMode.EdgeAgent:
                        break;
                    default:
                        throw new ArgumentException("Self-managed VM connection mode is invalid.", nameof(ProviderSpec));
                }

                if (vm.MaxWorkers is < 1 or > 100) throw new ArgumentException("Self-managed VM workers must be between 1 and 100.", nameof(ProviderSpec));
                break;
            default:
                throw new ArgumentException("Build pool provider is unsupported.", nameof(ProviderSpec));
        }
    }

    private void Touch(DateTimeOffset? now = null)
    {
        UpdatedAt = (now ?? DateTimeOffset.UtcNow).ToUniversalTime();
        RowVersion++;
    }
}
