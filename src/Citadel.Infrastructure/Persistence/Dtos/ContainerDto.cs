namespace Infrastructure.Persistence.Dtos;

internal sealed record ContainerDto(
    string Id, // Guid
    string PlatformId, // Guid
    string ContainerId,
    string Name,
    string Image,
    long Created,
    long Updated,
    string State, // ContainerStateStatus
    string Ports, // List<ContainerPort> 
    string? Stack,
    PlatformDto? Platform = null
)
{
    public ICollection<ContainerStatDto> Stats { get; init; } = [];

    public ContainerDto() : this(string.Empty, string.Empty, string.Empty, string.Empty, string.Empty, 0, 0, string.Empty, string.Empty, string.Empty, null)
    {
        
    }

}

internal sealed record ContainerStatDto(
     string Id,// Guid
     string ContainerId,// Guid
     long Created,
     double MemoryActive,
     double MemoryCache,
     double? CpuUsage,
     double? MemoryLimit,
     double? RxBytes,
     double? TxBytes
)
{
    public ContainerStatDto() : this(string.Empty, string.Empty, 0, 0, 0, 0, 0, 0, 0)
    {
        
    }
}

public sealed record PlatformContainerInfoDto(
    string Address,
    string ContainerId,
    string ConnectorType // PlatformConnectorType
)
{
    public PlatformContainerInfoDto() : this(string.Empty, string.Empty, string.Empty)
    {
        
    }
};