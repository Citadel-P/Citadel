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
    long? MemorySwap,
    long? PidsLimit,
    bool? AutoRemove,
    bool? Privileged,
    bool? ReadonlyRootfs,
    ContainerRestartPolicy? RestartPolicy,
    Dictionary<string, string>? Labels,
    List<string>? EnvVars,
    List<string>? Ports,
    List<string>? Volumes,
    List<HostMount>? Mounts,
    List<string>? CapAdd,
    List<string>? CapDrop,
    List<string>? SecurityOpt,
    string? NetworkMode,
    Dictionary<string, EndpointSettings>? Networks,
    List<string>? EntryPoint,
    List<string>? Command
    );
