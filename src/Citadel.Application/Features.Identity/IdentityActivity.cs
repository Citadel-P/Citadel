using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Activities;
using Domain.Entities.Identity;

namespace Application.Features.Identity;

internal static class IdentityActivity
{
    internal static ActivityEvent Create(
        Guid resourceId,
        string resourceName,
        Guid actorId,
        ActivityEventType eventType,
        ActivityEventInfo info)
        => new(
            platformId: null,
            resourceId: resourceId,
            actorId: actorId,
            resourceName: resourceName,
            eventType: eventType,
            status: ActivityStatus.Success,
            info: info);

    internal static async Task<UserActivitySnapshot?> CaptureUserAsync(
        IUnitOfWork unitOfWork,
        Guid userId,
        CancellationToken cancellationToken)
    {
        var user = await unitOfWork.Users.GetDetailsAsync(userId, cancellationToken);
        if (user is null)
            return null;

        var resourceAccesses = await unitOfWork.ResourceAccesses.GetAllByActorIdAsync(user.ActorId, cancellationToken);
        return new UserActivitySnapshot(
            user.Email,
            user.IsEnabled,
            SortIds(user.Teams?.Select(static item => item.Id)),
            SortIds(user.Roles?.Select(static item => item.Id)),
            SnapshotAccesses(resourceAccesses));
    }

    internal static UserActivitySnapshot Snapshot(
        string email,
        bool isEnabled,
        IEnumerable<Guid> teamIds,
        IEnumerable<Guid> roleIds,
        IEnumerable<ResourceAccessView> resourceAccesses)
        => new(
            email,
            isEnabled,
            SortIds(teamIds),
            SortIds(roleIds),
            SnapshotAccesses(resourceAccesses));

    internal static async Task<TeamActivitySnapshot?> CaptureTeamAsync(
        IUnitOfWork unitOfWork,
        Guid teamId,
        CancellationToken cancellationToken)
    {
        var team = await unitOfWork.Teams.GetDetailsAsync(teamId, cancellationToken);
        if (team is null)
            return null;

        var resourceAccesses = await unitOfWork.ResourceAccesses.GetAllByActorIdAsync(team.ActorId, cancellationToken);
        return new TeamActivitySnapshot(
            team.IsEnabled,
            SortIds(team.Members?.Select(static member => member.ActorId)),
            SortIds(team.Roles?.Select(static item => item.Id)),
            SnapshotAccesses(resourceAccesses));
    }

    internal static TeamActivitySnapshot Snapshot(
        bool isEnabled,
        IEnumerable<Guid> memberActorIds,
        IEnumerable<Guid> roleIds,
        IEnumerable<TeamResourceAccessModel> resourceAccesses)
        => new(
            isEnabled,
            SortIds(memberActorIds),
            SortIds(roleIds),
            resourceAccesses
                .OrderBy(static access => access.ResourceType)
                .ThenBy(static access => access.ResourceId)
                .ThenBy(static access => access.PermissionLevel)
                .Select(static access => new IdentityResourceAccessSnapshot(
                    access.ResourceType,
                    access.ResourceId,
                    access.PermissionLevel,
                    Permission.ToSpecificPermissionsMask(access.SpecificPermissions)))
                .ToArray());

    internal static RoleActivitySnapshot Snapshot(Role role)
        => new(
            role.RoleType,
            role.Permissions
                .OrderBy(static permission => permission.ResourceType)
                .ThenBy(static permission => permission.PermissionLevel)
                .Select(static permission => new RolePermissionActivitySnapshot(
                    permission.ResourceType,
                    permission.PermissionLevel,
                    Permission.ToSpecificPermissionsMask(permission.SpecificPermissions)))
                .ToArray());

    internal static bool Same(UserActivitySnapshot left, UserActivitySnapshot right)
        => string.Equals(left.Email, right.Email, StringComparison.Ordinal)
           && left.IsEnabled == right.IsEnabled
           && left.TeamIds.SequenceEqual(right.TeamIds)
           && left.RoleIds.SequenceEqual(right.RoleIds)
           && left.ResourceAccesses.SequenceEqual(right.ResourceAccesses);

    internal static bool Same(TeamActivitySnapshot left, TeamActivitySnapshot right)
        => left.IsEnabled == right.IsEnabled
           && left.MemberActorIds.SequenceEqual(right.MemberActorIds)
           && left.RoleIds.SequenceEqual(right.RoleIds)
           && left.ResourceAccesses.SequenceEqual(right.ResourceAccesses);

    internal static bool Same(RoleActivitySnapshot left, RoleActivitySnapshot right)
        => left.RoleType == right.RoleType
           && left.Permissions.SequenceEqual(right.Permissions);

    internal static UserActivitySnapshot WithRole(UserActivitySnapshot snapshot, Guid roleId, bool add)
        => snapshot with { RoleIds = ChangeIds(snapshot.RoleIds, roleId, add) };

    internal static TeamActivitySnapshot WithRole(TeamActivitySnapshot snapshot, Guid roleId, bool add)
        => snapshot with { RoleIds = ChangeIds(snapshot.RoleIds, roleId, add) };

    internal static TeamActivitySnapshot WithMember(TeamActivitySnapshot snapshot, Guid actorId, bool add)
        => snapshot with { MemberActorIds = ChangeIds(snapshot.MemberActorIds, actorId, add) };

    internal static UserActivitySnapshot WithResourceAccess(
        UserActivitySnapshot snapshot,
        IdentityResourceAccessSnapshot access,
        bool add)
        => snapshot with { ResourceAccesses = ChangeAccesses(snapshot.ResourceAccesses, access, add) };

    internal static TeamActivitySnapshot WithResourceAccess(
        TeamActivitySnapshot snapshot,
        IdentityResourceAccessSnapshot access,
        bool add)
        => snapshot with { ResourceAccesses = ChangeAccesses(snapshot.ResourceAccesses, access, add) };

    private static Guid[] SortIds(IEnumerable<Guid>? ids)
        => ids?.Distinct().Order().ToArray() ?? [];

    private static Guid[] ChangeIds(IEnumerable<Guid> ids, Guid id, bool add)
        => (add ? ids.Append(id) : ids.Where(existing => existing != id))
            .Distinct()
            .Order()
            .ToArray();

    private static IdentityResourceAccessSnapshot[] ChangeAccesses(
        IEnumerable<IdentityResourceAccessSnapshot> accesses,
        IdentityResourceAccessSnapshot access,
        bool add)
        => (add ? accesses.Append(access) : accesses.Where(existing => existing != access))
            .Distinct()
            .OrderBy(static item => item.ResourceType)
            .ThenBy(static item => item.ResourceId)
            .ThenBy(static item => item.PermissionLevel)
            .ToArray();

    private static IdentityResourceAccessSnapshot[] SnapshotAccesses(
        IEnumerable<ResourceAccessView>? resourceAccesses)
        => resourceAccesses?
            .OrderBy(static access => access.ResourceType)
            .ThenBy(static access => access.ResourceId)
            .ThenBy(static access => access.PermissionLevel)
            .Select(static access => new IdentityResourceAccessSnapshot(
                access.ResourceType,
                access.ResourceId,
                access.PermissionLevel,
                Permission.ToSpecificPermissionsMask(access.SpecificPermissions)))
            .ToArray() ?? [];

    private static IdentityResourceAccessSnapshot[] SnapshotAccesses(
        IEnumerable<ResourceAccessDetails> resourceAccesses)
        => resourceAccesses
            .OrderBy(static access => access.ResourceType)
            .ThenBy(static access => access.ResourceId)
            .ThenBy(static access => access.PermissionLevel)
            .Select(static access => new IdentityResourceAccessSnapshot(
                access.ResourceType,
                access.ResourceId,
                access.PermissionLevel,
                Permission.ToSpecificPermissionsMask(access.SpecificPermissions)))
            .ToArray();
}
