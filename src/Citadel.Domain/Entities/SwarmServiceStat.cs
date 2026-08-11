namespace Domain.Entities;

public readonly record struct SwarmServiceStat(
    Guid PlatformId,
    string DockerServiceId,
    Guid? SwarmServiceId,
    Guid? StackId,
    string ServiceName,
    string TaskKey,
    string DockerTaskId,
    double? MemoryActive,
    double? MemoryCache,
    double? CpuUsage,
    double? MemoryLimit,
    double? RxBytes,
    double? TxBytes,
    long Created)
{
    public Guid Id { get; } = Guid.CreateVersion7();
}

public sealed record SwarmServiceStatAttribution(
    Guid ContainerId,
    Guid PlatformId,
    string DockerServiceId,
    Guid? SwarmServiceId,
    Guid? StackId,
    string ServiceName,
    string TaskKey,
    string DockerTaskId);

public sealed record SwarmServiceStatIdentity(
    Guid PlatformId,
    string DockerServiceId,
    Guid? SwarmServiceId,
    Guid? StackId,
    string ServiceName);

public readonly record struct SwarmServiceStatSample(
    double? MemoryActive,
    double? MemoryCache,
    double? CpuUsage,
    double? MemoryLimit,
    double? RxBytes,
    double? TxBytes,
    long Created);
