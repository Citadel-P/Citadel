using Hosting.Common;

namespace Domain.Contracts.Resources.Identity;

public sealed record ResourceAccessDetails(
    Guid Id,
    Guid ActorId,
    Guid ResourceId,
    ResourceType ResourceType,
    string? ResourceName,
    PermissionLevel PermissionLevel,
    IEnumerable<SpecificPermission>? SpecificPermissions);
