using Domain;
using Domain.Contracts.Resources.Platforms;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record PrunePlatformView(
    PruneResource Resource,
    long SpaceReclaimed,
    IReadOnlyList<string> VolumesDeleted,
    IReadOnlyList<string> NetworksDeleted,
    IReadOnlyList<string> ImagesDeleted,
    IReadOnlyList<string> BuildCacheDeleted)
{
    internal static PrunePlatformView Map(PrunePlatformResult result)
        => new(
            result.Resource,
            result.SpaceReclaimed,
            result.VolumesDeleted,
            result.NetworksDeleted,
            result.ImagesDeleted,
            result.BuildCacheDeleted);
}
