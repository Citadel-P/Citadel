using Application.Features.Platforms.Commands;
using Domain;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record PrunePlatformInput(PruneResource Resource)
{
    internal PrunePlatform ToCommand(Guid platformId) => new(platformId, Resource);
}
