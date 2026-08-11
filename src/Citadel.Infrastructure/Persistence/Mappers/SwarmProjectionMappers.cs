using System.Text.Json;
using Domain;
using Domain.Entities.Platforms;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Volumes;
using Infrastructure.Persistence.Dtos;

namespace Infrastructure.Persistence.Mappers;

internal static class SwarmProjectionMappers
{
    internal static SwarmNodeProjection ToDomain(this SwarmNodeProjectionDto value) => new(
        value.PlatformId, value.DockerNodeId, value.VersionIndex, value.Hostname, value.Role, value.IsLeader,
        value.Reachability, value.Status, value.StatusMessage, value.Availability, value.EngineVersion,
        value.OperatingSystem, value.Architecture, value.Address, DeserializeLabels(value.Labels),
        value.RunningTaskCount, value.DesiredTaskCount, ToOffset(value.DockerCreatedAt), ToOffset(value.DockerUpdatedAt),
        ToOffset(value.ObservedAt), value.IsStale);

    internal static SwarmServiceProjection ToDomain(this SwarmServiceProjectionDto value) => new(
        value.PlatformId, value.DockerServiceId, value.VersionIndex, value.Name, value.Mode, value.Image,
        value.RunningTaskCount, value.DesiredTaskCount, value.UpdateState, value.UpdateMessage,
        DeserializeList(value.Ports), DeserializeList(value.NetworkIds), DeserializeList(value.SecretIds),
        DeserializeList(value.ConfigIds), DeserializeLabels(value.Labels), ToOffset(value.DockerCreatedAt),
        ToOffset(value.DockerUpdatedAt), ToOffset(value.ObservedAt), value.IsStale,
        Enum.Parse<SwarmServiceOwnership>(value.Ownership), value.DockerStackNamespace,
        value.OwnershipDiagnostic, value.SwarmServiceId, value.LiveRuntimeHash, value.ForceUpdate, value.StackId);

    internal static SwarmTaskProjection ToDomain(this SwarmTaskProjectionDto value) => new(
        value.PlatformId, value.DockerTaskId, value.VersionIndex, value.Name, value.DockerServiceId,
        value.ServiceName, value.Slot, value.DockerNodeId, value.NodeHostname, value.DesiredState, value.State,
        value.StatusMessage, value.Error, value.Image, DeserializeList(value.Ports), ToOffset(value.StatusTimestamp),
        ToOffset(value.DockerCreatedAt), ToOffset(value.DockerUpdatedAt), ToOffset(value.ObservedAt), value.IsStale,
        value.DockerContainerId);

    internal static SwarmNetworkProjection ToDomain(this SwarmNetworkProjectionDto value) => new(
        value.PlatformId, value.DockerNetworkId, value.Name, value.Scope, value.Driver, value.IsAttachable,
        value.IsInternal, value.IsIngress, value.IsEncrypted, value.EnableIPv6, DeserializeList(value.Subnets),
        DeserializeList(value.ServiceNames), DeserializeLabels(value.Labels), ToOffset(value.DockerCreatedAt),
        ToOffset(value.ObservedAt), value.IsStale);

    internal static SwarmSecretProjection ToDomain(this SwarmSecretProjectionDto value) => new(
        value.PlatformId, value.DockerSecretId, value.VersionIndex, value.Name, value.Driver,
        DeserializeList(value.ServiceNames), DeserializeLabels(value.Labels), ToOffset(value.DockerCreatedAt),
        ToOffset(value.DockerUpdatedAt), ToOffset(value.ObservedAt), value.IsStale);

    internal static SwarmConfigProjection ToDomain(this SwarmConfigProjectionDto value) => new(
        value.PlatformId, value.DockerConfigId, value.VersionIndex, value.Name, value.TemplatingDriver,
        DeserializeList(value.ServiceNames), DeserializeLabels(value.Labels), ToOffset(value.DockerCreatedAt),
        ToOffset(value.DockerUpdatedAt), ToOffset(value.ObservedAt), value.IsStale);

    internal static SwarmNodeRuntimeProjectionState ToDomain(this SwarmNodeRuntimeProjectionStateDto value) => new(
        value.PlatformId, value.DockerNodeId, value.ReconciliationGeneration,
        ToOffset(value.ReconciliationStartedAt), ToOffset(value.ReconciliationCompletedAt),
        ToOffset(value.LastSuccessfulReconciliationAt), value.IsStale, ToOffset(value.StaleSince),
        value.StaleReason, ToOffset(value.LastEventStreamConnectedAt), ToOffset(value.LastEventGapAt),
        ToOffset(value.LastStatsSampleAt), value.AgentVersion, value.DockerVersion);

    internal static SwarmNodeImageProjection ToDomain(this SwarmNodeImageProjectionDto value)
    {
        var resource = JsonSerializer.Deserialize(value.Resource, PlatformJsonContext.Default.ImageResult)
                       ?? throw new InvalidDataException("A persisted Swarm Node Image projection has no payload.");
        ApplyNodeMetadata(resource, value.DockerNodeId, value.NodeHostname, value.IsStale, value.StaleReason);
        return new SwarmNodeImageProjection(
            value.Id, value.PlatformId, value.DockerNodeId, value.DockerImageId, value.ContentIdentity,
            resource, ToOffset(value.ObservedAt), value.IsStale, value.NodeHostname, value.StaleReason);
    }

    internal static SwarmNodeVolumeProjection ToDomain(this SwarmNodeVolumeProjectionDto value)
    {
        var resource = JsonSerializer.Deserialize(value.Resource, PlatformJsonContext.Default.DockerVolumeResult)
                       ?? throw new InvalidDataException("A persisted Swarm Node Volume projection has no payload.");
        resource.PlatformId = value.PlatformId;
        ApplyNodeMetadata(resource, value.DockerNodeId, value.NodeHostname, value.IsStale, value.StaleReason);
        return new SwarmNodeVolumeProjection(
            value.PlatformId, value.DockerNodeId, value.VolumeName, resource,
            ToOffset(value.ObservedAt), value.IsStale, value.NodeHostname, value.StaleReason);
    }

    internal static SwarmNodeNetworkProjection ToDomain(this SwarmNodeNetworkProjectionDto value)
    {
        var resource = JsonSerializer.Deserialize(value.Resource, PlatformJsonContext.Default.DockerNetworkResult)
                       ?? throw new InvalidDataException("A persisted Swarm Node Network projection has no payload.");
        resource.PlatformId = value.PlatformId;
        ApplyNodeMetadata(resource, value.DockerNodeId, value.NodeHostname, value.IsStale, value.StaleReason);
        return new SwarmNodeNetworkProjection(
            value.PlatformId, value.DockerNodeId, value.DockerNetworkId, resource,
            ToOffset(value.ObservedAt), value.IsStale, value.NodeHostname, value.StaleReason);
    }

    private static void ApplyNodeMetadata(
        ImageResult resource,
        string dockerNodeId,
        string? nodeHostname,
        bool isStale,
        string? staleReason)
    {
        resource.DockerNodeId = dockerNodeId;
        resource.NodeHostname = nodeHostname;
        resource.IsStale = isStale;
        resource.StaleReason = staleReason;
    }

    private static void ApplyNodeMetadata(
        DockerVolumeResult resource,
        string dockerNodeId,
        string? nodeHostname,
        bool isStale,
        string? staleReason)
    {
        resource.DockerNodeId = dockerNodeId;
        resource.NodeHostname = nodeHostname;
        resource.IsStale = isStale;
        resource.StaleReason = staleReason;
    }

    private static void ApplyNodeMetadata(
        DockerNetworkResult resource,
        string dockerNodeId,
        string? nodeHostname,
        bool isStale,
        string? staleReason)
    {
        resource.DockerNodeId = dockerNodeId;
        resource.NodeHostname = nodeHostname;
        resource.IsStale = isStale;
        resource.StaleReason = staleReason;
    }

    private static DateTimeOffset ToOffset(DateTime value) =>
        new(DateTime.SpecifyKind(value, DateTimeKind.Utc));

    private static DateTimeOffset? ToOffset(DateTime? value) =>
        value is null ? null : ToOffset(value.Value);

    private static IReadOnlyDictionary<string, string> DeserializeLabels(string value) =>
        JsonSerializer.Deserialize(value, PlatformJsonContext.Default.DictionaryStringString) ?? [];

    private static IReadOnlyList<string> DeserializeList(string value) =>
        JsonSerializer.Deserialize(value, PlatformJsonContext.Default.StringArray) ?? [];
}
