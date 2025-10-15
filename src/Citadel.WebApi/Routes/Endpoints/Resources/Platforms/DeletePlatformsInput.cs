using Application.Features.Platforms.Commands;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record DeletePlatformsInput(IEnumerable<Guid> Ids)
{
    internal DeletePlatforms ToCommand() => new(Ids);
}