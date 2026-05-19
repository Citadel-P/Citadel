using Hosting.Common;

namespace Application.Permissions;

public readonly record struct PermissionCacheKey(
    Guid UserId,
    ResourceType ResourceType,
    Guid? ResourceId);
