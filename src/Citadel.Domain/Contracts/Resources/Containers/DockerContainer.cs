using Domain.Entities;

namespace Domain.Contracts.Resources.Containers;

public sealed record DockerContainer(
    string Name,
    string Image,
    string Id,
    string ImageId,
    ContainerStateStatus State,
    long? Created = null,
    string? Stack = null,
    DockerContainerStat? ContainerStat = null,
    ResourceControlState? ControlState = null,
    IDictionary<string, IReadOnlyList<HostPortBinding>>? Ports = null,
    Guid? DeploymentId = null,
    Guid? StackId = null,
    bool IsSystem = false,
    ContainerSystemRole? SystemRole = null,
    bool HasCitadelOwnershipLabels = false,
    bool IsSwarmTask = false
    );

public record struct DockerContainerStat(
    double? MemoryActive,
    double? MemoryCache, 
    double? CpuUsage, 
    double? MemoryLimit, 
    double? RxBytes, 
    double? TxBytes,
    long? Created = null);
