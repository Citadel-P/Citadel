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
    string? AgentVersion)
{
    public ICollection<PlatformStatDto> Stats { get; init; } = [];
    public PlatformDto() : this(Guid.Empty, string.Empty, string.Empty, 0, 0, 0, 0, 0, string.Empty, string.Empty, string.Empty, string.Empty, string.Empty)
    {
        
    }
}

internal sealed record PlatformStatDto(
    Guid Id,
    Guid PlatformId,
    long Created,
    double? CpuUsage,
    double? MemoryUsage,
    double? RxBytes,
    double? TxBytes);