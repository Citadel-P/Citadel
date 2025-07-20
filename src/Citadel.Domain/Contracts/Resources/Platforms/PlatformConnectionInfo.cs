namespace Domain.Contracts.Resources.Platforms;

public sealed record PlatformConnectionInfo(
    Guid Id,
    string Address,
    PlatformConnectorType ConnectorType);
