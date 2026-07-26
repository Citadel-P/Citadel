namespace Infrastructure.Persistence.Dtos;

internal record PlatformDto(
    Guid Id,
    string Name,
    string Address,
    int NetworkCount,
    int VolumeCount,
    long ImageCount,
    long CpuCount,
    long MemTotal,
    string Status, // PlatformStatus
    string ConnectorType, // PlatformConnectorType
    string PlatformDescriptor, //PlatformDescriptor
    string? ServerVersion,
    string? AgentVersion,
    string? Description,
    long DeploymentCount = 0,
    long StackCount = 0,
    string? TagsJson = null)
{
    public ICollection<PlatformStatDto> Stats { get; init; } = [];
    public PlatformDto() : this(Guid.Empty, string.Empty, string.Empty, 0, 0, 0, 0, 0, string.Empty, string.Empty, string.Empty, string.Empty, string.Empty, null, 0, 0)
    {

    }
}

internal record struct PlatformStatDto(
    Guid Id,
    Guid PlatformId,
    long Created,
    double? CpuUsage,
    double? MemoryUsage,
    double? RxBytes,
    double? TxBytes,
    long? DiskUsedBytes,
    long? DiskTotalBytes,
    double? DiskUsage);
