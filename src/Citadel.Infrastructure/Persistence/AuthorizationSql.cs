namespace Infrastructure.Persistence;

internal static class AuthorizationSql
{
    internal const string ActorScopeCte = """
        ActorScope AS (
            SELECT Users.ActorId
            FROM Users
            JOIN Actors userActor ON userActor.Id = Users.ActorId
            WHERE Users.Id = @UserId
              AND userActor.IsEnabled

            UNION

            SELECT t.ActorId
            FROM Teams t
            JOIN UsersTeams ut ON ut.TeamId = t.Id
            JOIN Actors teamActor ON teamActor.Id = t.ActorId
            WHERE ut.UserId = @UserId
              AND teamActor.IsEnabled
        )
        """;

    internal const string GlobalAccessCte = """
        GlobalAccess AS (
            SELECT 1 AS HasAccess
            FROM ActorRoles ar
            JOIN Permissions p ON p.RoleId = ar.RoleId
            JOIN ActorScope actorScope ON actorScope.ActorId = ar.ActorId
            WHERE p.ResourceType = @ResourceType
              AND p.ResourceAction = @Action
            LIMIT 1
        )
        """;

    internal const string PermissionGlobalAccessCte = """
        GlobalAccess AS (
            SELECT 1 AS HasAccess
            FROM ActorRoles ar
            JOIN Permissions p ON p.RoleId = ar.RoleId
            JOIN ActorScope actorScope ON actorScope.ActorId = ar.ActorId
            WHERE p.ResourceType = @PermissionResourceType
              AND p.ResourceAction = @Action
            LIMIT 1
        )
        """;

    internal const string ResourcePredicatePrefix = """
        (
            EXISTS (SELECT 1 FROM GlobalAccess)
            OR EXISTS (
                SELECT 1
                FROM ResourceAccesses ra
                JOIN ActorScope actorScope ON actorScope.ActorId = ra.ActorId
                WHERE ra.ResourceType = @ResourceType
                  AND ra.Action = @Action
                  AND ra.ResourceId = 
        """;

    internal const string PermissionResourcePredicatePrefix = """
        (
            EXISTS (SELECT 1 FROM GlobalAccess)
            OR EXISTS (
                SELECT 1
                FROM ResourceAccesses ra
                JOIN ActorScope actorScope ON actorScope.ActorId = ra.ActorId
                WHERE ra.ResourceType = @PermissionResourceType
                  AND ra.Action = @Action
                  AND ra.ResourceId = 
        """;

    internal const string ResourcePredicateSuffix = """
            )
        )
        """;
}
