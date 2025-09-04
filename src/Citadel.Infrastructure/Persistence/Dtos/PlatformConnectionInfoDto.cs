namespace Infrastructure.Persistence.Dtos;

internal sealed record PlatformConnectionInfoDto(
    Guid Id,
    string Name,
    string Address,
    string ConnectorType); // PlatformConnectorType 