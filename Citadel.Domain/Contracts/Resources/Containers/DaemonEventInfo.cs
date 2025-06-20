namespace Domain.Contracts.Resources.Containers;

public sealed record DaemonEventInfo(
    string Id,
    string Action,
    string ContainerId,
    DockerContainer? Container,
    ContainerEventType Type);
