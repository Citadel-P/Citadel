using Contracts.Broker.Models;

namespace Infrastructure.Entities;

public class SystemInfo
{
    public Guid Id { get; }

    public Guid PlatformId { get; }

    /// <summary>
    /// The Docker daemon ID, it identifies the remote agent instance
    /// </summary>
    public string DaemonId { get; private set; }

    /// <summary>
    /// Get or Set the networks count (only networks that are in use, dangling one's should not be included in the count)
    /// </summary>
    public short NetworksCount { get; private set; }

    /// <summary>
    /// Get or Set the volumes count (only networks that are in use, dangling one's should not be included in the count)
    /// </summary>
    public short VolumesCount { get; private set; }

    public long Containers { get; private set; }

    public long ContainersRunning { get; private set; }

    public long ContainersPaused { get; private set; }

    public long ContainersStopped { get; private set; }

    public long Images { get; private set; }

    public string Driver { get; private set; }

    public string OperatingSystem { get; private set; }

    public string OSVersion { get; private set; }

    public string OSType { get; private set; }

    public string Architecture { get; private set; }

    public long NCPU { get; private set; }

    public long MemTotal { get; private set; }

    public string ServerVersion { get; private set; }

    public string AgentVersion { get; private set; }

    public SwarmInfo SwarmInfo { get; private set; }

    public bool EqualsTo(SystemInfoMessage message)
    {
        return Images == message.Images &&
               Containers == message.Containers &&
               ContainersPaused == message.ContainersPaused &&
               ContainersRunning == message.ContainersRunning &&
               ContainersStopped == message.ContainersStopped &&
               Driver == message.Driver &&
               NCPU == message.NCPU &&
               OSType == message.OSType &&
               ServerVersion == message.ServerVersion &&
               MemTotal == message.MemTotal &&
               AgentVersion == message.AgentVersion &&
               NetworksCount == message.NetworksCount &&
               VolumesCount == message.VolumesCount &&
               (SwarmInfo == null || SwarmInfo.EqualsTo(message.Swarm));
    }

    public void UpdateWith(SystemInfoMessage message)
    {
        Images = message.Images;
        Containers = message.Containers;
        ContainersPaused = message.ContainersPaused;
        ContainersRunning = message.ContainersRunning;
        ContainersStopped = message.ContainersStopped;
        Driver = message.Driver;
        NCPU = message.NCPU;
        OSType = message.OSType;
        ServerVersion = message.ServerVersion;
        MemTotal = message.MemTotal;
        AgentVersion = message.AgentVersion;
        NetworksCount = message.NetworksCount;
        VolumesCount = message.VolumesCount;
        SwarmInfo?.UpdateWith(message.Swarm);
    }

    public void SetDaemonId(string to)
        => DaemonId = to;
}