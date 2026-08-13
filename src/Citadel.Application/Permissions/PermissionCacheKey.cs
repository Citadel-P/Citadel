using Hosting.Common;

namespace Application.Permissions;

public readonly record struct PermissionCacheKey(
    Guid ActorId,
    ResourceType ResourceType,
    Guid? ResourceId);
