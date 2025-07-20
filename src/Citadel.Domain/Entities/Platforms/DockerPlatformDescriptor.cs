using System.Text.Json.Serialization;

namespace Domain.Entities.Platforms;

[method: JsonConstructor]
public record DockerPlatformDescriptor(
        string DaemonId,
        long ContainerCount,
        long ContainersRunning,
        long ContainersPaused,
        long ContainersStopped,
        string? Driver = null,
        string? OperatingSystem = null,
        string? OsVersion = null,
        string? OsType = null,
        string? Architecture = null) : PlatformDescriptor
{
    public DockerPlatformDescriptor Create(
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
        =>
        this with
        {
            DaemonId = daemonId ?? DaemonId,
            ContainerCount = containerCount ?? ContainerCount,
            ContainersRunning = containersRunning ?? ContainersRunning,
            ContainersPaused = containersPaused ?? ContainersPaused,
            ContainersStopped = containersStopped ?? ContainersStopped,
            Driver = driver ?? Driver,
            OperatingSystem = operatingSystem ?? OperatingSystem,
            OsVersion = osVersion ?? OsVersion,
            OsType = osType ?? OsType,
            Architecture = architecture ?? Architecture
        };
}
