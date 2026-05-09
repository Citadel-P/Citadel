using Hosting.Common;

namespace Domain.Contracts.Resources.Identity;

public sealed record ResourceAccessView(
    ResourceType ResourceType,
    Guid ResourceId,
    string? ResourceName,
    PermissionLevel PermissionLevel,
    IEnumerable<SpecificPermission>? SpecificPermissions);

public sealed record PatchUserModel(
    string? Email,
    string? Password,
    bool? IsEnabled,
    IEnumerable<Guid>? TeamIds,
    IEnumerable<Guid>? RoleIds,
    IEnumerable<ResourceAccessView>? ResourceAccesses);
