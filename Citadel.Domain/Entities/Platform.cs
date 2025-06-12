using System.Text.Json.Serialization;
using Domain.Entities.Platforms;

namespace Domain.Entities;

public class Platform
{
    private readonly List<PlatformStat> stats = [];

    private Platform() { /* For EF Core */ }

    [method: JsonConstructor]
    public Platform(
        string name,
        string address,
        int networkCount,
        int volumeCount,
        long imageCount,
        long cpuCount,
        long memTotal,
        string? serverVersion,
        string? agentVersion,
        PlatformType type,
        PlatformStatus status,
        PlatformDescriptor platformDescriptor)
    {
        Id = Guid.CreateVersion7();
        Name = name;
        Type = type;
        Status = status;
        Address = address;
        MemTotal = memTotal;
        CpuCount = cpuCount;
        ImageCount = imageCount;
        VolumeCount = volumeCount;
        NetworkCount = networkCount;
        ServerVersion = serverVersion;
        AgentVersion = agentVersion;
        PlatformDescriptor = platformDescriptor;
    }

    public Guid Id { get; private set; }
    public string Name { get; internal set; }
    public string Address { get; internal set; }
    public PlatformType Type { get; private set; }
    public PlatformStatus Status { get; private set; }
    public int NetworkCount { get; private set; }
    public int VolumeCount { get; private set; }
    public long ImageCount { get; private set; }
    public long CpuCount { get; private set; }
    public long MemTotal { get; private set; }
    public string? AgentVersion { get; private set; }
    public string? ServerVersion { get; private set; }
    public PlatformDescriptor PlatformDescriptor { get; private set; }
    public IReadOnlyCollection<PlatformStat> Stats => stats;

    public void PartialUpdate(
        string? name = null,
        string? address = null,
        int? networkCount = null,
        int? volumeCount = null,
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
        if (descriptor != null) PlatformDescriptor = descriptor;
    }

    public Platform AppendStat(PlatformStat stat)
    {
        if (stat == null) throw new ArgumentNullException(nameof(stat), "Stat cannot be null.");
        stats.Add(stat);
        return this;
    }   
}
