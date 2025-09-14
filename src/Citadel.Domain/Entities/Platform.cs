using System.Text.Json.Serialization;
using Domain.Entities.Platforms;

namespace Domain.Entities;

[method: JsonConstructor]
public class Platform(
    string name,
    string address,
    int networkCount,
    int volumeCount,
    long imageCount,
    long cpuCount,
    long memTotal,
    string? serverVersion,
    string? agentVersion,
    PlatformStatus status,
    PlatformConnectorType connectorType,
    PlatformDescriptor platformDescriptor)
{
    private readonly List<PlatformStat> stats = [];

    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; internal set; } = name;
    public string Address { get; internal set; } = address;
    public PlatformStatus Status { get; private set; } = status;
    public PlatformConnectorType ConnectorType { get; private set; } = connectorType;
    public int NetworkCount { get; private set; } = networkCount;
    public int VolumeCount { get; private set; } = volumeCount;
    public long ImageCount { get; private set; } = imageCount;
    public long CpuCount { get; private set; } = cpuCount;
    public long MemTotal { get; private set; } = memTotal;
    public string? AgentVersion { get; private set; } = agentVersion;
    public string? ServerVersion { get; private set; } = serverVersion;
    public PlatformDescriptor PlatformDescriptor { get; private set; } = platformDescriptor;
    public IReadOnlyCollection<PlatformStat>? Stats => stats;

    public static Platform FromPersistence(
        Guid id,
        string name,
        string address,
        int networkCount,
        int volumeCount,
        long imageCount,
        long cpuCount,
        long memTotal,
        PlatformStatus status,
        PlatformConnectorType connectorType,
        PlatformDescriptor platformDescriptor,
        string? serverVersion = null,
        string? agentVersion = null,
        IReadOnlyCollection<PlatformStat>? stats = null
        )
    {
        var platform = new Platform(
            name: name,
            address: address,
            networkCount: networkCount,
            volumeCount: volumeCount,
            imageCount: imageCount,
            cpuCount: cpuCount,
            memTotal: memTotal,
            serverVersion: serverVersion,
            agentVersion: agentVersion,
            status: status,
            connectorType: connectorType,
            platformDescriptor: platformDescriptor)
        {
            Id = id,
        };
        if (stats is not null)
        {
            foreach (var stat in stats)
                platform.AppendStat(stat);
        }

        return platform;
    }

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
        stats.Add(stat);
        return this;
    }   
}
