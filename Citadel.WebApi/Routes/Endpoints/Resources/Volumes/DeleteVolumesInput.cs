using Application.Features.Volumes.Commands;

namespace WebApi.Routes.Endpoints.Resources.Volumes;

public sealed record DeleteVolumesInput (Guid PlatformId, string[] Names, bool? Force)
{
    internal DeleteVolumesCommand ToCommand() => new(PlatformId, Names, Force);
}
