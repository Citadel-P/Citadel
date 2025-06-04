using System.Text.Json.Serialization;

namespace Infrastructure.Entities;

[method: JsonConstructor]
public class DockerPlatformDescriptor(
        string daemonId,
        long containerCount,
        long containersRunning,
        long containersPaused,
        long containersStopped,
        string? driver = null,
        string? operatingSystem = null,
        string? osVersion = null,
        string? osType = null,
        string? architecture = null) : PlatformDescriptor
{
    public string DaemonId { get; private set; } = daemonId;
    public long ContainerCount { get; private set; } = containerCount;
    public long ContainersRunning { get; private set; } = containersRunning;
    public long ContainersPaused { get; private set; } = containersPaused;
    public long ContainersStopped { get; private set; } = containersStopped;
    public string? Driver { get; private set; } = driver;
    public string? OperatingSystem { get; private set; } = operatingSystem;
    public string? OsVersion { get; private set; } = osVersion;
    public string? OsType { get; private set; } = osType;
    public string? Architecture { get; private set; } = architecture;

    public void PartialUpdate(
        string? daemonId = null,
        long? containerCount = null,
        long? containersRunning = null,
        long? containersPaused = null,
        long? containersStopped = null,
        string? driver = null,
        string? operatingSystem = null,
        string? osVersion = null,
        string? osType = null,
        string? architecture = null)
    {
        if (daemonId != null) DaemonId = daemonId;
        if (containerCount != null) ContainerCount = containerCount.Value;
        if (containersRunning != null) ContainersRunning = containersRunning.Value;
        if (containersPaused != null) ContainersPaused = containersPaused.Value;
        if (containersStopped != null) ContainersStopped = containersStopped.Value;
        if (driver != null) Driver = driver;
        if (operatingSystem != null) OperatingSystem = operatingSystem;
        if (osVersion != null) OsVersion = osVersion;
        if (osType != null) OsType = osType;
        if (architecture != null) Architecture = architecture;
    }
}
