using Domain.Entities;

namespace Domain.Contracts.Resources.Containers;

public sealed record ContainerInfo (
    string Name,
    string ContainerId,
    Guid PlatformId,
    string StartedAt,
    string FinishedAt,
    string PlatformName,
    IList<string> Volumes,
    IDictionary<string, IReadOnlyList<HostPortBinding>> Ports,
    IDictionary<string, string> Networks,
    ContainerStateStatus State,
    Image? Image = null);
