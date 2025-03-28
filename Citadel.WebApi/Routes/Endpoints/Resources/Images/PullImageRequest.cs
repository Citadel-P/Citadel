using Application.Features.Images.Commands;

namespace WebApi.Routes.Endpoints.Resources.Images;

public sealed record PullImageRequest(Guid PlatformId, string RegistryName, string RepositoryName, string ImageTag)
{
    internal PullImage ToCommand() => new (PlatformId, RegistryName, RepositoryName, ImageTag);
}