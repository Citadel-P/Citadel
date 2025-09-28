namespace Infrastructure.Persistence.Dtos;

internal record ContainerDto(
    Guid Id,
    Guid PlatformId,
    string ContainerId,
    string Name,
    string Image,
    string ImageId,
    long Created,
    long Updated,
    string State, // ContainerStateStatus
    string Ports, // List<ContainerPort> 
    string? Stack,
    Guid? ImageEntityId = null,
    PlatformDto? Platform = null
)
{
    public ICollection<ContainerStatDto> Stats { get; init; } = [];

    public ContainerDto() : this(Guid.Empty, Guid.Empty, string.Empty, string.Empty, string.Empty, string.Empty, 0, 0, string.Empty, string.Empty, string.Empty, null)
    {
        
    }
}

internal record struct ContainerStatDto(
     Guid Id,
     Guid ContainerId,
     long Created,
     double? MemoryActive,
     double? MemoryCache,
     double? CpuUsage,
     double? MemoryLimit,
     double? RxBytes,
     double? TxBytes
);

internal sealed record ContainerWithLastStatDto(
     long? Stat_Created,
     double? Stat_MemoryActive,
     double? Stat_MemoryCache,
     double? Stat_CpuUsage,
     double? Stat_MemoryLimit,
     double? Stat_RxBytes,
     double? Stat_TxBytes
    ) : ContainerDto; 
