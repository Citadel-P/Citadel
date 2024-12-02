namespace Infrastructure.Entities;

public class SystemInfo
{
    public Guid Id { get; private set; }

    public Guid PlatformId { get; }
    public string DaemonId { get; private set; }
    public int NetworksCount { get; private set; }
    public int VolumesCount { get; private set; }
    public long Containers { get; private set; }
    public long ContainersRunning { get; private set; }
    public long ContainersPaused { get; private set; }
    public long ContainersStopped { get; private set; }
    public long Images { get; private set; }
    public string Driver { get; private set; }
    public string OperatingSystem { get; private set; }
    public string OsVersion { get; private set; }
    public string OsType { get; private set; }
    public string Architecture { get; private set; }
    public long Ncpu { get; private set; }
    public long MemTotal { get; private set; }
    public string ServerVersion { get; private set; }
    public string AgentVersion { get; private set; }
    public SwarmInfo SwarmInfo { get; private set; }

    /// <summary>
    /// EF navigation
    /// </summary>
    public Platform Platform { get; private set; }

    public static SystemInfo Create(
         string daemonId,
         int networksCount,
         int volumesCount,
         long containers,
         long containersRunning,
         long containersPaused,
         long containersStopped,
         long images,
         string driver,
         string operatingSystem,
         string osVersion,
         string osType,
         string architecture,
         long ncpu,
         long memTotal,
         string serverVersion,
         string agentVersion,
         SwarmInfo swarmInfo
        ) => new ()
        {
            Id = Guid.CreateVersion7(),
            DaemonId = daemonId,
            NetworksCount = networksCount,
            VolumesCount = volumesCount,
            Containers = containers,
            ContainersRunning = containersRunning,
            ContainersPaused = containersPaused,
            ContainersStopped = containersStopped,
            Images = images,
            Driver = driver,
            OperatingSystem = operatingSystem,
            OsVersion = osVersion,
            OsType = osType,
            Architecture = architecture,
            Ncpu = ncpu,
            MemTotal = memTotal,
            ServerVersion = serverVersion,
            AgentVersion = agentVersion,
            SwarmInfo = swarmInfo
        };

    public void PartialUpdate(
         int? networksCount = null,
         int? volumesCount = null,
         long? containers = null,
         long? containersRunning = null,
         long? containersPaused = null,
         long? containersStopped = null,
         long? images = null,
         string driver = null,
         string operatingSystem = null,
         string osVersion = null,
         string osType = null,
         string architecture = null,
         long? ncpu = null,
         long? memTotal = null,
         string serverVersion = null,
         string agentVersion = null
        )
    {
        if (networksCount != null) NetworksCount = networksCount.Value;
        if (volumesCount != null) VolumesCount = volumesCount.Value;
        if (containers != null) Containers = containers.Value;
        if (containersRunning != null) ContainersRunning = containersRunning.Value;
        if (containersPaused != null) ContainersPaused = containersPaused.Value;
        if (containersStopped != null) ContainersStopped = containersStopped.Value;
        if (images != null) Images = images.Value;
        if (driver != null) Driver = driver;
        if (operatingSystem != null) OperatingSystem = operatingSystem;
        if (osVersion != null) OsVersion = osVersion;
        if (osType != null) OsType = osType;
        if (architecture != null) Architecture = architecture;
        if (ncpu != null) Ncpu = ncpu.Value;
        if (memTotal != null) MemTotal = memTotal.Value;
        if (serverVersion != null) ServerVersion = serverVersion;
        if (agentVersion != null) AgentVersion = agentVersion;
    }
}