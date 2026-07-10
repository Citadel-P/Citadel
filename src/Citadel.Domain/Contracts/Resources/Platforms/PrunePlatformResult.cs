using Domain;

namespace Domain.Contracts.Resources.Platforms;

public sealed record PrunePlatformResult(
    PruneResource Resource,
    long SpaceReclaimed,
    IReadOnlyList<string> VolumesDeleted,
    IReadOnlyList<string> NetworksDeleted,
    IReadOnlyList<string> ImagesDeleted,
    IReadOnlyList<string> BuildCacheDeleted);
