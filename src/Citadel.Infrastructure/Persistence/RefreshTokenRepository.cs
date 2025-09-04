using System.Data;
using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using Infrastructure.Persistence.Dtos;
using Infrastructure.TypeHandlers;

namespace Infrastructure.Persistence;

internal class RefreshTokenRepository(IDbConnection db, Func<IDbTransaction> tx) : IRefreshTokenRepository 
{
    public Task<int> AddAsync(RefreshToken refreshToken, CancellationToken cancellationToken)
    {
        const string sql = "INSERT INTO RefreshTokens (Id, UserId, CreatedAt) VALUES (@Id, @UserId, @CreatedAt)";
        var parameters = new
        {
            Id = refreshToken.Id.Format(),
            UserId = refreshToken.UserId.Format(),
            CreatedAt =refreshToken.CreatedAt.ToString(),
        };
        return db.ExecuteAsync(sql, parameters, tx());
    }

    public Task<int> CountAsync(Guid userId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT COUNT(*) FROM RefreshTokens WHERE UserId = @UserId";
        return db.ExecuteScalarAsync<int>(sql, new { UserId = userId.Format() }, tx());
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
                Roles.Name as RoleName, 
                Permissions.PermissionCode
            FROM Token
            JOIN Users ON Users.Id = Token.UserId
            LEFT JOIN UsersTeams ON Users.Id = UsersTeams.UserId
            LEFT JOIN Teams ON Teams.Id = UsersTeams.TeamId
            LEFT JOIN Roles ON Teams.RoleId = Roles.Id
            LEFT JOIN Permissions ON Roles.Id = Permissions.RoleId;
            """;

        var result = await db.QueryAsync<UserAuthInfoDto>(sql,
            new { Id = id.Format() },
            transaction: tx());

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

        return db.ExecuteAsync(sql, new { UserId = userId.Format(), Limit = tokensToRemoveCount }, transaction: tx());
    }

    public Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM RefreshTokens WHERE Id = @Id";
        return db.ExecuteAsync(sql, new { Id = id.Format() }, tx());
    }
}
