namespace Infrastructure.Persistence.Dtos;

internal sealed record PlatformWithSingleStatDto : PlatformDto
{
    public Guid? Stat_Id { get; init; }
    public long? Stat_Created { get; init; }
    public double? Stat_CpuUsage { get; init; }
    public double? Stat_MemoryUsage { get; init; }
    public double? Stat_RxBytes { get; init; }
    public double? Stat_TxBytes { get; init; }
    public long? Stat_DiskUsedBytes { get; init; }
    public long? Stat_DiskTotalBytes { get; init; }
    public double? Stat_DiskUsage { get; init; }
}
