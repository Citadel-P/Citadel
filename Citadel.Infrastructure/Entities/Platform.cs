namespace Infrastructure.Entities;

public class Platform
{
    public Guid Id { get; private set; }
    public string Name { get; private set; } = null!;
    public string Address { get; private set; } = null!;
    public string DaemonId { get; private set; } = null!;
    public PlatformStatus Status { get; private set; }
    public int NetworksCount { get; private set; }
    public int VolumesCount { get; private set; }
    public long Containers { get; private set; }
    public long ContainersRunning { get; private set; }
    public long ContainersPaused { get; private set; }
    public long ContainersStopped { get; private set; }
    public long Images { get; private set; }
    public string? Driver { get; private set; }
    public string? OperatingSystem { get; private set; }
    public string? OsVersion { get; private set; }
    public string? OsType { get; private set; }
    public string? Architecture { get; private set; }
    public long Ncpu { get; private set; }
    public long MemTotal { get; private set; }
    public string? ServerVersion { get; private set; }
    public string? AgentVersion { get; private set; }
    public SwarmInfo SwarmInfo { get; private set; } = null!;
    public ICollection<ContainerInfo> ContainersInfo { get; private set; } = [];
    public ICollection<PlatformStat> Stats { get; private set; } = [];

    public static Platform Create(
        string name,
        string address,
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
        SwarmInfo swarmInfo,
        IEnumerable<PlatformStat>? stats = null)
        => new () { 
            Id = Guid.CreateVersion7(),
            Name = name, 
            Address = address,
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
            SwarmInfo = swarmInfo,
            Stats = stats?.ToList() ?? [],
            Status = PlatformStatus.Online
        };

    public void PartialUpdate(
        string? name = null,
        string? address = null,
        string? daemonId = null,
        int? networksCount = null,
        int? volumesCount = null,
        long? containers = null,
        long? containersRunning = null,
        long? containersPaused = null,
        long? containersStopped = null,
        long? images = null,
        string? driver = null,
        string? operatingSystem = null,
        string? osVersion = null,
        string? osType = null,
        string? architecture = null,
        long? ncpu = null,
        long? memTotal = null,
        string? serverVersion = null,
        string? agentVersion = null,
        PlatformStatus? platformStatus = null)
    {
        if (name != null) Name = name;
        if (address != null) Address = address;
        if (daemonId != null) DaemonId = daemonId;
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
        if (platformStatus != null) Status = platformStatus.Value;
    }

    public void AppendStats(IEnumerable<PlatformStat> stats)
    {
        if (stats == null || !stats.Any()) return;
        
        foreach (var stat in stats)
        {
            Stats.Add(stat);
        }
    }
}