using System.Data;
using System.Threading;
using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;

namespace Infrastructure.Persistence;

internal class RefreshTokenRepository(IDbConnection db, IDbTransaction tx) : IRefreshTokenRepository 
{
    public Task<int> AddAsync(RefreshToken refreshToken, CancellationToken cancellationToken)
    {
        const string sql = "INSERT INTO RefreshTokens (Id, UserId, CreatedAt) VALUES (@Id, @UserId, @CreatedAt)";
        var parameters = new
        {
            Id = refreshToken.Id,
            UserId = refreshToken.UserId,
            CreatedAt = refreshToken.CreatedAt
        };
        return db.ExecuteAsync(new CommandDefinition(
            sql,
            parameters,
            tx, 
            cancellationToken: cancellationToken));
    }

    public Task<int> CountAsync(Guid userId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT COUNT(*) FROM RefreshTokens WHERE UserId = @UserId";
        return db.ExecuteScalarAsync<int>(new CommandDefinition(
            sql,
            new { UserId = userId },
            tx, 
            cancellationToken: cancellationToken));
    }

    public async Task<UserAuthInfo?> GetUserAuthInfoByRefreshTokenIdAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql =
            """
            WITH Token AS (
                SELECT UserId FROM RefreshTokens WHERE Id = @Id LIMIT 1
            )
            SELECT 
                Users.Id, 
                Users.Name, 
                Users.Email,
                Users.Password,
                Roles.Name as RoleName, 
                Permissions.PermissionCode
            FROM Token
            JOIN Users ON Users.Id = Token.UserId
            LEFT JOIN UsersTeams ON Users.Id = UsersTeams.UserId
            LEFT JOIN Teams ON Teams.Id = UsersTeams.TeamId
            LEFT JOIN Roles ON Teams.RoleId = Roles.Id
            LEFT JOIN Permissions ON Roles.Id = Permissions.RoleId;
            """;

        var result = await db.QueryAsync<(Guid Id, string Name, string Email, string RoleName, int? PermissionCode)>(new CommandDefinition(
            sql,
            new { Id = id },
            transaction: tx,
            cancellationToken: cancellationToken));

        return result
            .GroupBy(r => new { r.Id, r.Name, r.Email })
            .Select(g => new UserAuthInfo(
                g.Key.Id,
                g.Key.Name,
                g.Key.Email,
                null,
                [.. g.Where(r => !string.IsNullOrWhiteSpace(r.RoleName))
                    .Select(r => r.RoleName!)
                    .Distinct()],
                [.. g.Where(r => r.PermissionCode.HasValue)
                    .Select(r => (AppPermission)r.PermissionCode!.Value)
                    .Distinct()]
            )).FirstOrDefault();
    }

    public Task<int> DeleteOldestTokensAsync(Guid userId, int tokensToRemoveCount, CancellationToken cancellationToken)
    {
        const string sql = 
            """
            DELETE FROM RefreshTokens
            WHERE Id IN (
                SELECT Id
                FROM RefreshTokens
                WHERE UserId = @UserId
                ORDER BY CreatedAt ASC
                LIMIT @Limit
            )
            """;

        var cmd = new CommandDefinition(sql, new { UserId = userId, Limit = tokensToRemoveCount }, transaction: tx, cancellationToken: cancellationToken);
        return db.ExecuteAsync(cmd);
    }

    public Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM RefreshTokens WHERE Id = @Id";
        return db.ExecuteAsync(new CommandDefinition(
            sql,
            new { Id = id },
            tx, 
            cancellationToken: cancellationToken));
    }
}
