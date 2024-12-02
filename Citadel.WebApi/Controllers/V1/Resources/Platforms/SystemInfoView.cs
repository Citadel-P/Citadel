namespace WebApi.Controllers.V1.Resources.Platforms;

public sealed record SystemInfoView(
    Guid Id,
    string DaemonId,
    int NetworksCount,
    int VolumesCount,
    long Containers,
    long ContainersRunning,
    long ContainersPaused,
    long ContainersStopped,
    long Images,
    string Driver,
    string OperatingSystem,
    string OsVersion,
    string OsType,
    string Architecture,
    long Ncpu,
    long MemTotal,
    string ServerVersion,
    string AgentVersion,
    SwarmInfoView SwarmInfo
    );