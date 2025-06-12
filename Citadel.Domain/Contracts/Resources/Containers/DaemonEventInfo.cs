using Domain.Entities;

namespace Domain.Contracts.Resources.Containers;

public sealed record DaemonEventInfo(
    string Id,
    string Action,
    string ContainerId,
    Container? Container,
    ContainerEventType Type);
