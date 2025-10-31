namespace Infrastructure.Persistence.Dtos;

internal record ContainerDto(
    Guid Id,
    Guid PlatformId,
    string DockerContainerId,
    string Name,
    string DockerImageId,
    long Created,
    long Updated,
    string State, // ContainerStateStatus
    string Ports, // List<ContainerPort> 
    string? Stack,
    Guid? ImageId = null
)
{
    public ICollection<ContainerStatDto> Stats { get; init; } = [];

    public ContainerDto() : this(Guid.Empty, Guid.Empty, string.Empty, string.Empty, string.Empty, 0, 0, string.Empty, string.Empty, string.Empty, null)
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

internal record ContainerWithImageDto(
     Guid? Image_ImageId = null,
     Guid? Image_platformId = null,
     Guid? Image_RegistryId = null,
     string? Image_Name = null,
     string? Image_Tags = null, // List<string>
     string? Image_DockerImageId = null,
     double? Image_Size = null,
     int? Image_Containers = null,
     DateTime? Image_CreatedAt = null,
     bool? Image_IsUpToDate = null,
     DateTime? Image_UpdatedAt = null
    ) : ContainerDto;

internal record ContainerWithLastStatDto(
     long? Stat_Created,
     double? Stat_MemoryActive,
     double? Stat_MemoryCache,
     double? Stat_CpuUsage,
     double? Stat_MemoryLimit,
     double? Stat_RxBytes,
     double? Stat_TxBytes
    ) : ContainerWithImageDto; 
