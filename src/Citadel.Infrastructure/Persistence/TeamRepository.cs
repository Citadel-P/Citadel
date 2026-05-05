using Dapper;
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

internal sealed class TeamRepository(IDbConnection db, Func<IDbTransaction> tx) : ITeamRepository
{
    private const string TeamAggregateCtes = """
        TeamMembers AS (
            SELECT ut.TeamId, COUNT(*)::int AS TotalMembers
            FROM UsersTeams ut
            GROUP BY ut.TeamId
        ),
        TeamRoles AS (
            SELECT ar.ActorId, ARRAY_AGG(DISTINCT r.Name ORDER BY r.Name) AS Roles
            FROM ActorRoles ar
            JOIN Roles r ON r.Id = ar.RoleId
            GROUP BY ar.ActorId
        )
        """;

    private const string TeamAggregateJoins = """

        LEFT JOIN TeamMembers members ON members.TeamId = t.Id
        LEFT JOIN TeamRoles roles ON roles.ActorId = t.ActorId

        """;

    private const string TargetTeamAggregateJoins = """

        LEFT JOIN TeamMembers members ON members.TeamId = tt.Id
        LEFT JOIN TeamRoles roles ON roles.ActorId = tt.ActorId

        """;

    public async Task<Team?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Teams WHERE Id = @Id";
        var result = await db.QuerySingleOrDefaultAsync<TeamDto>(sql, new { Id = id, cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<Team>> GetAllAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Teams ORDER BY Name ASC";
        var result = await db.QueryAsync<TeamDto>(sql, transaction: tx());
        return result.ToDomain();
    }

    public async Task<TeamDetails?> GetDetailsAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "WITH " + TeamAggregateCtes + " " + """
            SELECT
                t.Id,
                t.Name,
                t.ActorId,
                a.IsEnabled,
                COALESCE(members.TotalMembers, 0) AS TotalMembers,
                COALESCE(roles.Roles, ARRAY[]::text[]) AS Roles
            FROM Teams t
            JOIN Actors a ON a.Id = t.ActorId
            """ + TeamAggregateJoins + """
            WHERE t.Id = @Id
            LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<TeamWithActorDto>(sql, new { Id = id, cancellationToken }, transaction: tx());
        return result?.ToDetails();
    }

    public async Task<PagedResult<TeamDetails>> GetPagedAsync(int page, int pageSize, string? name, CancellationToken cancellationToken)
    {
        const string selectSql = "WITH " + TeamAggregateCtes + " " + """
        SELECT
            t.Id,
            t.Name,
            t.ActorId,
            a.IsEnabled,
            COALESCE(members.TotalMembers, 0) AS TotalMembers,
            COALESCE(roles.Roles, ARRAY[]::text[]) AS Roles
        FROM Teams t
        JOIN Actors a ON a.Id = t.ActorId
        """ + TeamAggregateJoins + """
        WHERE (@Name IS NULL OR t.Name ILIKE '%' || @Name || '%')
        ORDER BY t.Name ASC
        LIMIT @PageSize OFFSET @Offset
        """;
        const string countSql = """
        SELECT COUNT(*)
        FROM Teams t
        JOIN Actors a ON a.Id = t.ActorId
        WHERE (@Name IS NULL OR t.Name ILIKE '%' || @Name || '%')
        """;

        var offset = (page - 1) * pageSize;
        var totalCount = await db.QuerySingleAsync<int>(countSql, new { Name = name }, transaction: tx());
        var rows = await db.QueryAsync<TeamWithActorDto>(selectSql, new { PageSize = pageSize, Offset = offset, Name = name, cancellationToken }, transaction: tx());

        return new PagedResult<TeamDetails>(rows.ToDetails(), totalCount, page, pageSize);
    }

    public async Task<PagedResult<TeamDetails>> GetAuthorizedPagedAsync(Guid userId, ResourceType resourceType, ResourceAction action, int page, int pageSize, string? name, CancellationToken cancellationToken)
    {
        const string selectSql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + ", " + TeamAggregateCtes + " " + """
            SELECT
                t.Id,
                t.Name,
                t.ActorId,
                a.IsEnabled,
                COALESCE(members.TotalMembers, 0) AS TotalMembers,
                COALESCE(roles.Roles, ARRAY[]::text[]) AS Roles
            FROM Teams t
            JOIN Actors a ON a.Id = t.ActorId
            """ + TeamAggregateJoins + """
            WHERE (@Name IS NULL OR t.Name ILIKE '%' || @Name || '%') AND
        """ + AuthorizationSql.ResourcePredicatePrefix + "t.Id" + AuthorizationSql.ResourcePredicateSuffix + " ORDER BY t.Name ASC LIMIT @PageSize OFFSET @Offset;";

        const string countSql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + " SELECT COUNT(*) FROM Teams t WHERE (@Name IS NULL OR t.Name ILIKE '%' || @Name || '%') AND"
            + AuthorizationSql.ResourcePredicatePrefix + "t.Id" + AuthorizationSql.ResourcePredicateSuffix + ";";

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
        var rows = await db.QueryAsync<TeamWithActorDto>(selectSql, parameters, transaction: tx());

        return new PagedResult<TeamDetails>(rows.ToDetails(), totalCount, page, pageSize);
    }

    public Task<IEnumerable<TeamSearchItem>> SearchAsync(string query, int limit, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, Name
            FROM Teams
            WHERE Name ILIKE '%' || @Query || '%'
            ORDER BY Name ASC
            LIMIT @Limit
            """;

        return db.QueryAsync<TeamSearchItem>(sql, new { Query = query, Limit = limit, cancellationToken }, transaction: tx());
    }

    public Task<IEnumerable<TeamSearchItem>> SearchAuthorizedAsync(Guid userId, ResourceType resourceType, ResourceAction action, string query, int limit, CancellationToken cancellationToken)
    {
        const string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + " " + """
            SELECT t.Id, t.Name
            FROM Teams t
            WHERE t.Name ILIKE '%' || @Query || '%' AND
        """ + AuthorizationSql.ResourcePredicatePrefix + "t.Id" + AuthorizationSql.ResourcePredicateSuffix + " ORDER BY t.Name ASC LIMIT @Limit;";

        var parameters = new
        {
            UserId = userId,
            ResourceType = EnumFormatter<ResourceType>.GetValue(resourceType),
            Action = EnumFormatter<ResourceAction>.GetValue(action),
            Query = query,
            Limit = limit,
            cancellationToken
        };

        return db.QueryAsync<TeamSearchItem>(sql, parameters, transaction: tx());
    }

    public async Task<IEnumerable<Team>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Teams WHERE Id = ANY(@Ids) ORDER BY Name ASC";
        var idArray = ids as Guid[] ?? [.. ids];
        var result = await db.QueryAsync<TeamDto>(sql, new { Ids = idArray, cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public async Task<bool> GetConflictsAsync(string name, Guid? excludeId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Teams WHERE Name = @Name AND (@ExcludeId IS NULL OR Id != @ExcludeId)) AS NameExists";
        var result = await db.QuerySingleAsync<TeamConflictCheckDto>(sql, new { Name = name, ExcludeId = excludeId, cancellationToken }, transaction: tx());
        return result.NameExists;
    }

    public async Task<(Team? Team, bool IsEnabled, bool NameExists)> GetTeamUpdateStateAsync(Guid id, string? name, CancellationToken cancellationToken)
    {
        const string sql = """
            WITH TargetTeam AS (
                SELECT
                    t.Id,
                    t.Name,
                    t.ActorId,
                    a.IsEnabled
                FROM Teams t
                JOIN Actors a ON a.Id = t.ActorId
                WHERE t.Id = @Id
            )
            SELECT
                tt.Id,
                tt.Name,
                tt.ActorId,
                tt.IsEnabled,
                CASE WHEN tt.Id IS NULL OR @Name IS NULL OR @Name = tt.Name THEN false
                     ELSE EXISTS (SELECT 1 FROM Teams WHERE Name = @Name AND Id != @Id)
                END AS NameExists
            FROM (SELECT 1) seed
            LEFT JOIN TargetTeam tt ON 1 = 1
            """;

        var result = await db.QuerySingleAsync<TeamUpdateStateDto>(sql, new { Id = id, Name = name, cancellationToken }, transaction: tx());
        var team = result.Id.HasValue && result.ActorId.HasValue && result.Name is not null
            ? Team.FromPersistence(result.Id.Value, result.Name, result.ActorId.Value)
            : null;

        return (team, result.IsEnabled ?? false, result.NameExists);
    }

    public Task<bool> ExistsByNameAsync(string name, Guid? excludeId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Teams WHERE Name = @Name AND (@ExcludeId IS NULL OR Id != @ExcludeId))";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, ExcludeId = excludeId, cancellationToken }, transaction: tx());
    }

    public Task<int> AddAsync(Team team, CancellationToken cancellationToken)
    {
        const string sql = "INSERT INTO Teams (Id, Name, ActorId) VALUES (@Id, @Name, @ActorId)";
        return db.ExecuteAsync(sql, new { Id = team.Id, team.Name, ActorId = team.ActorId, cancellationToken }, transaction: tx());
    }

    public Task<int> UpdateAsync(Team team, CancellationToken cancellationToken)
    {
        const string sql = "UPDATE Teams SET Name = @Name WHERE Id = @Id";
        return db.ExecuteAsync(sql, new { Id = team.Id, team.Name, cancellationToken }, transaction: tx());
    }

    public Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM Teams WHERE Id = ANY(@Ids)";
        var idArray = ids as Guid[] ?? [.. ids];
        return db.ExecuteAsync(sql, new { Ids = idArray, cancellationToken }, transaction: tx());
    }

    public Task<IEnumerable<Guid>> GetUserIdsAsync(Guid teamId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT UserId FROM UsersTeams WHERE TeamId = @TeamId";
        return db.QueryAsync<Guid>(sql, new { TeamId = teamId, cancellationToken }, transaction: tx());
    }

    public async Task<(TeamDetails? Team, bool UserExists, bool HasMember)> GetMemberAssignmentStateAsync(Guid teamId, Guid userId, CancellationToken cancellationToken)
    {
        const string sql = "WITH " + """
            TargetTeam AS (
                SELECT
                    t.Id,
                    t.Name,
                    t.ActorId,
                    a.IsEnabled
                FROM Teams t
                JOIN Actors a ON a.Id = t.ActorId
                WHERE t.Id = @TeamId
            ),
            TargetUser AS (
                SELECT 1 AS ExistsFlag
                FROM Users
                WHERE Id = @UserId
            ),
            ExistingMember AS (
                SELECT 1 AS ExistsFlag
                FROM UsersTeams ut
                WHERE ut.TeamId = @TeamId
                  AND ut.UserId = @UserId
            )
            """ + ", " + TeamAggregateCtes + " " + """
            SELECT
                tt.Id,
                tt.Name,
                tt.ActorId,
                tt.IsEnabled,
                COALESCE(members.TotalMembers, 0) AS TotalMembers,
                COALESCE(roles.Roles, ARRAY[]::text[]) AS Roles,
                EXISTS (SELECT 1 FROM TargetUser) AS UserExists,
                EXISTS (SELECT 1 FROM ExistingMember) AS HasMember
            FROM (SELECT 1) seed
            LEFT JOIN TargetTeam tt ON 1 = 1
            """ + TargetTeamAggregateJoins;

        var result = await db.QuerySingleAsync<TeamMemberAssignmentStateDto>(sql, new { TeamId = teamId, UserId = userId, cancellationToken }, transaction: tx());
        var team = result.Id.HasValue && result.ActorId.HasValue && result.IsEnabled.HasValue && result.Name is not null
            ? new TeamDetails(result.Id.Value, result.Name, result.ActorId.Value, result.IsEnabled.Value, result.TotalMembers, result.Roles)
            : null;

        return (team, result.UserExists, result.HasMember);
    }

    public Task<int> AddMemberAsync(Guid teamId, Guid userId, CancellationToken cancellationToken)
    {
        const string sql = "INSERT INTO UsersTeams (UserId, TeamId) VALUES (@UserId, @TeamId)";
        return db.ExecuteAsync(sql, new { UserId = userId, TeamId = teamId, cancellationToken }, transaction: tx());
    }

    public Task<int> RemoveMemberAsync(Guid teamId, Guid userId, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM UsersTeams WHERE TeamId = @TeamId AND UserId = @UserId";
        return db.ExecuteAsync(sql, new { TeamId = teamId, UserId = userId, cancellationToken }, transaction: tx());
    }

    public async Task<int> ReplaceMembersAsync(Guid teamId, IEnumerable<Guid> userIds, CancellationToken cancellationToken)
    {
        const string deleteSql = "DELETE FROM UsersTeams WHERE TeamId = @TeamId";
        await db.ExecuteAsync(deleteSql, new { TeamId = teamId, cancellationToken }, transaction: tx());

        var rows = 0;
        foreach (var userId in userIds)
        {
            const string insertSql = "INSERT INTO UsersTeams (UserId, TeamId) VALUES (@UserId, @TeamId)";
            rows += await db.ExecuteAsync(insertSql, new { UserId = userId, TeamId = teamId, cancellationToken }, transaction: tx());
        }

        return rows;
    }
}
