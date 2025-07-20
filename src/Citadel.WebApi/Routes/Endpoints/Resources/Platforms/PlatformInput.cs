using Application.Features.Platforms.Commands;
using Domain;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record PlatformInput(
    string Name,
    string? Address,
    PlatformType Type = PlatformType.Docker,
    PlatformConnectorType ConnectorType = PlatformConnectorType.Local)
{
    internal CreatePlatform ToCommand() => new(Name, Address, Type, ConnectorType);
}
