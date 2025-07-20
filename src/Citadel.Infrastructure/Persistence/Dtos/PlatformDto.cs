namespace Infrastructure.Persistence.Dtos;

internal sealed record PlatformDto(
    string Id, //Guid
    string Name, //Guid
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
    public PlatformDto() : this(string.Empty, string.Empty, string.Empty, 0, 0, 0, 0, 0, string.Empty, string.Empty, string.Empty, string.Empty, string.Empty)
    {
        
    }
}

internal sealed record PlatformStatDto(
    string Id,//Guid
    string PlatformId,//Guid
    long Created,
    double? CpuUsage,
    double? MemoryUsage,
    double? RxBytes,
    double? TxBytes)
{
    public PlatformStatDto() : this (string.Empty, string.Empty, 0, 0, 0, 0, 0)
    {
        
    }
}