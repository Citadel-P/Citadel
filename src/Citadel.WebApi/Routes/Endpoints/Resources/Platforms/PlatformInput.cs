using Application.Features.Platforms.Commands;
using Domain;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record PlatformInput(
    string Name,
    string? Address,
    string? Description = null,
    PlatformType Type = PlatformType.Docker,
    PlatformConnectorType ConnectorType = PlatformConnectorType.Local)
{
    internal CreatePlatform ToCommand() => new(Name, Address, Description, Type, ConnectorType);
}

public sealed record CreatePlatformInput(
    string Name,
    string? Address,
    string? Description = null,
    PlatformType Type = PlatformType.Docker,
    PlatformConnectorType ConnectorType = PlatformConnectorType.Local,
    IReadOnlyCollection<Guid>? TagIds = null)
{
    internal CreatePlatform ToCommand() => new(Name, Address, Description, Type, ConnectorType, TagIds);
}
