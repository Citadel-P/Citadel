using Hosting.Common;

namespace Domain.Contracts.Resources.Identity;

public sealed record TeamResourceAccessModel(ResourceType ResourceType, Guid ResourceId, ResourceAction Action);

public sealed record PatchTeamModel(
    bool? IsEnabled,
    IEnumerable<Guid>? UserIds,
    IEnumerable<Guid>? RoleIds,
    IEnumerable<TeamResourceAccessModel>? ResourceAccesses);
