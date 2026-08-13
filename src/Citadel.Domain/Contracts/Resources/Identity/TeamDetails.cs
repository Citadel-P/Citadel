namespace Domain.Contracts.Resources.Identity;

public sealed record TeamDetails(
    Guid Id,
    string Name,
    Guid ActorId,
    bool IsEnabled,
    int? TotalMembers = 0,
    IEnumerable<ResourceInfo>? Users = null,
    IEnumerable<ResourceInfo>? Roles = null,
    IEnumerable<ResourceAccessView>? ResourceAccesses = null,
    IEnumerable<ActorTeamMemberInfo>? Members = null);

public sealed record ActorTeamMemberInfo(
    Guid ActorId,
    Guid ResourceId,
    string Name,
    ActorType PrincipalType);
