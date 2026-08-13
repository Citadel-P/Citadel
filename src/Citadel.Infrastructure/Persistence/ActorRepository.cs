using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class ActorRepository(IDbConnection db, Func<IDbTransaction> tx) : IActorRepository
{
    public Task AcquireRunAsLockAsync(Guid actorId, CancellationToken cancellationToken)
        => db.ExecuteAsync(
            "SELECT pg_advisory_xact_lock(hashtextextended(@LockKey, 0))",
            new { LockKey = $"citadel-run-as:{actorId:N}", cancellationToken },
            transaction: tx());

    public Task<int> AddAsync(Actor actor, CancellationToken cancellationToken)
    {
        const string sql = "INSERT INTO Actors (Id, Type, IsEnabled) VALUES (@Id, @Type, @IsEnabled)";
        return db.ExecuteAsync(sql, new
        {
            Id = actor.Id,
            Type = actor.Type == ActorType.ServiceAccount
                ? nameof(ActorType.ServiceAccount)
                : EnumFormatter<ActorType>.GetValue(actor.Type),
            actor.IsEnabled,
            cancellationToken
        }, transaction: tx());
    }

    public async Task<Actor?> GetById(Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                Actors.Id,
                Actors.Type,
                COALESCE(Users.Name, ServiceAccounts.Name, Teams.Name, CASE WHEN Actors.Type = 'System' THEN 'System' END) AS Name,
                Actors.IsEnabled AS IsEnabled
            FROM Actors
            LEFT JOIN Users ON Users.ActorId = Actors.Id
            LEFT JOIN ServiceAccounts ON ServiceAccounts.ActorId = Actors.Id
            LEFT JOIN Teams ON Teams.ActorId = Actors.Id
            WHERE Actors.Id = @Id
            LIMIT 1
            """;
        var result = await db.QuerySingleOrDefaultAsync<ActorDto>(sql, new { Id = id, cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<string[]?> GetRoleNamesAsync(Guid actorId, CancellationToken cancellationToken)
    {
        const string existsSql = "SELECT EXISTS (SELECT 1 FROM Actors WHERE Id = @ActorId AND IsEnabled)";
        if (!await db.ExecuteScalarAsync<bool>(existsSql, new { ActorId = actorId, cancellationToken }, transaction: tx()))
            return null;

        const string sql = """
            SELECT DISTINCT r.Name
            FROM ActorRoles ar
            JOIN Roles r ON r.Id = ar.RoleId
            WHERE ar.ActorId = @ActorId
            UNION
            SELECT DISTINCT r.Name
            FROM ActorTeamMemberships membership
            JOIN Teams t ON t.Id = membership.TeamId
            JOIN Actors teamActor ON teamActor.Id = t.ActorId AND teamActor.IsEnabled
            JOIN ActorRoles ar ON ar.ActorId = t.ActorId
            JOIN Roles r ON r.Id = ar.RoleId
            WHERE membership.MemberActorId = @ActorId
            """;
        var roles = await db.QueryAsync<string>(sql, new { ActorId = actorId, cancellationToken }, transaction: tx());
        return [.. roles];
    }

    public Task<RunAsActorInfo?> GetRunAsInfoAsync(Guid actorId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                a.Id AS ActorId,
                COALESCE(u.Id, sa.Id) AS PrincipalId,
                a.Type,
                COALESCE(u.Name, sa.Name) AS Name,
                a.IsEnabled,
                sa.ArchivedAtUtc IS NOT NULL AS IsArchived,
                ARRAY(
                    SELECT DISTINCT roleName
                    FROM (
                        SELECT r.Name AS roleName
                        FROM ActorRoles ar
                        JOIN Roles r ON r.Id = ar.RoleId
                        WHERE ar.ActorId = a.Id
                        UNION
                        SELECT r.Name AS roleName
                        FROM ActorTeamMemberships membership
                        JOIN Teams t ON t.Id = membership.TeamId
                        JOIN Actors teamActor ON teamActor.Id = t.ActorId AND teamActor.IsEnabled
                        JOIN ActorRoles ar ON ar.ActorId = t.ActorId
                        JOIN Roles r ON r.Id = ar.RoleId
                        WHERE membership.MemberActorId = a.Id
                    ) effectiveRoles
                ) AS Roles
            FROM Actors a
            LEFT JOIN Users u ON u.ActorId = a.Id
            LEFT JOIN ServiceAccounts sa ON sa.ActorId = a.Id
            WHERE a.Id = @ActorId
              AND ((a.Type = 'User' AND u.Id IS NOT NULL)
                OR (a.Type IN ('Service', 'ServiceAccount') AND sa.Id IS NOT NULL))
            LIMIT 1
            """;
        return db.QuerySingleOrDefaultAsync<RunAsActorInfo>(
            sql,
            new { ActorId = actorId, cancellationToken },
            transaction: tx());
    }

    public Task<bool> HasCustomAccessConfigurationAsync(
        IEnumerable<Guid> actorIds,
        CancellationToken cancellationToken)
    {
        var ids = actorIds as Guid[] ?? [.. actorIds];
        if (ids.Length == 0)
            return Task.FromResult(false);

        const string sql = """
            SELECT
                EXISTS (
                    SELECT 1
                    FROM ActorRoles ar
                    JOIN Roles r ON r.Id = ar.RoleId
                    WHERE ar.ActorId = ANY(@ActorIds)
                      AND r.RoleType = @CustomRoleType
                )
                OR EXISTS (
                    SELECT 1
                    FROM ResourceAccesses ra
                    WHERE ra.ActorId = ANY(@ActorIds)
                )
            """;
        return db.ExecuteScalarAsync<bool>(
            sql,
            new
            {
                ActorIds = ids,
                CustomRoleType = EnumFormatter<RoleType>.GetValue(RoleType.Custom),
                cancellationToken
            },
            transaction: tx());
    }

    public Task<int> UpdateAsync(Actor actor, CancellationToken cancellationToken)
    {
        const string sql = "UPDATE Actors SET IsEnabled = @IsEnabled WHERE Id = @Id";
        return db.ExecuteAsync(sql, new { Id = actor.Id, IsEnabled = actor.IsEnabled, cancellationToken }, transaction: tx());
    }
}
