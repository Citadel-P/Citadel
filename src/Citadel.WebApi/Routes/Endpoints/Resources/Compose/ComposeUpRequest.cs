using Application.Features.Compose.Commands;

namespace WebApi.Routes.Endpoints.Resources.Compose;

public sealed record ComposeUpRequest(Guid PlatformId, string RegistryName, string RepositoryName, string ComposeFileAsStr)
{
    internal ComposeUp ToCommand() => new (PlatformId, RegistryName, RepositoryName, ComposeFileAsStr);
}
