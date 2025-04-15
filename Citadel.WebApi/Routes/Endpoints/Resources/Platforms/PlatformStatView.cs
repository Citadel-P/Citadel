namespace WebApi.Routes.Endpoints.Resources.Platforms;

public record struct PlatformStatView(
    double MemoryUsage,
    double CpuUsage, 
    long Created,
    double RxBytes,
    double TxBytes);