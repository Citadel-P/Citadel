namespace WebApi.Controllers.V1.Resources.Platforms;

public sealed record SystemInfoView
{
    public Guid Id { get; set; }

    public string DaemonId { get; set; }

    public short NetworksCount { get; set; }

    public short VolumesCount { get; set; }

    public long Containers { get; set; }

    public long ContainersRunning { get; set; }

    public long ContainersPaused { get; set; }

    public long ContainersStopped { get; set; }

    public long Images { get; set; }

    public string Driver { get; set; }

    public string OperatingSystem { get; set; }

    public string OSVersion { get; set; }

    public string OSType { get; set; }

    public string Architecture { get; set; }

    public long NCPU { get; set; }

    public long MemTotal { get; set; }

    public string ServerVersion { get; set; }

    public string AgentVersion { get; set; }

    // Will be populated only if extended infos are requested
    public IList<string> Warnings { get; set; }

    public SwarmInfoView SwarmInfo { get; set; }
}