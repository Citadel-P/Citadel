namespace WebApi.Controllers.V1.Resources.Containers;

public sealed record ContainerStatView(
        double? MemoryUsage,
        double? CpuUsage,
        double? MemoryLimit,
        ulong? RxBytes,
        ulong? TxBytes,
        DateTime? CreatedAtUtc);