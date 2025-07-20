using Domain.Entities.Platforms;

namespace Domain.Contracts.Resources.Platforms;

public sealed record PlatformResult(
        string Name,
        string Address,
        int NetworkCount,
        int VolumeCount,
        long ImageCount,
        long CpuCount,
        long MemTotal,
        string? ServerVersion,
        string? AgentVersion,
        PlatformDescriptor? Descriptor,
        DockerPlatformStat? PlatformStat = null);

public sealed record DockerPlatformStat(
        long Created,
        double MemoryUsage,
        double CpuUsage,
        double RxBytes,
        double TxBytes);