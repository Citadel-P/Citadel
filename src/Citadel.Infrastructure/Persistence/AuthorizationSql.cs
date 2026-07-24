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

    internal const string AuthorizedResourcesCte = """
        AuthorizedResources AS (
            SELECT DISTINCT permissions.ResourceType, NULL::uuid AS ResourceId
            FROM ActorRoles actorRoles
            JOIN Permissions permissions ON permissions.RoleId = actorRoles.RoleId
            JOIN ActorScope actorScope ON actorScope.ActorId = actorRoles.ActorId
            WHERE (permissions.PermissionLevel & @GrantedPermissionMask) <> 0

            UNION

            SELECT DISTINCT resourceAccesses.ResourceType, resourceAccesses.ResourceId
            FROM ResourceAccesses resourceAccesses
            JOIN ActorScope actorScope ON actorScope.ActorId = resourceAccesses.ActorId
            WHERE (resourceAccesses.PermissionLevel & @GrantedPermissionMask) <> 0
        )
        """;

    internal const string GlobalAccessCte = """
        GlobalAccess AS (
            SELECT 1 AS HasAccess
            FROM ActorRoles ar
            JOIN Permissions p ON p.RoleId = ar.RoleId
            JOIN ActorScope actorScope ON actorScope.ActorId = ar.ActorId
            WHERE p.ResourceType = @ResourceType
              AND (p.PermissionLevel & @GrantedPermissionMask) <> 0
              AND (@SpecificPermission = 0 OR (p.SpecificPermissions & @SpecificPermission) = @SpecificPermission)
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
              AND (p.PermissionLevel & @GrantedPermissionMask) <> 0
              AND (@SpecificPermission = 0 OR (p.SpecificPermissions & @SpecificPermission) = @SpecificPermission)
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
                  AND (ra.PermissionLevel & @GrantedPermissionMask) <> 0
                  AND (@SpecificPermission = 0 OR (ra.SpecificPermissions & @SpecificPermission) = @SpecificPermission)
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
                  AND (ra.PermissionLevel & @GrantedPermissionMask) <> 0
                  AND (@SpecificPermission = 0 OR (ra.SpecificPermissions & @SpecificPermission) = @SpecificPermission)
                  AND ra.ResourceId = 
        """;

    internal const string ResourcePredicateSuffix = """
            )
        )
        """;
}
