using Domain.Contracts.Resources.Swarm;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Volumes;

namespace Domain.Entities.Platforms;

public sealed record SwarmServiceProjection(
    Guid PlatformId, string DockerServiceId, long VersionIndex, string Name, string Mode,
    string Image, int RunningTaskCount, int DesiredTaskCount, string UpdateState,
    string? UpdateMessage, IReadOnlyList<string> Ports, IReadOnlyList<string> NetworkIds,
    IReadOnlyList<string> SecretIds, IReadOnlyList<string> ConfigIds,
    IReadOnlyDictionary<string, string> Labels, DateTimeOffset? DockerCreatedAt,
    DateTimeOffset? DockerUpdatedAt, DateTimeOffset ObservedAt, bool IsStale,
    SwarmServiceOwnership Ownership = SwarmServiceOwnership.Unmanaged,
    string? DockerStackNamespace = null,
    string? OwnershipDiagnostic = null,
    Guid? SwarmServiceId = null,
    string? LiveRuntimeHash = null,
    long ForceUpdate = 0,
    Guid? StackId = null)
{
    public static SwarmServiceProjection FromObservation(Guid platformId, SwarmServiceResult value, DateTimeOffset observedAt) =>
        new(platformId, value.Id, value.VersionIndex, value.Name, value.Mode, value.Image,
            value.RunningTaskCount, value.DesiredTaskCount, value.UpdateState, value.UpdateMessage,
            value.Ports, value.NetworkIds, value.SecretIds, value.ConfigIds, value.Labels,
            value.CreatedAt, value.UpdatedAt, observedAt, false,
            SwarmServiceOwnershipClassifier.Classify(value.Labels),
            SwarmServiceOwnershipClassifier.GetDockerStackNamespace(value.Labels),
            SwarmServiceOwnershipClassifier.GetDiagnostic(value.Labels),
            SwarmServiceOwnershipClassifier.GetSwarmServiceId(value.Labels),
            value.RuntimeHash,
            value.ForceUpdate);
}

internal static class SwarmServiceOwnershipClassifier
{
    private const string CitadelPrefix = "com.citadel.";
    private const string ManagedLabel = CitadelPrefix + "managed";
    private const string DeploymentIdLabel = CitadelPrefix + "deployment-id";
    private const string StackIdLabel = CitadelPrefix + "stack-id";
    private const string ServiceIdLabel = CitadelPrefix + "service-id";
    private const string SystemLabel = CitadelPrefix + "system";
    private const string SystemRoleLabel = CitadelPrefix + "system-role";
    private const string DockerStackNamespaceLabel = "com.docker.stack.namespace";

    public static SwarmServiceOwnership Classify(IReadOnlyDictionary<string, string> labels)
    {
        if (labels.TryGetValue(SystemLabel, out var system)
            && string.Equals(system, "true", StringComparison.OrdinalIgnoreCase)
            && labels.ContainsKey(SystemRoleLabel))
        {
            return SwarmServiceOwnership.System;
        }

        if (!IsCitadelManaged(labels))
            return GetDockerStackNamespace(labels) is null
                ? SwarmServiceOwnership.Unmanaged
                : SwarmServiceOwnership.DockerStackExternal;

        var hasService = labels.ContainsKey(ServiceIdLabel);
        var hasOtherOwner = labels.ContainsKey(DeploymentIdLabel) || labels.ContainsKey(StackIdLabel);
        if (hasService)
        {
            return hasOtherOwner || GetSwarmServiceId(labels) is null
                ? SwarmServiceOwnership.OwnershipConflict
                : SwarmServiceOwnership.CitadelService;
        }


        if (labels.ContainsKey(StackIdLabel))
        {
            return GetDockerStackNamespace(labels) is not null && GetStackId(labels) is not null
                ? SwarmServiceOwnership.CitadelStack
                : SwarmServiceOwnership.OwnershipConflict;
        }

        return GetDockerStackNamespace(labels) is not null
            ? SwarmServiceOwnership.DockerStackExternal
            : SwarmServiceOwnership.Unmanaged;
    }

    public static Guid? GetSwarmServiceId(IReadOnlyDictionary<string, string> labels) =>
        labels.TryGetValue(ServiceIdLabel, out var value) && Guid.TryParse(value, out var id)
            ? id
            : null;

    public static Guid? GetStackId(IReadOnlyDictionary<string, string> labels) =>
        labels.TryGetValue(StackIdLabel, out var value) && Guid.TryParse(value, out var id)
            ? id
            : null;

    public static string? GetDockerStackNamespace(IReadOnlyDictionary<string, string> labels) =>
        labels.TryGetValue(DockerStackNamespaceLabel, out var value) && !string.IsNullOrWhiteSpace(value)
            ? value
            : null;

    public static string? GetDiagnostic(IReadOnlyDictionary<string, string> labels)
    {
        if (!IsCitadelManaged(labels))
            return null;
        if (labels.ContainsKey(ServiceIdLabel) && GetSwarmServiceId(labels) is null)
            return "Invalid Citadel Service ownership label";
        if (labels.ContainsKey(ServiceIdLabel)
            && (labels.ContainsKey(DeploymentIdLabel) || labels.ContainsKey(StackIdLabel)))
            return "Conflicting Citadel ownership labels";
        if (labels.ContainsKey(StackIdLabel) && GetStackId(labels) is null)
            return "Invalid Citadel Stack ownership label";
        if (labels.ContainsKey(StackIdLabel) && GetDockerStackNamespace(labels) is null)
            return "Citadel Stack ownership is missing the Docker Stack namespace";
        if (labels.ContainsKey(StackIdLabel))
            return null;
        return HasCitadelOwnerMetadata(labels) ? "Orphaned Citadel metadata" : null;
    }

    private static bool HasCitadelOwnerMetadata(IReadOnlyDictionary<string, string> labels) =>
        IsCitadelManaged(labels)
        && (labels.ContainsKey(DeploymentIdLabel) || labels.ContainsKey(StackIdLabel));

    private static bool IsCitadelManaged(IReadOnlyDictionary<string, string> labels) =>
        labels.TryGetValue(ManagedLabel, out var managed)
        && string.Equals(managed, "true", StringComparison.OrdinalIgnoreCase);
}

public sealed record SwarmTaskProjection(
    Guid PlatformId, string DockerTaskId, long VersionIndex, string Name, string DockerServiceId,
    string ServiceName, int? Slot, string DockerNodeId, string NodeHostname, string DesiredState,
    string State, string? StatusMessage, string? Error, string Image, IReadOnlyList<string> Ports,
    DateTimeOffset? StatusTimestamp, DateTimeOffset? DockerCreatedAt, DateTimeOffset? DockerUpdatedAt,
    DateTimeOffset ObservedAt, bool IsStale, string? DockerContainerId = null)
{
    public static SwarmTaskProjection FromObservation(
        Guid platformId, SwarmTaskResult value, string serviceName, string nodeHostname, DateTimeOffset observedAt) =>
        new(platformId, value.Id, value.VersionIndex, value.Name, value.ServiceId, serviceName,
            value.Slot, value.NodeId, nodeHostname, value.DesiredState, value.State, value.StatusMessage,
            value.Error, value.Image, value.Ports, value.StatusTimestamp, value.CreatedAt, value.UpdatedAt,
            observedAt, false, value.ContainerId);
}

public sealed record SwarmNetworkProjection(
    Guid PlatformId, string DockerNetworkId, string Name, string Scope, string Driver,
    bool IsAttachable, bool IsInternal, bool IsIngress, bool IsEncrypted, bool EnableIPv6,
    IReadOnlyList<string> Subnets, IReadOnlyList<string> ServiceNames, IReadOnlyDictionary<string, string> Labels,
    DateTimeOffset? DockerCreatedAt, DateTimeOffset ObservedAt, bool IsStale)
{
    public static SwarmNetworkProjection FromObservation(Guid platformId, SwarmNetworkResult value, IReadOnlyList<string> serviceNames, DateTimeOffset observedAt) =>
        new(platformId, value.Id, value.Name, value.Scope, value.Driver, value.IsAttachable,
            value.IsInternal, value.IsIngress, value.IsEncrypted, value.EnableIPv6, value.Subnets, serviceNames,
            value.Labels, value.CreatedAt, observedAt, false);
}

public sealed record SwarmSecretProjection(
    Guid PlatformId, string DockerSecretId, long VersionIndex, string Name, string? Driver,
    IReadOnlyList<string> ServiceNames, IReadOnlyDictionary<string, string> Labels, DateTimeOffset? DockerCreatedAt,
    DateTimeOffset? DockerUpdatedAt, DateTimeOffset ObservedAt, bool IsStale)
{
    public static SwarmSecretProjection FromObservation(Guid platformId, SwarmSecretResult value, IReadOnlyList<string> serviceNames, DateTimeOffset observedAt) =>
        new(platformId, value.Id, value.VersionIndex, value.Name, value.Driver, serviceNames, value.Labels,
            value.CreatedAt, value.UpdatedAt, observedAt, false);
}

public sealed record SwarmConfigProjection(
    Guid PlatformId, string DockerConfigId, long VersionIndex, string Name, string? TemplatingDriver,
    IReadOnlyList<string> ServiceNames, IReadOnlyDictionary<string, string> Labels, DateTimeOffset? DockerCreatedAt,
    DateTimeOffset? DockerUpdatedAt, DateTimeOffset ObservedAt, bool IsStale)
{
    public static SwarmConfigProjection FromObservation(Guid platformId, SwarmConfigResult value, IReadOnlyList<string> serviceNames, DateTimeOffset observedAt) =>
        new(platformId, value.Id, value.VersionIndex, value.Name, value.TemplatingDriver, serviceNames, value.Labels,
            value.CreatedAt, value.UpdatedAt, observedAt, false);
}

public sealed record SwarmProjectionSnapshot(
    IReadOnlyList<SwarmNodeProjection> Nodes,
    IReadOnlyList<SwarmServiceProjection> Services,
    IReadOnlyList<SwarmTaskProjection> Tasks,
    IReadOnlyList<SwarmNetworkProjection> Networks,
    IReadOnlyList<SwarmSecretProjection> Secrets,
    IReadOnlyList<SwarmConfigProjection> Configs);

public sealed record SwarmProjectionSummary(
    bool IsStale,
    int NodeCount,
    int ManagerCount,
    int ReachableManagerCount,
    bool HasLeader,
    bool IsManagerInventoryStale,
    int ServiceCount,
    PlatformWorkloadStatusCounts ServiceStatusCounts,
    int RunningTaskCount,
    int DesiredTaskCount,
    int NetworkCount,
    int LocalNetworkCount,
    int VolumeCount,
    int ImageCount);

public sealed record SwarmNodeLocalResourceSnapshot(
    Guid PlatformId,
    IReadOnlyList<SwarmNodeImageProjection> Images,
    IReadOnlyList<SwarmNodeVolumeProjection> Volumes,
    IReadOnlyList<SwarmNodeNetworkProjection> Networks);

public sealed record SwarmNodeRuntimeProjectionState(
    Guid PlatformId,
    string DockerNodeId,
    long ReconciliationGeneration,
    DateTimeOffset? ReconciliationStartedAt,
    DateTimeOffset? ReconciliationCompletedAt,
    DateTimeOffset? LastSuccessfulReconciliationAt,
    bool IsStale,
    DateTimeOffset? StaleSince,
    string? StaleReason,
    DateTimeOffset? LastEventStreamConnectedAt,
    DateTimeOffset? LastEventGapAt,
    DateTimeOffset? LastStatsSampleAt,
    string? AgentVersion,
    string? DockerVersion);

public sealed record SwarmNodeImageProjection(
    Guid Id,
    Guid PlatformId,
    string DockerNodeId,
    string DockerImageId,
    string ContentIdentity,
    ImageResult Resource,
    DateTimeOffset ObservedAt,
    bool IsStale,
    string? NodeHostname = null,
    string? StaleReason = null)
{
    public static SwarmNodeImageProjection FromObservation(
        Guid platformId,
        string dockerNodeId,
        ImageResult resource,
        DateTimeOffset observedAt)
    {
        var digest = resource.RepoDigests?.FirstOrDefault(static value => !string.IsNullOrWhiteSpace(value));
        return new(
            Guid.CreateVersion7(),
            platformId,
            dockerNodeId,
            resource.Id,
            digest ?? resource.Id,
            resource,
            observedAt,
            false);
    }
}

public sealed record SwarmNodeVolumeProjection(
    Guid PlatformId,
    string DockerNodeId,
    string VolumeName,
    DockerVolumeResult Resource,
    DateTimeOffset ObservedAt,
    bool IsStale,
    string? NodeHostname = null,
    string? StaleReason = null)
{
    public static SwarmNodeVolumeProjection FromObservation(
        Guid platformId,
        string dockerNodeId,
        DockerVolumeResult resource,
        DateTimeOffset observedAt) =>
        new(platformId, dockerNodeId, resource.Name, resource, observedAt, false);
}

public sealed record SwarmNodeNetworkProjection(
    Guid PlatformId,
    string DockerNodeId,
    string DockerNetworkId,
    DockerNetworkResult Resource,
    DateTimeOffset ObservedAt,
    bool IsStale,
    string? NodeHostname = null,
    string? StaleReason = null)
{
    public static SwarmNodeNetworkProjection FromObservation(
        Guid platformId,
        string dockerNodeId,
        DockerNetworkResult resource,
        DateTimeOffset observedAt) =>
        new(platformId, dockerNodeId, resource.Id, resource, observedAt, false);
}
