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
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class UserRepository(IDbConnection db, Func<IDbTransaction> tx) : IUserRepository
{
    private const string UserAggregateCtes = """
        UserTeams AS (
            SELECT ut.UserId, ARRAY_AGG(DISTINCT t.Name ORDER BY t.Name) AS Teams
            FROM UsersTeams ut
            JOIN Teams t ON t.Id = ut.TeamId
            GROUP BY ut.UserId
        ),
        UserRoles AS (
            SELECT ar.ActorId, ARRAY_AGG(DISTINCT r.Name ORDER BY r.Name) AS Roles
            FROM ActorRoles ar
            JOIN Roles r ON r.Id = ar.RoleId
            GROUP BY ar.ActorId
        )
        """;

    private const string UserAggregateJoins = """

        LEFT JOIN UserTeams teams ON teams.UserId = u.Id
        LEFT JOIN UserRoles roles ON roles.ActorId = u.ActorId

        """;

    public async Task<User?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Users WHERE Id = @Id";
        var result = await db.QuerySingleOrDefaultAsync<UserDto>(sql, new { Id = id, cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<User>> GetAllAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Users ORDER BY Name ASC";
        var result = await db.QueryAsync<UserDto>(sql, transaction: tx());
        return result.ToDomain();
    }

    public async Task<PagedResult<UserDetails>> GetPagedAsync(int page, int pageSize, string? name, CancellationToken cancellationToken)
    {
        name = string.IsNullOrWhiteSpace(name) ? null : name;


        const string selectSql = "WITH " + UserAggregateCtes + " " + """
            SELECT
                u.Id,
                u.Name,
                u.Email,
                u.ActorId,
                a.IsEnabled,
                u.CreatedAt,
                u.CreatedByActorId,
                COALESCE(teams.Teams, ARRAY[]::text[]) AS Teams,
                COALESCE(roles.Roles, ARRAY[]::text[]) AS Roles
            FROM Users u
            JOIN Actors a ON a.Id = u.ActorId
            """ + UserAggregateJoins + """
            WHERE (@Name IS NULL OR u.Name ILIKE '%' || @Name || '%')
            ORDER BY u.Name ASC
            LIMIT @PageSize OFFSET @Offset
            """;

        const string countSql = """
            SELECT COUNT(*)
            FROM Users u
            JOIN Actors a ON a.Id = u.ActorId
            WHERE (@Name IS NULL OR u.Name ILIKE '%' || @Name || '%')
        """;

        var offset = (page - 1) * pageSize;

        var totalCount = await db.QuerySingleAsync<int>(countSql, new { Name = name }, transaction: tx());

        var rows = await db.QueryAsync<UserWithActorDto>(selectSql, new { PageSize = pageSize, Offset = offset, Name = name }, transaction: tx());

        return new PagedResult<UserDetails>(rows.ToDetails(), totalCount, page, pageSize);
    }

    public async Task<PagedResult<UserDetails>> GetAuthorizedPagedAsync(Guid userId, ResourceType resourceType, ResourceAction action, int page, int pageSize, string? name, CancellationToken cancellationToken)
    {
        const string selectSql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + ", " + UserAggregateCtes + " " + """
            SELECT
                u.Id,
                u.Name,
                u.Email,
                u.ActorId,
                a.IsEnabled,
                u.CreatedAt,
                u.CreatedByActorId,
                COALESCE(teams.Teams, ARRAY[]::text[]) AS Teams,
                COALESCE(roles.Roles, ARRAY[]::text[]) AS Roles
            FROM Users u
            JOIN Actors a ON a.Id = u.ActorId
            """ + UserAggregateJoins + """
            WHERE (@Name IS NULL OR u.Name ILIKE '%' || @Name || '%')
        """ + AuthorizationSql.ResourcePredicatePrefix + "u.Id" + AuthorizationSql.ResourcePredicateSuffix + " ORDER BY u.Name ASC LIMIT @PageSize OFFSET @Offset;";

        const string countSql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + " SELECT COUNT(*) FROM Users u WHERE (@Name IS NULL OR u.Name ILIKE '%' || @Name || '%') "
            + AuthorizationSql.ResourcePredicatePrefix + "u.Id" + AuthorizationSql.ResourcePredicateSuffix + ";";

        var offset = (page - 1) * pageSize;
        var parameters = new
        {
            UserId = userId,
            ResourceType = EnumFormatter<ResourceType>.GetValue(resourceType),
            Action = EnumFormatter<ResourceAction>.GetValue(action),
            PageSize = pageSize,
            Offset = offset,
            Name = name,
            cancellationToken
        };

        var totalCount = await db.QuerySingleAsync<int>(countSql, parameters, transaction: tx());
        var rows = await db.QueryAsync<UserWithActorDto>(selectSql, parameters, transaction: tx());

        return new PagedResult<UserDetails>(rows.ToDetails(), totalCount, page, pageSize);
    }

    public Task<IEnumerable<UserSearchItem>> SearchAsync(string query, int limit, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, Name, Email
            FROM Users
            WHERE Name ILIKE '%' || @Query || '%'
               OR Email ILIKE '%' || @Query || '%'
            ORDER BY Name ASC
            LIMIT @Limit
            """;

        return db.QueryAsync<UserSearchItem>(sql, new { Query = query, Limit = limit, cancellationToken }, transaction: tx());
    }

    public Task<IEnumerable<UserSearchItem>> SearchAuthorizedAsync(Guid userId, ResourceType resourceType, ResourceAction action, string query, int limit, CancellationToken cancellationToken)
    {
        const string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + " " + """
            SELECT u.Id, u.Name, u.Email
            FROM Users u
            WHERE (u.Name ILIKE '%' || @Query || '%' OR u.Email ILIKE '%' || @Query || '%') AND
        """ + AuthorizationSql.ResourcePredicatePrefix + "u.Id" + AuthorizationSql.ResourcePredicateSuffix + " ORDER BY u.Name ASC LIMIT @Limit;";

        var parameters = new
        {
            UserId = userId,
            ResourceType = EnumFormatter<ResourceType>.GetValue(resourceType),
            Action = EnumFormatter<ResourceAction>.GetValue(action),
            Query = query,
            Limit = limit,
            cancellationToken
        };

        return db.QueryAsync<UserSearchItem>(sql, parameters, transaction: tx());
    }

    public async Task<IEnumerable<User>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Users WHERE Id = ANY(@Ids) ORDER BY Name ASC";
        var idArray = ids as Guid[] ?? [.. ids];
        var result = await db.QueryAsync<UserDto>(sql, new { Ids = idArray, cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public async Task<(bool NameExists, bool EmailExists)> GetConflictsAsync(string name, string email, Guid? excludeId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                EXISTS (SELECT 1 FROM Users WHERE Name = @Name AND (@ExcludeId IS NULL OR Id != @ExcludeId)) AS NameExists,
                EXISTS (SELECT 1 FROM Users WHERE Email = @Email AND (@ExcludeId IS NULL OR Id != @ExcludeId)) AS EmailExists
            """;

        var result = await db.QuerySingleAsync<UserConflictCheckDto>(sql, new { Name = name, Email = email, ExcludeId = excludeId, cancellationToken }, transaction: tx());
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
                CASE WHEN tu.Id IS NULL OR @Name IS NULL OR @Name = tu.Name THEN false
                     ELSE EXISTS (SELECT 1 FROM Users WHERE Name = @Name AND Id != @Id)
                END AS NameExists,
                CASE WHEN tu.Id IS NULL OR @Email IS NULL OR @Email = tu.Email THEN false
                     ELSE EXISTS (SELECT 1 FROM Users WHERE Email = @Email AND Id != @Id)
                END AS EmailExists
            FROM (SELECT 1) seed
            LEFT JOIN TargetUser tu ON 1 = 1
            """;

        var result = await db.QuerySingleAsync<UserUpdateStateDto>(sql, new { Id = id, Name = name, Email = email, cancellationToken }, transaction: tx());
        var user = result.Id.HasValue && result.ActorId.HasValue && result.CreatedAt.HasValue && result.CreatedByActorId.HasValue && result.Name is not null && result.Email is not null && result.Password is not null
            ? User.FromPersistence(result.Id.Value, result.Name, result.Email, result.Password, result.ActorId.Value, result.CreatedByActorId.Value, result.CreatedAt.Value)
            : null;

        return (user, result.IsEnabled ?? false, result.NameExists, result.EmailExists);
    }

    public Task<bool> ExistsByNameAsync(string name, Guid? excludeId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Users WHERE Name = @Name AND (@ExcludeId IS NULL OR Id != @ExcludeId))";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, ExcludeId = excludeId, cancellationToken }, transaction: tx());
    }

    public Task<bool> ExistsByEmailAsync(string email, Guid? excludeId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Users WHERE Email = @Email AND (@ExcludeId IS NULL OR Id != @ExcludeId))";
        return db.ExecuteScalarAsync<bool>(sql, new { Email = email, ExcludeId = excludeId, cancellationToken }, transaction: tx());
    }

    public Task<int> AddAsync(User user, CancellationToken cancellationToken)
    {
        const string sql = "INSERT INTO Users (Id, Name, Email, Password, ActorId, CreatedAt, CreatedByActorId) VALUES (@Id, @Name, @Email, @Password, @ActorId, @CreatedAt, @CreatedByActorId)";
        return db.ExecuteAsync(sql, new
        {
            Id = user.Id,
            user.Name,
            user.Email,
            user.Password,
            ActorId = user.ActorId,
            user.CreatedAt,
            CreatedByActorId = user.CreatedByActorId,
            cancellationToken
        }, transaction: tx());
    }

    public Task<int> UpdateAsync(User user, CancellationToken cancellationToken)
    {
        const string sql = "UPDATE Users SET Name = @Name, Email = @Email, Password = @Password WHERE Id = @Id";
        return db.ExecuteAsync(sql, new { Id = user.Id, user.Name, user.Email, user.Password, cancellationToken }, transaction: tx());
    }

    public Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM Users WHERE Id = ANY(@Ids)";
        var idArray = ids as Guid[] ?? [.. ids];
        return db.ExecuteAsync(sql, new { Ids = idArray, cancellationToken }, transaction: tx());
    }

    public Task<IEnumerable<Guid>> GetTeamIdsAsync(Guid userId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT TeamId FROM UsersTeams WHERE UserId = @UserId";
        return db.QueryAsync<Guid>(sql, new { UserId = userId, cancellationToken }, transaction: tx());
    }

    public async Task<int> ReplaceTeamsAsync(Guid userId, IEnumerable<Guid> teamIds, CancellationToken cancellationToken)
    {
        const string deleteSql = "DELETE FROM UsersTeams WHERE UserId = @UserId";
        await db.ExecuteAsync(deleteSql, new { UserId = userId, cancellationToken }, transaction: tx());

        var teamIdArray = teamIds as Guid[] ?? [.. teamIds];
        if (teamIdArray.Length == 0)
            return 0;

        const string insertSql = """
            INSERT INTO UsersTeams (UserId, TeamId)
            SELECT @UserId, teamId
            FROM unnest(@TeamIds::uuid[]) AS teamId
            """;

        return await db.ExecuteAsync(insertSql, new { UserId = userId, TeamIds = teamIdArray, cancellationToken }, transaction: tx());
    }

    public async Task<bool> HasPermissionAsync(Guid userId, ResourceType resourceType, ResourceAction action, Guid? resourceId, CancellationToken ct)
    {
        const string sql = """
            WITH ActorScope AS (
                SELECT Users.ActorId 
                FROM Users 
                JOIN Actors userActor ON userActor.Id = Users.ActorId
                WHERE Users.Id = @UserId
                  AND userActor.IsEnabled

                UNION

                -- Team actors
                SELECT t.ActorId
                FROM Teams t
                JOIN UsersTeams ut ON ut.TeamId = t.Id
                JOIN Actors teamActor ON teamActor.Id = t.ActorId
                WHERE ut.UserId = @UserId
                  AND teamActor.IsEnabled
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
            UserId = userId,
            ResourceId = resourceId,
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
                  AND userActor.IsEnabled
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
                WHERE teamActor.IsEnabled
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
