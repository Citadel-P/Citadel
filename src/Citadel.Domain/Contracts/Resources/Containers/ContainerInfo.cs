namespace Domain.Contracts.Resources.Containers;

public sealed record ContainerInfo (
    Guid Id,
    string Name,
    string ContainerId,
    Guid PlatformId,
    string PlatformName,
    ContainerStateStatus State);
