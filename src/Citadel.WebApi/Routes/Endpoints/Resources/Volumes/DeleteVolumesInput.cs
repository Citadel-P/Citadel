using Application.Features.Volumes.Commands;

namespace WebApi.Routes.Endpoints.Resources.Volumes;

public sealed record DeleteVolumesInput (Guid PlatformId, string[] Names, bool? Force)
{
    internal DeleteVolume ToCommand() => new(PlatformId, Names, Force);
}
