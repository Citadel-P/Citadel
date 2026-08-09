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
    string? ClusterId = null,
    bool PruneHistoricalSwarmTaskContainers = true,
    long DeploymentCount = 0,
    long StackCount = 0,
    long DeploymentHealthyCount = 0,
    long DeploymentDegradedCount = 0,
    long DeploymentFailedCount = 0,
    long DeploymentStoppedCount = 0,
    long DeploymentInProgressCount = 0,
    long DeploymentUnknownCount = 0,
    long StackHealthyCount = 0,
    long StackDegradedCount = 0,
    long StackFailedCount = 0,
    long StackStoppedCount = 0,
    long StackPausedCount = 0,
    long StackInProgressCount = 0,
    long StackUnknownCount = 0,
    string? TagsJson = null)
{
    public ICollection<PlatformStatDto> Stats { get; init; } = [];
    public PlatformDto() : this(Guid.Empty, string.Empty, string.Empty, 0, 0, 0, 0, 0, string.Empty, string.Empty, string.Empty, null, null, null)
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
