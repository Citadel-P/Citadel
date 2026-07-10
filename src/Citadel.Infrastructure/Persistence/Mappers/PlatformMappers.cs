using System.Text.Json;
using Domain;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using Infrastructure.Persistence.Dtos;

namespace Infrastructure.Persistence.Mappers;

internal static class PlatformMappers
{
    internal static Platform ToDomain(this PlatformDto platform)
    {
        var result = Platform.FromPersistence(
            id: platform.Id,
            name: platform.Name,
            address: platform.Address,
            networkCount: platform.NetworkCount,
            volumeCount: platform.VolumeCount,
            imageCount: platform.ImageCount,
            cpuCount: platform.CpuCount,
            memTotal: platform.MemTotal,
            status: Enum.Parse<PlatformStatus>(platform.Status),
            connectorType: Enum.Parse<PlatformConnectorType>(platform.ConnectorType),
            platformDescriptor: JsonSerializer.Deserialize(platform.PlatformDescriptor, PlatformJsonContext.Default.PlatformDescriptor)
                ?? throw new NotImplementedException($"PlatformDescriptor is missing for platform id {platform.Id}"),
            serverVersion: platform.ServerVersion,
            agentVersion: platform.AgentVersion,
            stats: platform.Stats?.Select(ToDomain).ToList(),
            description: platform.Description,
            deploymentCount: platform.DeploymentCount,
            stackCount: platform.StackCount);

        result.AssignTags(platform.TagsJson.ToTagSummaries());
        return result;
    }

    internal static IEnumerable<Platform> ToDomain(this IEnumerable<PlatformWithSingleStatDto> platforms)
        => platforms.Select(ToDomain);

    internal static Platform ToDomain(this PlatformWithSingleStatDto platform)
    {
        var result = Platform.FromPersistence(
            id: platform.Id,
            name: platform.Name,
            address: platform.Address,
            networkCount: platform.NetworkCount,
            volumeCount: platform.VolumeCount,
            imageCount: platform.ImageCount,
            cpuCount: platform.CpuCount,
            memTotal: platform.MemTotal,
            status: Enum.Parse<PlatformStatus>(platform.Status),
            connectorType: Enum.Parse<PlatformConnectorType>(platform.ConnectorType),
            platformDescriptor: JsonSerializer.Deserialize(platform.PlatformDescriptor, PlatformJsonContext.Default.PlatformDescriptor)
              ?? throw new NotImplementedException($"PlatformDescriptor is missing for platform id {platform.Id}"),
            serverVersion: platform.ServerVersion,
            agentVersion: platform.AgentVersion,
            description: platform.Description,
            deploymentCount: platform.DeploymentCount,
            stackCount: platform.StackCount,
            stats: [new PlatformStat(
                Created: platform?.Stat_Created ?? 0,
                MemoryUsage: platform?.Stat_MemoryUsage ?? 0,
                CpuUsage: platform?.Stat_CpuUsage ?? 0,
                RxBytes: platform?.Stat_RxBytes ?? 0,
                TxBytes: platform?.Stat_TxBytes ?? 0,
                PlatformId: platform?.Id ?? Guid.Empty
                )]);

        result.AssignTags(platform.TagsJson.ToTagSummaries());
        return result;
    }

    internal static IEnumerable<PlatformStat> ToDomain(this IEnumerable<PlatformStatDto> stats)
        => stats.Select(ToDomain);

    internal static PlatformStat ToDomain(this PlatformStatDto stat)
        => new (
            PlatformId: stat.PlatformId != Guid.Empty ?stat.PlatformId : Guid.Empty,
            Created: stat.Created,
            CpuUsage: stat.CpuUsage ?? 0,
            MemoryUsage: stat.MemoryUsage ?? 0,
            RxBytes: stat.RxBytes ?? 0,
            TxBytes: stat.TxBytes ?? 0);

    internal static PlatformConnectionInfo ToDomain(this PlatformConnectionInfoDto dto)
        => new (
            Id: dto.Id,
            Name: dto.Name,
            Address: dto.Address,
            ConnectorType: Enum.Parse<PlatformConnectorType>(dto.ConnectorType)
            );
}



