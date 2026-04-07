using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using Hosting.Common;
using Hosting.Common.Models;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using System.Text.Json;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class UserRepository(IDbConnection db, Func<IDbTransaction> tx) : IUserRepository
{
    public async Task<User?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Users WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<UserDto>(sql, new { Id = id.Format(), cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<User>> GetAllAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Users ORDER BY Name ASC";
        var result = await db.QueryAsync<UserDto>(sql, transaction: tx());
        return result.ToDomain();
    }

    public async Task<PagedResult<UserDetails>> GetPagedAsync(int page, int pageSize, CancellationToken cancellationToken)
    {
        const string selectSql = """
            SELECT
                u.Id,
                u.Name,
                u.Email,
                u.Password,
                u.ActorId,
                a.IsEnabled,
                u.CreatedAt,
                u.CreatedByActorId
            FROM Users u
            JOIN Actors a ON a.Id = u.ActorId
            ORDER BY u.Name ASC
            LIMIT @PageSize OFFSET @Offset
            """;
        const string countSql = "SELECT COUNT(*) FROM Users";

        var offset = (page - 1) * pageSize;
        var totalCount = await db.QuerySingleAsync<int>(countSql, transaction: tx());
        var rows = await db.QueryAsync<UserWithActorDto>(selectSql, new { PageSize = pageSize, Offset = offset, cancellationToken }, transaction: tx());

        return new PagedResult<UserDetails>(rows.ToDetails(), totalCount, page, pageSize);
    }

    public async Task<PagedResult<UserDetails>> GetAuthorizedPagedAsync(Guid userId, ResourceType resourceType, ResourceAction action, int page, int pageSize, CancellationToken cancellationToken)
    {
        const string selectSql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + " " + """
            SELECT
                u.Id,
                u.Name,
                u.Email,
                u.Password,
                u.ActorId,
                a.IsEnabled,
                u.CreatedAt,
                u.CreatedByActorId
            FROM Users u
            JOIN Actors a ON a.Id = u.ActorId
            WHERE
        """ + AuthorizationSql.ResourcePredicatePrefix + "u.Id" + AuthorizationSql.ResourcePredicateSuffix + " ORDER BY u.Name ASC LIMIT @PageSize OFFSET @Offset;";

        const string countSql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + " SELECT COUNT(*) FROM Users u WHERE "
            + AuthorizationSql.ResourcePredicatePrefix + "u.Id" + AuthorizationSql.ResourcePredicateSuffix + ";";

        var offset = (page - 1) * pageSize;
        var parameters = new
        {
            UserId = userId.Format(),
            ResourceType = EnumFormatter<ResourceType>.GetValue(resourceType),
            Action = EnumFormatter<ResourceAction>.GetValue(action),
            PageSize = pageSize,
            Offset = offset,
            cancellationToken
        };

        var totalCount = await db.QuerySingleAsync<int>(countSql, parameters, transaction: tx());
        var rows = await db.QueryAsync<UserWithActorDto>(selectSql, parameters, transaction: tx());

        return new PagedResult<UserDetails>(rows.ToDetails(), totalCount, page, pageSize);
    }

    public async Task<IEnumerable<User>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Users WHERE Id IN (SELECT value FROM json_each(@Ids)) ORDER BY Name ASC";
        var result = await db.QueryAsync<UserDto>(sql, new { Ids = JsonSerializer.Serialize(ids, DeploymentJsonContext.Default.IEnumerableGuid), cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public async Task<(bool NameExists, bool EmailExists)> GetConflictsAsync(string name, string email, Guid? excludeId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                EXISTS (SELECT 1 FROM Users WHERE Name = @Name AND (@ExcludeId IS NULL OR Id != @ExcludeId)) AS NameExists,
                EXISTS (SELECT 1 FROM Users WHERE Email = @Email AND (@ExcludeId IS NULL OR Id != @ExcludeId)) AS EmailExists
            """;

        var result = await db.QuerySingleAsync<UserConflictCheckDto>(sql, new { Name = name, Email = email, ExcludeId = excludeId?.Format(), cancellationToken }, transaction: tx());
        return (result.NameExists, result.EmailExists);
    }

    public async Task<(User? User, bool IsEnabled, bool NameExists, bool EmailExists)> GetUserUpdateStateAsync(Guid id, string? name, string? email, CancellationToken cancellationToken)
    {
        const string sql = """
            WITH TargetUser AS (
                SELECT
                    u.Id,
                    u.Name,
                    u.Email,
                    u.Password,
                    u.ActorId,
                    a.IsEnabled,
                    u.CreatedAt,
                    u.CreatedByActorId
                FROM Users u
                JOIN Actors a ON a.Id = u.ActorId
                WHERE u.Id = @Id
            )
            SELECT
                tu.Id,
                tu.Name,
                tu.Email,
                tu.Password,
                tu.ActorId,
                tu.IsEnabled,
                tu.CreatedAt,
                tu.CreatedByActorId,
                CASE WHEN tu.Id IS NULL OR @Name IS NULL OR @Name = tu.Name THEN 0
                     ELSE EXISTS (SELECT 1 FROM Users WHERE Name = @Name AND Id != @Id)
                END AS NameExists,
                CASE WHEN tu.Id IS NULL OR @Email IS NULL OR @Email = tu.Email THEN 0
                     ELSE EXISTS (SELECT 1 FROM Users WHERE Email = @Email AND Id != @Id)
                END AS EmailExists
            FROM (SELECT 1) seed
            LEFT JOIN TargetUser tu ON 1 = 1
            """;

        var result = await db.QuerySingleAsync<UserUpdateStateDto>(sql, new { Id = id.Format(), Name = name, Email = email, cancellationToken }, transaction: tx());
        var user = result.Id.HasValue && result.ActorId.HasValue && result.CreatedAt.HasValue && result.CreatedByActorId.HasValue && result.Name is not null && result.Email is not null && result.Password is not null
            ? User.FromPersistence(result.Id.Value, result.Name, result.Email, result.Password, result.ActorId.Value, result.CreatedByActorId.Value, result.CreatedAt.Value)
            : null;

        return (user, result.IsEnabled ?? false, result.NameExists, result.EmailExists);
    }

    public Task<bool> ExistsByNameAsync(string name, Guid? excludeId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Users WHERE Name = @Name AND (@ExcludeId IS NULL OR Id != @ExcludeId))";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, ExcludeId = excludeId?.Format(), cancellationToken }, transaction: tx());
    }

    public Task<bool> ExistsByEmailAsync(string email, Guid? excludeId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Users WHERE Email = @Email AND (@ExcludeId IS NULL OR Id != @ExcludeId))";
        return db.ExecuteScalarAsync<bool>(sql, new { Email = email, ExcludeId = excludeId?.Format(), cancellationToken }, transaction: tx());
    }

    public Task<int> AddAsync(User user, CancellationToken cancellationToken)
    {
        const string sql = "INSERT INTO Users (Id, Name, Email, Password, ActorId, CreatedAt, CreatedByActorId) VALUES (@Id, @Name, @Email, @Password, @ActorId, @CreatedAt, @CreatedByActorId)";
        return db.ExecuteAsync(sql, new
        {
            Id = user.Id.Format(),
            user.Name,
            user.Email,
            user.Password,
            ActorId = user.ActorId.Format(),
            user.CreatedAt,
            CreatedByActorId = user.CreatedByActorId.Format(),
            cancellationToken
        }, transaction: tx());
    }

    public Task<int> UpdateAsync(User user, CancellationToken cancellationToken)
    {
        const string sql = "UPDATE Users SET Name = @Name, Email = @Email, Password = @Password WHERE Id = @Id";
        return db.ExecuteAsync(sql, new { Id = user.Id.Format(), user.Name, user.Email, user.Password, cancellationToken }, transaction: tx());
    }

    public Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM Users WHERE Id IN (SELECT value FROM json_each(@Ids))";
        return db.ExecuteAsync(sql, new { Ids = JsonSerializer.Serialize(ids, DeploymentJsonContext.Default.IEnumerableGuid), cancellationToken }, transaction: tx());
    }

    public Task<IEnumerable<Guid>> GetTeamIdsAsync(Guid userId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT TeamId FROM UsersTeams WHERE UserId = @UserId";
        return db.QueryAsync<Guid>(sql, new { UserId = userId.Format(), cancellationToken }, transaction: tx());
    }

    public Task<IEnumerable<Guid>> GetActorRoleIdsAsync(Guid actorId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT RoleId FROM ActorRoles WHERE ActorId = @ActorId";
        return db.QueryAsync<Guid>(sql, new { ActorId = actorId.Format(), cancellationToken }, transaction: tx());
    }

    public async Task<(UserDetails? User, bool RoleExists, bool HasRole)> GetRoleAssignmentStateAsync(Guid userId, Guid roleId, CancellationToken cancellationToken)
    {
        const string sql = """
            WITH TargetUser AS (
                SELECT
                    u.Id,
                    u.Name,
                    u.Email,
                    u.ActorId,
                    a.IsEnabled,
                    u.CreatedAt,
                    u.CreatedByActorId
                FROM Users u
                JOIN Actors a ON a.Id = u.ActorId
                WHERE u.Id = @UserId
            ),
            TargetRole AS (
                SELECT 1 AS ExistsFlag
                FROM Roles
                WHERE Id = @RoleId
            ),
            ExistingAssignment AS (
                SELECT 1 AS ExistsFlag
                FROM ActorRoles ar
                JOIN TargetUser tu ON tu.ActorId = ar.ActorId
                WHERE ar.RoleId = @RoleId
            )
            SELECT
                tu.Id,
                tu.Name,
                tu.Email,
                tu.ActorId,
                tu.IsEnabled,
                tu.CreatedAt,
                tu.CreatedByActorId,
                EXISTS (SELECT 1 FROM TargetRole) AS RoleExists,
                EXISTS (SELECT 1 FROM ExistingAssignment) AS HasRole
            FROM (SELECT 1) seed
            LEFT JOIN TargetUser tu ON 1 = 1
            """;

        var result = await db.QuerySingleAsync<UserRoleAssignmentStateDto>(sql, new { UserId = userId.Format(), RoleId = roleId.Format(), cancellationToken }, transaction: tx());
        var user = result.Id.HasValue && result.ActorId.HasValue && result.IsEnabled.HasValue && result.CreatedAt.HasValue && result.CreatedByActorId.HasValue
            ? new UserDetails(result.Id.Value, result.Name!, result.Email!, result.ActorId.Value, result.IsEnabled.Value, result.CreatedAt.Value, result.CreatedByActorId.Value)
            : null;

        return (user, result.RoleExists, result.HasRole);
    }

    public Task<int> AddActorRoleAsync(Guid actorId, Guid roleId, CancellationToken cancellationToken)
    {
        const string sql = "INSERT INTO ActorRoles (ActorId, RoleId) VALUES (@ActorId, @RoleId)";
        return db.ExecuteAsync(sql, new { ActorId = actorId.Format(), RoleId = roleId.Format(), cancellationToken }, transaction: tx());
    }

    public Task<int> RemoveActorRoleAsync(Guid actorId, Guid roleId, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM ActorRoles WHERE ActorId = @ActorId AND RoleId = @RoleId";
        return db.ExecuteAsync(sql, new { ActorId = actorId.Format(), RoleId = roleId.Format(), cancellationToken }, transaction: tx());
    }

    public async Task<int> ReplaceTeamsAsync(Guid userId, IEnumerable<Guid> teamIds, CancellationToken cancellationToken)
    {
        const string deleteSql = "DELETE FROM UsersTeams WHERE UserId = @UserId";
        await db.ExecuteAsync(deleteSql, new { UserId = userId.Format(), cancellationToken }, transaction: tx());

        var rows = 0;
        foreach (var teamId in teamIds)
        {
            const string insertSql = "INSERT INTO UsersTeams (UserId, TeamId) VALUES (@UserId, @TeamId)";
            rows += await db.ExecuteAsync(insertSql, new { UserId = userId.Format(), TeamId = teamId.Format(), cancellationToken }, transaction: tx());
        }

        return rows;
    }

    public async Task<int> ReplaceActorRolesAsync(Guid actorId, IEnumerable<Guid> roleIds, CancellationToken cancellationToken)
    {
        const string deleteSql = "DELETE FROM ActorRoles WHERE ActorId = @ActorId";
        await db.ExecuteAsync(deleteSql, new { ActorId = actorId.Format(), cancellationToken }, transaction: tx());

        var rows = 0;
        foreach (var roleId in roleIds)
        {
            const string insertSql = "INSERT INTO ActorRoles (ActorId, RoleId) VALUES (@ActorId, @RoleId)";
            rows += await db.ExecuteAsync(insertSql, new { ActorId = actorId.Format(), RoleId = roleId.Format(), cancellationToken }, transaction: tx());
        }

        return rows;
    }

    public async Task<bool> HasPermissionAsync(Guid userId, ResourceType resourceType, ResourceAction action, Guid? resourceId, CancellationToken ct)
    {
        const string sql = """
            WITH ActorScope AS (
                SELECT Users.ActorId 
                FROM Users 
                JOIN Actors userActor ON userActor.Id = Users.ActorId
                WHERE Users.Id = @UserId
                  AND userActor.IsEnabled = 1

                UNION

                -- Team actors
                SELECT t.ActorId
                FROM Teams t
                JOIN UsersTeams ut ON ut.TeamId = t.Id
                JOIN Actors teamActor ON teamActor.Id = t.ActorId
                WHERE ut.UserId = @UserId
                  AND teamActor.IsEnabled = 1
            ),
            GlobalAccess AS (
                SELECT 1
                FROM ActorRoles ar
                JOIN Permissions p ON p.RoleId = ar.RoleId
                JOIN ActorScope actorScope ON actorScope.ActorId = ar.ActorId
                WHERE p.ResourceType = @ResourceType
                  AND p.ResourceAction = @Action
            )
            SELECT
                EXISTS (
                    SELECT 1
                    FROM GlobalAccess
                )
                OR
                (
                    @ResourceId IS NOT NULL AND EXISTS (
                        SELECT 1
                        FROM ResourceAccesses ra
                        JOIN ActorScope actorScope ON actorScope.ActorId = ra.ActorId
                        WHERE ra.ResourceType = @ResourceType
                          AND ra.ResourceId = @ResourceId
                          AND ra.Action = @Action
                    )
                );
            """;

        return await db.ExecuteScalarAsync<bool>(sql, new
        {
            UserId = userId.Format(),
            ResourceId = resourceId?.Format(),
            ResourceType = EnumFormatter<ResourceType>.GetValue(resourceType),
            Action = EnumFormatter<ResourceAction>.GetValue(action)
        });
    }

    public async Task<UserAuthInfo?> GetUserAuthInfoByEmailOrNameAsync(string emailOrName, CancellationToken cancellationToken)
    {
        const string sql =
            """
            WITH TargetUser AS (
                SELECT Users.*
                FROM Users
                JOIN Actors userActor ON userActor.Id = Users.ActorId
                WHERE (Users.Email = @EmailOrName OR Users.Name = @EmailOrName)
                  AND userActor.IsEnabled = 1
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
                JOIN Actors teamActor ON teamActor.Id = Teams.ActorId
                WHERE teamActor.IsEnabled = 1
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
            new { EmailOrName = emailOrName },
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
