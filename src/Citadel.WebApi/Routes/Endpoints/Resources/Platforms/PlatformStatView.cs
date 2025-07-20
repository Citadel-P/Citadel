namespace WebApi.Routes.Endpoints.Resources.Platforms;

public record struct PlatformStatView(
    long Created,
    double TxBytes,
    double RxBytes,
    double CpuUsage,
    double MemoryUsage);