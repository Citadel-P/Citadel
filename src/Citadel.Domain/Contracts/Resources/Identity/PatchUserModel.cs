using Hosting.Common;

namespace Domain.Contracts.Resources.Identity;

public sealed record UserResourceAccessModel(ResourceType ResourceType, Guid ResourceId, ResourceAction Action);

public sealed record PatchUserModel(
    string? Email,
    string? Password,
    bool? IsEnabled,
    IEnumerable<Guid>? TeamIds,
    IEnumerable<Guid>? RoleIds,
    IEnumerable<UserResourceAccessModel>? ResourceAccesses);
