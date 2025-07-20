using Application.Features.Volumes.Commands;

namespace WebApi.Routes.Endpoints.Resources.Volumes;

public sealed record CreateVolumeInput(
    Guid PlatformId,
    string Name,
    string Driver,
    Dictionary<string, string>? Labels = null,
    Dictionary<string, string>? Options = null)
{
    internal CreateVolume ToCommand()
        => new (
            PlatformId: PlatformId,
            Name: Name,
            Driver: Driver,
            Labels: Labels,
            Options: Options);
}
