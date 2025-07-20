namespace Domain.Contracts.Resources.Containers;

public sealed record PlatformContainersInfo(
    string Address,
    PlatformConnectorType ConnectorType,
    IReadOnlyCollection<string> ContainerIds
);