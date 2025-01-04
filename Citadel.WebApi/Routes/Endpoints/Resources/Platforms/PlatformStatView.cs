namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record PlatformStatView(double MemoryUsage, double CpuUsage, long Created, double RxBytes, double TxBytes);