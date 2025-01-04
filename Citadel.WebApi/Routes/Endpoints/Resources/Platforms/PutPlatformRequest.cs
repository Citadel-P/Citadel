using Application.Features.Platforms.Commands;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record PutPlatformRequest(
    Guid? Id,
    string Name,
    string Address
    )
{
    internal UpsertPlatform ToCommand() => new(Id, Name, Address);
};