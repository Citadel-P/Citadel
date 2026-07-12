using System.Data;
using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;

namespace Infrastructure.Persistence;

internal class RefreshTokenRepository(IDbConnection db, Func<IDbTransaction> tx) : IRefreshTokenRepository 
{
    public Task<int> AddAsync(RefreshToken refreshToken, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO RefreshTokens (
                Id,
                UserId,
                CreatedAt,
                LastSeenAt,
                ExpiresAt,
                UserAgent,
                IpAddress)
            VALUES (
                @Id,
                @UserId,
                @CreatedAt,
                @LastSeenAt,
                @ExpiresAt,
                @UserAgent,
                @IpAddress)
            """;
        var parameters = new
        {
            Id = refreshToken.Id,
            UserId = refreshToken.UserId,
            CreatedAt = refreshToken.CreatedAt,
            LastSeenAt = refreshToken.LastSeenAt,
            ExpiresAt = refreshToken.ExpiresAt,
            UserAgent = refreshToken.UserAgent,
            IpAddress = refreshToken.IpAddress,
        };
        return db.ExecuteAsync(sql, parameters, tx());
    }

    public Task<int> CountAsync(Guid userId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT COUNT(*) FROM RefreshTokens WHERE UserId = @UserId";
        return db.ExecuteScalarAsync<int>(sql, new { UserId = userId }, tx());
    }

    public async Task<UserAuthInfo?> GetUserAuthInfoByRefreshTokenIdAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql =
            """
            WITH Token AS (
                SELECT UserId FROM RefreshTokens WHERE Id = @Id AND ExpiresAt > @Now LIMIT 1
            ),
            TargetUser AS (
                SELECT Users.Id, Users.Name, Users.Email, Users.ActorId
                FROM Token
                JOIN Users ON Users.Id = Token.UserId
                JOIN Actors userActor ON userActor.Id = Users.ActorId
                WHERE userActor.IsEnabled
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
                '' AS Password,
                Roles.Name as RoleName, 
                Permissions.ResourceType::integer AS PermissionResourceType,
                Permissions.PermissionLevel::integer AS PermissionLevel,
                Permissions.SpecificPermissions::integer AS SpecificPermissions
            FROM TargetUser
            LEFT JOIN ActorScope ON 1 = 1
            LEFT JOIN ActorRoles ON ActorRoles.ActorId = ActorScope.ActorId
            LEFT JOIN Roles ON Roles.Id = ActorRoles.RoleId
            LEFT JOIN Permissions ON Roles.Id = Permissions.RoleId
            """;

        var result = await db.QueryAsync<UserAuthInfoDto>(sql,
            new { Id = id, Now = DateTime.UtcNow },
            transaction: tx());

        return result
            .GroupBy(r => new { r.Id, r.Name, r.Email, r.ActorId })
            .Select(g => new UserAuthInfo(
               g.Key.Id,
               g.Key.ActorId,
               g.Key.Name,
               g.Key.Email,
                null,
                [.. g.Where(r => !string.IsNullOrWhiteSpace(r.RoleName))
                    .Select(r => r.RoleName!)
                    .Distinct()]
            )).FirstOrDefault();
    }

    public async Task<Guid?> GetActiveTokenIdAsync(Guid id, Guid userId, DateTime now, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id
            FROM RefreshTokens
            WHERE Id = @Id
              AND UserId = @UserId
              AND ExpiresAt > @Now
            LIMIT 1
            """;

        return await db.QuerySingleOrDefaultAsync<Guid?>(
            sql,
            new { Id = id, UserId = userId, Now = now },
            transaction: tx());
    }

    public async Task<IReadOnlyList<UserSessionRecord>> GetActiveSessionsAsync(Guid userId, DateTime now, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, UserAgent, IpAddress, CreatedAt, LastSeenAt, ExpiresAt
            FROM RefreshTokens
            WHERE UserId = @UserId
              AND ExpiresAt > @Now
            ORDER BY LastSeenAt DESC, CreatedAt DESC
            """;

        var rows = await db.QueryAsync<UserSessionRecord>(
            sql,
            new { UserId = userId, Now = now },
            transaction: tx());
        return rows.AsList();
    }

    public Task<int> TouchAsync(Guid id, DateTime lastSeenAt, string? userAgent, string? ipAddress, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE RefreshTokens
            SET LastSeenAt = @LastSeenAt,
                UserAgent = @UserAgent,
                IpAddress = @IpAddress
            WHERE Id = @Id
              AND ExpiresAt > @LastSeenAt
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                LastSeenAt = lastSeenAt,
                UserAgent = userAgent,
                IpAddress = ipAddress,
            },
            transaction: tx());
    }

    public Task<int> DeleteOwnedSessionAsync(Guid sessionId, Guid userId, Guid? currentSessionId, CancellationToken cancellationToken)
    {
        const string sql = """
            DELETE FROM RefreshTokens
            WHERE Id = @SessionId
              AND UserId = @UserId
              AND (@CurrentSessionId IS NULL OR Id <> @CurrentSessionId)
            """;

        return db.ExecuteAsync(
            sql,
            new { SessionId = sessionId, UserId = userId, CurrentSessionId = currentSessionId },
            transaction: tx());
    }

    public Task<int> DeleteOtherTokensAsync(Guid userId, Guid keepTokenId, CancellationToken cancellationToken)
    {
        const string sql = """
            DELETE FROM RefreshTokens
            WHERE UserId = @UserId
              AND Id <> @KeepTokenId
            """;

        return db.ExecuteAsync(sql, new { UserId = userId, KeepTokenId = keepTokenId }, transaction: tx());
    }

    public Task<int> DeleteAllTokensAsync(Guid userId, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM RefreshTokens WHERE UserId = @UserId";
        return db.ExecuteAsync(sql, new { UserId = userId }, transaction: tx());
    }

    public Task<int> DeleteOldestTokensAsync(Guid userId, int tokensToRemoveCount, CancellationToken cancellationToken)
    {
        const string sql = 
            """
            DELETE FROM RefreshTokens
            WHERE Id = ANY(ARRAY(
                SELECT Id
                FROM RefreshTokens
                WHERE UserId = @UserId
                ORDER BY CreatedAt ASC
                LIMIT @Limit
            ))
            """;

        return db.ExecuteAsync(sql, new { UserId = userId, Limit = tokensToRemoveCount }, transaction: tx());
    }

    public Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM RefreshTokens WHERE Id = @Id";
        return db.ExecuteAsync(sql, new { Id = id }, tx());
    }

    public Task<int> DeleteExpiredAsync(DateTime now, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM RefreshTokens WHERE ExpiresAt <= @Now";
        return db.ExecuteAsync(sql, new { Now = now }, tx());
    }
}
