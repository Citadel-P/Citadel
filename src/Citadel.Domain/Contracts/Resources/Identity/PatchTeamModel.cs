using Hosting.Common;

namespace Domain.Contracts.Resources.Identity;

public sealed record TeamResourceAccessModel(
    ResourceType ResourceType,
    Guid ResourceId,
    PermissionLevel PermissionLevel,
    IEnumerable<SpecificPermission>? SpecificPermissions);

public sealed record PatchTeamModel(
    bool? IsEnabled,
    IEnumerable<Guid>? UserIds,
    IEnumerable<Guid>? RoleIds,
    IEnumerable<TeamResourceAccessModel>? ResourceAccesses);
