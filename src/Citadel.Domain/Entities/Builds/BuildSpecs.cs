using Domain.Contracts.Resources;
using System.Text.Json.Serialization;

namespace Domain.Entities.Builds;

public sealed record BuildArgSpec(
    string Name,
    string? Value = null,
    Guid? ResourceBindingId = null);

public sealed record BuildSecretSpec(
    string Id,
    Guid SecretId);

public sealed record BuildPlatformSnapshot(
    Guid Id,
    string Name,
    string Address,
    PlatformConnectorType ConnectorType);

public sealed record BuildRegistrySnapshot(
    Guid Id,
    string Name,
    string RegistryHost);

public sealed record BuildRunPlatformResult(
    string? Digest,
    IReadOnlyList<string> ImageReferences,
    int ExitCode,
    string? RawMetadata = null);

[JsonPolymorphic(TypeDiscriminatorPropertyName = "$type")]
[JsonDerivedType(typeof(AwsEc2BuildAgentPoolProviderSpec), nameof(BuildAgentPoolProvider.AwsEc2))]
[JsonDerivedType(typeof(SelfManagedVmBuildAgentPoolProviderSpec), nameof(BuildAgentPoolProvider.SelfManagedVm))]
public abstract record BuildAgentPoolProviderSpec(BuildAgentPoolProvider Provider);

public sealed record AwsEc2BuildAgentPoolProviderSpec(
    string Region,
    string InstanceType,
    CpuArchitecture Architecture,
    string AmiId,
    int RootVolumeSizeGb,
    string SubnetId,
    IReadOnlyList<string> SecurityGroupIds,
    string? InstanceProfileName,
    bool AssignPublicIp,
    Guid? AwsCredentialSecretId,
    string? AssumeRoleArn,
    string? KeyPairName,
    IReadOnlyDictionary<string, string>? Tags = null)
    : BuildAgentPoolProviderSpec(BuildAgentPoolProvider.AwsEc2);

public sealed record SelfManagedVmBuildAgentPoolProviderSpec(
    string? Endpoint,
    CpuArchitecture Architecture,
    int MaxWorkers,
    Guid? RegistrationSecretId = null,
    IReadOnlyList<string>? Labels = null,
    BuildAgentPoolConnectionMode ConnectionMode = BuildAgentPoolConnectionMode.InboundAgent)
    : BuildAgentPoolProviderSpec(BuildAgentPoolProvider.SelfManagedVm);

public sealed record BuildAgentPoolSnapshot(
    Guid Id,
    string Name,
    string? Description,
    bool Enabled,
    BuildAgentPoolProvider Provider,
    BuildAgentPoolProviderSpec ProviderSpec,
    CpuArchitecture Architecture,
    string Region,
    string InstanceType,
    int MaxActiveBuilders,
    int QueueTimeoutSeconds,
    int ProvisioningTimeoutSeconds,
    int RegistrationTimeoutSeconds,
    int HeartbeatTimeoutSeconds,
    int CleanupTimeoutSeconds,
    int MaximumInstanceLifetimeSeconds,
    int FailureRetentionMinutes,
    BuildAgentPoolValidationStatus LastValidationStatus,
    string? LastValidationMessage,
    DateTimeOffset? LastValidatedAt);
