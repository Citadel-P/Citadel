using Domain.Entities.Platforms;

namespace Domain.Contracts.Resources.Platforms;

public sealed record PlatformSnapshot(
    Guid Id,
    string Name,
    string Address,
    string? Description,
    PlatformStatus Status,
    PlatformConnectorType ConnectorType,
    int NetworkCount,
    int VolumeCount,
    long ImageCount,
    long CpuCount,
    long MemTotal,
    string? ServerVersion,
    string? AgentVersion,
    PlatformDescriptor PlatformDescriptor);

public static class PlatformSnapshotExtensions
{
    public static PlatformSnapshot ToSnapshot(this Platform platform, Guid? id = null)
        => new(
            Id: id ?? platform.Id,
            Name: platform.Name,
            Address: platform.Address,
            Description: platform.Description,
            Status: platform.Status,
            ConnectorType: platform.ConnectorType,
            NetworkCount: platform.NetworkCount,
            VolumeCount: platform.VolumeCount,
            ImageCount: platform.ImageCount,
            CpuCount: platform.CpuCount,
            MemTotal: platform.MemTotal,
            ServerVersion: platform.ServerVersion,
            AgentVersion: platform.AgentVersion,
            PlatformDescriptor: platform.PlatformDescriptor);
}
