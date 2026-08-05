using System.Text.Json;
using Domain;
using Domain.Entities.Platforms;
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
        value.OwnershipDiagnostic);

    internal static SwarmTaskProjection ToDomain(this SwarmTaskProjectionDto value) => new(
        value.PlatformId, value.DockerTaskId, value.VersionIndex, value.Name, value.DockerServiceId,
        value.ServiceName, value.Slot, value.DockerNodeId, value.NodeHostname, value.DesiredState, value.State,
        value.StatusMessage, value.Error, value.Image, DeserializeList(value.Ports), ToOffset(value.StatusTimestamp),
        ToOffset(value.DockerCreatedAt), ToOffset(value.DockerUpdatedAt), ToOffset(value.ObservedAt), value.IsStale);

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

    private static DateTimeOffset ToOffset(DateTime value) =>
        new(DateTime.SpecifyKind(value, DateTimeKind.Utc));

    private static DateTimeOffset? ToOffset(DateTime? value) =>
        value is null ? null : ToOffset(value.Value);

    private static IReadOnlyDictionary<string, string> DeserializeLabels(string value) =>
        JsonSerializer.Deserialize(value, PlatformJsonContext.Default.DictionaryStringString) ?? [];

    private static IReadOnlyList<string> DeserializeList(string value) =>
        JsonSerializer.Deserialize(value, PlatformJsonContext.Default.StringArray) ?? [];
}
