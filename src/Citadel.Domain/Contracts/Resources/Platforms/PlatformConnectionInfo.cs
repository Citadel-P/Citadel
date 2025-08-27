namespace Domain.Contracts.Resources.Platforms;

public sealed record PlatformConnectionInfo(
    Guid Id,
    string Name,
    string Address,
    PlatformConnectorType ConnectorType);
