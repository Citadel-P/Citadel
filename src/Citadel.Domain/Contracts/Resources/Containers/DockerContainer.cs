using Domain.Entities;

namespace Domain.Contracts.Resources.Containers;

public sealed record DockerContainer
    (
    string Name,
    string Image,
    string ContainerId,
    ContainerStateStatus State,
    long? Created = null,
    string? Stack = null,
    string? Command = null,
    DockerContainerStat? ContainerStat = null,
    IReadOnlyList<ContainerPort>? Ports = null
    );

public sealed record DockerContainerStat 
    (
    double? MemoryUsage,
    double? CpuUsage,
    double? MemoryLimit,
    double? RxBytes,
    double? TxBytes
    );