using Domain;
using Domain.Entities;

namespace Infrastructure.Persistence.Dtos;

internal sealed class ContainerDto
{
    public Guid Id { get; set; }
    public Guid PlatformId { get; set; }
    public string ContainerId { get; set; } = default!;
    public string Name { get; set; } = default!;
    public string Image { get; set; } = default!;
    public long Created { get; set; }
    public long Updated { get; set; }
    public ContainerStateStatus State { get; set; }
    public List<ContainerPort> Ports { get; set; } = []; 
    public string? Stack { get; set; }

    public PlatformDto? Platform { get; set; }
    public List<ContainerStatDto>? Stats { get; set; } = [];
}

internal sealed class ContainerStatDto
{
    public Guid Id { get; set; }
    public Guid ContainerId { get; set; }
    public long Created { get; set; }
    public double? MemoryUsage { get; set; }
    public double? CpuUsage { get; set; }
    public double? MemoryLimit { get; set; }
    public double? RxBytes { get; set; }
    public double? TxBytes { get; set; }
}

