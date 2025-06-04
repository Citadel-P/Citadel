using Application.Features.Platforms.Commands;
using Infrastructure;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record CreatePlatformInput(
    string Name,
    string Address,
    PlatformType Type)
{
    internal CreatePlatform ToCommand() => new(Name, Address, Type);
}
