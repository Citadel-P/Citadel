using Domain;
using Domain.Entities.Platforms;

namespace Infrastructure.Persistence.Dtos;

internal sealed class PlatformDto
{
    public Guid Id { get; set; }
    public string Name { get; set; } = default!;
    public string Address { get; set; } = default!;
    public int NetworkCount { get; set; }
    public int VolumeCount { get; set; }
    public long ImageCount { get; set; }
    public long CpuCount { get; set; }
    public long MemTotal { get; set; }
    public PlatformStatus Status { get; set; }
    public PlatformConnectorType ConnectorType { get; set; }
    public PlatformDescriptor PlatformDescriptor { get; set; } = default!;
    public string? ServerVersion { get; set; }
    public string? AgentVersion { get; set; }

    public List<PlatformStatDto>? Stats { get; set; } = [];
}

internal sealed class PlatformStatDto
{
    public Guid Id { get; set; }
    public Guid PlatformId { get; set; }
    public long Created { get; set; }
    public double? CpuUsage { get; set; }
    public double? MemoryUsage { get; set; }
    public double? RxBytes { get; set; }
    public double? TxBytes { get; set; }
}