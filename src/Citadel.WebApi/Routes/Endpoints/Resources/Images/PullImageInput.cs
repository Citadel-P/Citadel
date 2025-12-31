using Application.Features.Images.Commands;

namespace WebApi.Routes.Endpoints.Resources.Images;

public sealed record PullImageInput(Guid PlatformId, Guid RegistryId, string ImageTag)
{
    internal PullImage ToCommand() => new (PlatformId, RegistryId, ImageTag);
}