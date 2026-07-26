namespace Infrastructure.Persistence.Dtos;

internal record PlatformWithSingleStatDto(
    Guid? Stat_Id,
    long? Stat_Created,
    double? Stat_CpuUsage,
    double? Stat_MemoryUsage,
    double? Stat_RxBytes,
    double? Stat_TxBytes,
    long? Stat_DiskUsedBytes,
    long? Stat_DiskTotalBytes,
    double? Stat_DiskUsage) : PlatformDto;
