using Application.Features.Images.Commands;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record PullImageRequest(Guid PlatformId, string RegistryName, string PackageName, string ImageTag)
{
    internal PullImage ToCommand() => new (PlatformId, RegistryName, PackageName, ImageTag);
}