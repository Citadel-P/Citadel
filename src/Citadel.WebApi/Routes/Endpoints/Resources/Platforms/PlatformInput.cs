using Application.Features.Platforms.Commands;
using Domain;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record PlatformInput(
    string Name,
    string? Address,
    string? Description = null,
    PlatformType Type = PlatformType.Docker,
    PlatformConnectorType ConnectorType = PlatformConnectorType.Local,
    bool PruneHistoricalSwarmTaskContainers = true)
{
    internal CreatePlatform ToCommand() => new(Name, Address, Description, Type, ConnectorType, PruneHistoricalSwarmTaskContainers);
}

public sealed record CreatePlatformInput(
    string Name,
    string? Address,
    string? Description = null,
    PlatformType Type = PlatformType.Docker,
    PlatformConnectorType ConnectorType = PlatformConnectorType.Local,
    bool PruneHistoricalSwarmTaskContainers = true,
    IReadOnlyCollection<Guid>? TagIds = null)
{
    internal CreatePlatform ToCommand() => new(Name, Address, Description, Type, ConnectorType, PruneHistoricalSwarmTaskContainers, TagIds);
}
