using Domain.Contracts.Resources.Networks;

namespace Domain.Contracts.Resources.Containers;

public sealed record CreateContainerCommand(
    string PlatformAddress,
    string ImageId,
    string? Name,
    string? WorkingDir,
    string? User,
    long? MemoryLimit,
    long? CpuQuota,
    long? MemoryReservation,
    bool? AutoRemove,
    ContainerRestartPolicy? RestartPolicy,
    Dictionary<string, string>? Labels,
    List<string>? EnvVars,
    List<string>? Ports,
    List<string>? Volumes,
    Dictionary<string, EndpointSettings>? Networks,
    List<string>? EntryPoint,
    List<string>? Command
    );
