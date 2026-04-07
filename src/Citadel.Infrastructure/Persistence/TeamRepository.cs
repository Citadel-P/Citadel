using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using System.Text.Json;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class TeamRepository(IDbConnection db, Func<IDbTransaction> tx) : ITeamRepository
{
    public async Task<Team?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Teams WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<TeamDto>(sql, new { Id = id.Format(), cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<Team>> GetAllAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Teams ORDER BY Name ASC";
        var result = await db.QueryAsync<TeamDto>(sql, transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<Team>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Teams WHERE Id IN (SELECT value FROM json_each(@Ids)) ORDER BY Name ASC";
        var result = await db.QueryAsync<TeamDto>(sql, new { Ids = JsonSerializer.Serialize(ids, DeploymentJsonContext.Default.IEnumerableGuid), cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public Task<bool> ExistsByNameAsync(string name, Guid? excludeId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Teams WHERE Name = @Name AND (@ExcludeId IS NULL OR Id != @ExcludeId))";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, ExcludeId = excludeId?.Format(), cancellationToken }, transaction: tx());
    }

    public Task<int> AddAsync(Team team, CancellationToken cancellationToken)
    {
        const string sql = "INSERT INTO Teams (Id, Name, ActorId) VALUES (@Id, @Name, @ActorId)";
        return db.ExecuteAsync(sql, new { Id = team.Id.Format(), team.Name, ActorId = team.ActorId.Format(), cancellationToken }, transaction: tx());
    }

    public Task<int> UpdateAsync(Team team, CancellationToken cancellationToken)
    {
        const string sql = "UPDATE Teams SET Name = @Name WHERE Id = @Id";
        return db.ExecuteAsync(sql, new { Id = team.Id.Format(), team.Name, cancellationToken }, transaction: tx());
    }

    public Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM Teams WHERE Id IN (SELECT value FROM json_each(@Ids))";
        return db.ExecuteAsync(sql, new { Ids = JsonSerializer.Serialize(ids, DeploymentJsonContext.Default.IEnumerableGuid), cancellationToken }, transaction: tx());
    }

    public Task<IEnumerable<Guid>> GetUserIdsAsync(Guid teamId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT UserId FROM UsersTeams WHERE TeamId = @TeamId";
        return db.QueryAsync<Guid>(sql, new { TeamId = teamId.Format(), cancellationToken }, transaction: tx());
    }

    public Task<IEnumerable<Guid>> GetActorRoleIdsAsync(Guid actorId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT RoleId FROM ActorRoles WHERE ActorId = @ActorId";
        return db.QueryAsync<Guid>(sql, new { ActorId = actorId.Format(), cancellationToken }, transaction: tx());
    }

    public async Task<int> ReplaceMembersAsync(Guid teamId, IEnumerable<Guid> userIds, CancellationToken cancellationToken)
    {
        const string deleteSql = "DELETE FROM UsersTeams WHERE TeamId = @TeamId";
        await db.ExecuteAsync(deleteSql, new { TeamId = teamId.Format(), cancellationToken }, transaction: tx());

        var rows = 0;
        foreach (var userId in userIds)
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
}
