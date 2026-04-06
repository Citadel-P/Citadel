using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using System.Data;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class UserRepository(IDbConnection db, Func<IDbTransaction> tx) : IUserRepository
{
    public async Task<bool> HasPermissionAsync(Guid userId, ResourceType resourceType, ResourceAction action, Guid? resourceId, CancellationToken ct)
    {
        const string sql = """
            WITH ActorScope AS (
                -- User actor
                SELECT ActorId 
                FROM Users 
                WHERE Id = @UserId

                UNION

                -- Team actors
                SELECT t.ActorId
                FROM Teams t
                JOIN UsersTeams ut ON ut.TeamId = t.Id
                WHERE ut.UserId = @UserId
            )
            SELECT
                -- 1. Global role-based permissions
                EXISTS (
                    SELECT 1
                    FROM ActorRoles ar
                    JOIN Permissions p ON p.RoleId = ar.RoleId
                    WHERE ar.ActorId IN (SELECT ActorId FROM ActorScope)
                      AND p.ResourceType = @ResourceType
                      AND p.ResourceAction = @Action
                )
                OR
                -- 2. Resource-specific access (only if resourceId provided)
                (
                    @ResourceId IS NOT NULL AND EXISTS (
                        SELECT 1
                        FROM ResourceAccesses ra
                        WHERE ra.ActorId IN (SELECT ActorId FROM ActorScope)
                          AND ra.ResourceType = @ResourceType
                          AND ra.ResourceId = @ResourceId
                          AND ra.Action = @Action
                    )
                );
            """;

        return await db.ExecuteScalarAsync<bool>(sql, new
        {
            UserId = userId.Format(),
            ResourceId = resourceId?.Format(), // important: nullable
            ResourceType = EnumFormatter<ResourceType>.GetValue(resourceType),
            Action = EnumFormatter<ResourceAction>.GetValue(action)
        });
    }

    public async Task<UserAuthInfo?> GetUserAuthInfoByEmailAsync(string email, CancellationToken cancellationToken)
    {
        const string sql =
            """
            WITH TargetUser AS (
                SELECT *
                FROM Users
                WHERE Email = @Email
                LIMIT 1
            ),
            ActorScope AS (
                SELECT TargetUser.ActorId
                FROM TargetUser

                UNION

                SELECT Teams.ActorId
                FROM TargetUser
                JOIN UsersTeams ON TargetUser.Id = UsersTeams.UserId
                JOIN Teams ON Teams.Id = UsersTeams.TeamId
            )
            SELECT 
                TargetUser.Id, 
                TargetUser.Name, 
                TargetUser.Email,
                TargetUser.ActorId,
                TargetUser.Password,
                Roles.Name as RoleName, 
                CASE
                    WHEN Permissions.ResourceType IS NOT NULL AND Permissions.ResourceAction IS NOT NULL
                    THEN Permissions.ResourceType || '_' || Permissions.ResourceAction
                    ELSE NULL
                END AS PermissionName
            FROM TargetUser
            LEFT JOIN ActorScope ON 1 = 1
            LEFT JOIN ActorRoles ON ActorRoles.ActorId = ActorScope.ActorId
            LEFT JOIN Roles ON Roles.Id = ActorRoles.RoleId
            LEFT JOIN Permissions ON Roles.Id = Permissions.RoleId
            """;

            var result = await db.QueryAsync<UserAuthInfoDto>(sql,
            new { Email = email },
            transaction: tx());

        return result
        .GroupBy(r => new { r.Id, r.Name, r.Email, r.ActorId, r.Password })
        .Select(g => new UserAuthInfo(
            g.Key.Id,
            g.Key.ActorId,
            g.Key.Name,
            g.Key.Email,
            g.Key.Password,
            [.. g.Where(r => !string.IsNullOrWhiteSpace(r.RoleName))
                    .Select(r => r.RoleName!)
                    .Distinct()],
            [.. g.Where(r => !string.IsNullOrWhiteSpace(r.PermissionName))
                    .Select(r => Enum.TryParse<AppPermission>(r.PermissionName, out var permission)
                        ? permission
                        : (AppPermission?)null)
                    .Where(permission => permission.HasValue)
                    .Select(permission => permission!.Value)
                    .Distinct()]
        )).FirstOrDefault();
    }
}
