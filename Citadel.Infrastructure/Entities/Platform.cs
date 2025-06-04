namespace Infrastructure.Entities;

public class Platform
{
    public Guid Id { get; private set; }
    public string Name { get; private set; } = null!;
    public string Address { get; private set; } = null!;
    public PlatformType Type { get; private set; }
    public PlatformStatus Status { get; private set; }
    public int NetworkCount { get; private set; }
    public int VolumeCount { get; private set; }
    public long ImageCount { get; private set; }
    public long CpuCount { get; private set; }
    public long MemTotal { get; private set; }
    public string? AgentVersion { get; private set; }
    public string? ServerVersion { get; private set; }
    public ICollection<PlatformStat> Stats { get; private set; } = [];
    public PlatformDescriptor PlatformDescriptor { get; private set; } = null!;

    public static Platform Create(
        string name,
        string address,
        int networkCount,
        int volumeCount,
        long imageCount,
        long cpuCount,
        long memTotal,
        string serverVersion,
        string agentVersion,
        PlatformDescriptor descriptor,
        IEnumerable<PlatformStat>? stats = null)
    {
        return new()
        {
            Id = Guid.CreateVersion7(),
            Name = name,
            Address = address,
            NetworkCount = networkCount,
            VolumeCount = volumeCount,
            ImageCount = imageCount,
            CpuCount = cpuCount,
            MemTotal = memTotal,
            ServerVersion = serverVersion,
            AgentVersion = agentVersion,
            PlatformDescriptor = descriptor,
            Stats = stats?.ToList() ?? [],
            Status = PlatformStatus.Online
        };
    }

    public void PartialUpdate(
        string? name = null,
        string? address = null,
        int? networkCount = null,
        int? volumeCount = null,
        long? containersRunning = null,
        long? containersPaused = null,
        long? containersStopped = null,
        long? imageCount = null,
        long? cpuCount = null,
        long? memTotal = null,
        string? serverVersion = null,
        string? agentVersion = null,
        PlatformDescriptor? descriptor = null,
        PlatformStatus? platformStatus = null)
    {
        if (name != null) Name = name;
        if (address != null) Address = address;
        if (networkCount != null) NetworkCount = networkCount.Value;
        if (volumeCount != null) VolumeCount = volumeCount.Value;
        if (imageCount != null) ImageCount = imageCount.Value;
        if (cpuCount != null) CpuCount = cpuCount.Value;
        if (memTotal != null) MemTotal = memTotal.Value;
        if (serverVersion != null) ServerVersion = serverVersion;
        if (agentVersion != null) AgentVersion = agentVersion;
        if (platformStatus != null) Status = platformStatus.Value;
        if (descriptor != null) PartialUpdate(descriptor);
    }

    private void PartialUpdate(PlatformDescriptor descriptor)
    {
        // Todo: use the merge strategy to update the configuration properties
        if (Type == PlatformType.Docker && descriptor is DockerPlatformDescriptor dockerPlatform)
        {
            PlatformDescriptor = dockerPlatform;
        }
        else if (Type == PlatformType.DockerSwarm && descriptor is DockerSwarmPlatformDescriptor dockerSwarmPlatform)
        {
            PlatformDescriptor = dockerSwarmPlatform;
        }
        else if (Type == PlatformType.Kubernetes && descriptor is KubernetesPlatformDescriptor kubernetesPlatform)
        {
            PlatformDescriptor = kubernetesPlatform;
        }
        else
        {
            throw new ArgumentException("Unsupported platform configuration type.", nameof(descriptor));
        }
        PlatformDescriptor = descriptor;
    }
}
