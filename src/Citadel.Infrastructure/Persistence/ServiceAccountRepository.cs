using System.Data;
using Dapper;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Contracts.Resources;
using Domain.Entities.Identity;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.Models;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;

namespace Infrastructure.Persistence;

internal sealed class ServiceAccountRepository(IDbConnection db, Func<IDbTransaction> tx) : IServiceAccountRepository
{
    private const string AggregateJoins = """
        LEFT JOIN LATERAL (
            SELECT JSONB_AGG(
                JSONB_BUILD_OBJECT('id', t.Id, 'name', t.Name)
                ORDER BY t.Name, t.Id
            )::text AS Teams
            FROM ActorTeamMemberships membership
            JOIN Teams t ON t.Id = membership.TeamId
            WHERE membership.MemberActorId = sa.ActorId
        ) teams ON TRUE
        LEFT JOIN LATERAL (
            SELECT JSONB_AGG(
                JSONB_BUILD_OBJECT('id', r.Id, 'name', r.Name)
                ORDER BY r.Name, r.Id
            )::text AS Roles
            FROM ActorRoles ar
            JOIN Roles r ON r.Id = ar.RoleId
            WHERE ar.ActorId = sa.ActorId
        ) roles ON TRUE
        LEFT JOIN LATERAL (
            SELECT
                COUNT(*) FILTER (
                    WHERE token.RevokedAtUtc IS NULL
                      AND (token.ExpiresAtUtc IS NULL OR token.ExpiresAtUtc > CURRENT_TIMESTAMP)
                )::int AS ActiveTokenCount,
                MAX(token.LastUsedAtUtc) AS LastUsedAtUtc
            FROM ServiceAccountTokens token
            WHERE token.ServiceAccountId = sa.Id
        ) tokens ON TRUE
        """;

    public async Task<IReadOnlyList<ResourceInfo>> GetRunAsCandidatesAsync(
        Guid actorId,
        bool isAdministrator,
        CancellationToken cancellationToken)
    {
        const string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + """
            SELECT sa.ActorId AS Id, sa.Name, 'Service Accounts' AS "Group"
            FROM ServiceAccounts sa
            JOIN Actors actor ON actor.Id = sa.ActorId
            WHERE actor.IsEnabled
              AND sa.ArchivedAtUtc IS NULL
              AND (
                    @IsAdministrator
                    OR EXISTS (SELECT 1 FROM GlobalAccess)
                    OR EXISTS (
                        SELECT 1
                        FROM ResourceAccesses ra
                        JOIN ActorScope actorScope ON actorScope.ActorId = ra.ActorId
                        WHERE ra.ResourceType = @ResourceType
                          AND ra.ResourceId = sa.Id
                          AND (ra.PermissionLevel & @GrantedPermissionMask) <> 0
                          AND (ra.SpecificPermissions & @SpecificPermission) = @SpecificPermission
                    )
              )
            ORDER BY sa.Name, sa.Id
            """;
        var rows = await db.QueryAsync<ResourceInfo>(
            sql,
            new
            {
                UserId = actorId,
                IsAdministrator = isAdministrator,
                ResourceType = (int)ResourceType.ServiceAccount,
                GrantedPermissionMask = PermissionHelpers.GetGrantedPermissionMask(PermissionLevel.Read),
                SpecificPermission = (int)SpecificPermission.Use,
                cancellationToken,
            },
            transaction: tx());
        return [.. rows];
    }

    public async Task<ServiceAccount?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM ServiceAccounts WHERE Id = @Id";
        var row = await db.QuerySingleOrDefaultAsync<ServiceAccountDto>(
            sql,
            new { Id = id, cancellationToken },
            transaction: tx());
        return row?.ToDomain();
    }

    public async Task<ServiceAccountDetails?> GetDetailsAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = $$"""
            SELECT
                sa.Id,
                sa.Name,
                sa.Description,
                sa.ActorId,
                actor.IsEnabled,
                sa.CreatedAt,
                sa.CreatedByActorId,
                sa.UpdatedAt,
                sa.ArchivedAtUtc,
                COALESCE(tokens.ActiveTokenCount, 0) AS ActiveTokenCount,
                tokens.LastUsedAtUtc,
                COALESCE(teams.Teams, '[]') AS Teams,
                COALESCE(roles.Roles, '[]') AS Roles
            FROM ServiceAccounts sa
            JOIN Actors actor ON actor.Id = sa.ActorId
            {{AggregateJoins}}
            WHERE sa.Id = @Id
            """;

        var row = await db.QuerySingleOrDefaultAsync<ServiceAccountDetailsDto>(
            sql,
            new { Id = id, cancellationToken },
            transaction: tx());
        return row?.ToDetails();
    }

    public async Task<PagedResult<ServiceAccountDetails>> GetPagedAsync(
        int page,
        int pageSize,
        string? name,
        bool includeArchived,
        CancellationToken cancellationToken)
    {
        name = string.IsNullOrWhiteSpace(name) ? null : name.Trim();
        var offset = (page - 1) * pageSize;

        const string countSql = """
            SELECT COUNT(*)
            FROM ServiceAccounts sa
            WHERE (@IncludeArchived OR sa.ArchivedAtUtc IS NULL)
              AND (@Name IS NULL OR sa.Name ILIKE '%' || @Name || '%')
            """;

        const string selectSql = $$"""
            SELECT
                sa.Id,
                sa.Name,
                sa.Description,
                sa.ActorId,
                actor.IsEnabled,
                sa.CreatedAt,
                sa.CreatedByActorId,
                sa.UpdatedAt,
                sa.ArchivedAtUtc,
                COALESCE(tokens.ActiveTokenCount, 0) AS ActiveTokenCount,
                tokens.LastUsedAtUtc,
                COALESCE(teams.Teams, '[]') AS Teams,
                COALESCE(roles.Roles, '[]') AS Roles
            FROM ServiceAccounts sa
            JOIN Actors actor ON actor.Id = sa.ActorId
            {{AggregateJoins}}
            WHERE (@IncludeArchived OR sa.ArchivedAtUtc IS NULL)
              AND (@Name IS NULL OR sa.Name ILIKE '%' || @Name || '%')
            ORDER BY sa.Name, sa.Id
            LIMIT @PageSize OFFSET @Offset
            """;

        var parameters = new
        {
            Name = name,
            IncludeArchived = includeArchived,
            PageSize = pageSize,
            Offset = offset,
            cancellationToken,
        };
        var totalCount = await db.QuerySingleAsync<int>(countSql, parameters, transaction: tx());
        var rows = await db.QueryAsync<ServiceAccountDetailsDto>(selectSql, parameters, transaction: tx());
        return new PagedResult<ServiceAccountDetails>(rows.ToDetails(), totalCount, page, pageSize);
    }

    public async Task<PagedResult<ServiceAccountDetails>> GetAuthorizedPagedAsync(
        Guid actorId,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        int page,
        int pageSize,
        string? name,
        bool includeArchived,
        CancellationToken cancellationToken)
    {
        name = string.IsNullOrWhiteSpace(name) ? null : name.Trim();
        var offset = (page - 1) * pageSize;
        const string accessPredicate = """
            (
                EXISTS (SELECT 1 FROM GlobalAccess)
                OR EXISTS (
                    SELECT 1
                    FROM ResourceAccesses ra
                    JOIN ActorScope actorScope ON actorScope.ActorId = ra.ActorId
                    WHERE ra.ResourceType = @ResourceType
                      AND ra.ResourceId = sa.Id
                      AND (ra.PermissionLevel & @GrantedPermissionMask) <> 0
                      AND (@SpecificPermission = 0 OR (ra.SpecificPermissions & @SpecificPermission) = @SpecificPermission)
                )
            )
            """;
        const string countSql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + """
            SELECT COUNT(*)
            FROM ServiceAccounts sa
            WHERE (@IncludeArchived OR sa.ArchivedAtUtc IS NULL)
              AND (@Name IS NULL OR sa.Name ILIKE '%' || @Name || '%')
              AND
            """ + accessPredicate;
        const string selectSql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + $$"""
            SELECT
                sa.Id,
                sa.Name,
                sa.Description,
                sa.ActorId,
                actor.IsEnabled,
                sa.CreatedAt,
                sa.CreatedByActorId,
                sa.UpdatedAt,
                sa.ArchivedAtUtc,
                COALESCE(tokens.ActiveTokenCount, 0) AS ActiveTokenCount,
                tokens.LastUsedAtUtc,
                COALESCE(teams.Teams, '[]') AS Teams,
                COALESCE(roles.Roles, '[]') AS Roles
            FROM ServiceAccounts sa
            JOIN Actors actor ON actor.Id = sa.ActorId
            {{AggregateJoins}}
            WHERE (@IncludeArchived OR sa.ArchivedAtUtc IS NULL)
              AND (@Name IS NULL OR sa.Name ILIKE '%' || @Name || '%')
              AND
            """ + accessPredicate + """
            ORDER BY sa.Name, sa.Id
            LIMIT @PageSize OFFSET @Offset
            """;
        var parameters = new
        {
            UserId = actorId,
            ResourceType = (int)ResourceType.ServiceAccount,
            GrantedPermissionMask = PermissionHelpers.GetGrantedPermissionMask(permissionLevel),
            SpecificPermission = (int)specificPermission,
            Name = name,
            IncludeArchived = includeArchived,
            PageSize = pageSize,
            Offset = offset,
            cancellationToken,
        };
        var totalCount = await db.QuerySingleAsync<int>(countSql, parameters, transaction: tx());
        var rows = await db.QueryAsync<ServiceAccountDetailsDto>(selectSql, parameters, transaction: tx());
        return new PagedResult<ServiceAccountDetails>(rows.ToDetails(), totalCount, page, pageSize);
    }

    public Task AcquireNameLockAsync(string name, CancellationToken cancellationToken)
        => db.ExecuteAsync(
            "SELECT pg_advisory_xact_lock(hashtextextended(@Name, 0))",
            new { Name = $"citadel-service-account:{name.Trim()}", cancellationToken },
            transaction: tx());

    public Task<bool> ExistsByNameAsync(string name, Guid? excludeId, CancellationToken cancellationToken)
        => db.QuerySingleAsync<bool>(
            """
            SELECT EXISTS (
                SELECT 1 FROM ServiceAccounts
                WHERE ArchivedAtUtc IS NULL
                  AND Name = @Name
                  AND (@ExcludeId IS NULL OR Id <> @ExcludeId)
            )
            """,
            new { Name = name.Trim(), ExcludeId = excludeId, cancellationToken },
            transaction: tx());

    public Task<int> AddAsync(ServiceAccount account, CancellationToken cancellationToken)
        => db.ExecuteAsync(
            """
            INSERT INTO ServiceAccounts
                (Id, Name, Description, ActorId, CreatedAt, CreatedByActorId, UpdatedAt, ArchivedAtUtc)
            VALUES
                (@Id, @Name, @Description, @ActorId, @CreatedAt, @CreatedByActorId, @UpdatedAt, @ArchivedAtUtc)
            """,
            new
            {
                account.Id,
                account.Name,
                account.Description,
                account.ActorId,
                account.CreatedAt,
                account.CreatedByActorId,
                account.UpdatedAt,
                account.ArchivedAtUtc,
                cancellationToken,
            },
            transaction: tx());

    public Task<int> UpdateAsync(ServiceAccount account, CancellationToken cancellationToken)
        => db.ExecuteAsync(
            """
            UPDATE ServiceAccounts
            SET Name = @Name,
                Description = @Description,
                UpdatedAt = @UpdatedAt,
                ArchivedAtUtc = @ArchivedAtUtc
            WHERE Id = @Id
            """,
            new
            {
                account.Id,
                account.Name,
                account.Description,
                account.UpdatedAt,
                account.ArchivedAtUtc,
                cancellationToken,
            },
            transaction: tx());

    public async Task<IReadOnlyList<Guid>> GetTeamIdsAsync(Guid actorId, CancellationToken cancellationToken)
    {
        var rows = await db.QueryAsync<Guid>(
            "SELECT TeamId FROM ActorTeamMemberships WHERE MemberActorId = @ActorId ORDER BY TeamId",
            new { ActorId = actorId, cancellationToken },
            transaction: tx());
        return rows.AsList();
    }

    public async Task<int> ReplaceTeamsAsync(Guid actorId, IEnumerable<Guid> teamIds, CancellationToken cancellationToken)
    {
        var transaction = tx();
        var affected = await db.ExecuteAsync(
            "DELETE FROM ActorTeamMemberships WHERE MemberActorId = @ActorId",
            new { ActorId = actorId, cancellationToken },
            transaction: transaction);

        foreach (var teamId in teamIds.Distinct())
        {
            affected += await db.ExecuteAsync(
                "INSERT INTO ActorTeamMemberships (MemberActorId, TeamId) VALUES (@ActorId, @TeamId)",
                new { ActorId = actorId, TeamId = teamId, cancellationToken },
                transaction: transaction);
        }

        return affected;
    }

    public async Task<PagedResult<ServiceAccountTokenDetails>> GetTokensAsync(
        Guid serviceAccountId,
        int page,
        int pageSize,
        CancellationToken cancellationToken)
    {
        var offset = (page - 1) * pageSize;
        var parameters = new { ServiceAccountId = serviceAccountId, PageSize = pageSize, Offset = offset, cancellationToken };
        var totalCount = await db.QuerySingleAsync<int>(
            "SELECT COUNT(*) FROM ServiceAccountTokens WHERE ServiceAccountId = @ServiceAccountId",
            parameters,
            transaction: tx());
        var rows = await db.QueryAsync<ServiceAccountTokenDto>(
            """
            SELECT
                token.Id,
                token.Name,
                token.ExpiresAtUtc,
                token.LastUsedAtUtc,
                token.RevokedAtUtc,
                token.RevokedByActorId,
                token.CreatedByActorId,
                COALESCE(creator.Name, 'Unknown') AS CreatedByName,
                token.CreatedAtUtc
            FROM ServiceAccountTokens token
            LEFT JOIN Users creator ON creator.ActorId = token.CreatedByActorId
            WHERE token.ServiceAccountId = @ServiceAccountId
            ORDER BY token.CreatedAtUtc DESC, token.Id DESC
            LIMIT @PageSize OFFSET @Offset
            """,
            parameters,
            transaction: tx());
        return new PagedResult<ServiceAccountTokenDetails>(rows.Select(x => x.ToDetails()), totalCount, page, pageSize);
    }

    public async Task<ServiceAccountCredentialInfo?> GetCredentialAsync(Guid credentialId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                token.Id AS CredentialId,
                token.ServiceAccountId,
                account.ActorId,
                account.Name,
                token.SecretHash,
                token.ExpiresAtUtc,
                token.RevokedAtUtc,
                actor.IsEnabled,
                account.ArchivedAtUtc
            FROM ServiceAccountTokens token
            JOIN ServiceAccounts account ON account.Id = token.ServiceAccountId
            JOIN Actors actor ON actor.Id = account.ActorId
            WHERE token.Id = @CredentialId
            """;
        var row = await db.QuerySingleOrDefaultAsync<ServiceAccountCredentialDto>(
            sql,
            new { CredentialId = credentialId, cancellationToken },
            transaction: tx());
        return row?.ToCredentialInfo();
    }

    public async Task<ServiceAccountTokenInsertResult> TryAddTokenAsync(
        ServiceAccountToken token,
        int maximumActiveTokens,
        DateTime now,
        CancellationToken cancellationToken)
    {
        var transaction = tx();
        var accountAvailable = await db.QuerySingleOrDefaultAsync<Guid?>(
            """
            SELECT account.Id
            FROM ServiceAccounts account
            JOIN Actors actor ON actor.Id = account.ActorId
            WHERE account.Id = @ServiceAccountId
              AND account.ArchivedAtUtc IS NULL
              AND actor.IsEnabled
            FOR UPDATE OF account, actor
            """,
            new { token.ServiceAccountId, cancellationToken },
            transaction: transaction);
        if (!accountAvailable.HasValue)
            return ServiceAccountTokenInsertResult.AccountUnavailable;

        var duplicateName = await db.QuerySingleAsync<bool>(
            "SELECT EXISTS (SELECT 1 FROM ServiceAccountTokens WHERE ServiceAccountId = @ServiceAccountId AND Name = @Name)",
            new { token.ServiceAccountId, token.Name, cancellationToken },
            transaction: transaction);
        if (duplicateName)
            return ServiceAccountTokenInsertResult.DuplicateName;

        var activeCount = await db.QuerySingleAsync<int>(
            """
            SELECT COUNT(*)
            FROM ServiceAccountTokens
            WHERE ServiceAccountId = @ServiceAccountId
              AND RevokedAtUtc IS NULL
              AND (ExpiresAtUtc IS NULL OR ExpiresAtUtc > @Now)
            """,
            new { token.ServiceAccountId, Now = now, cancellationToken },
            transaction: transaction);
        if (activeCount >= maximumActiveTokens)
            return ServiceAccountTokenInsertResult.ActiveLimitReached;

        await db.ExecuteAsync(
            """
            INSERT INTO ServiceAccountTokens
                (Id, ServiceAccountId, Name, SecretHash, ExpiresAtUtc, LastUsedAtUtc, RevokedAtUtc,
                 RevokedByActorId, CreatedByActorId, CreatedAtUtc)
            VALUES
                (@Id, @ServiceAccountId, @Name, @SecretHash, @ExpiresAtUtc, @LastUsedAtUtc, @RevokedAtUtc,
                 @RevokedByActorId, @CreatedByActorId, @CreatedAtUtc)
            """,
            new
            {
                token.Id,
                token.ServiceAccountId,
                token.Name,
                token.SecretHash,
                token.ExpiresAtUtc,
                token.LastUsedAtUtc,
                token.RevokedAtUtc,
                token.RevokedByActorId,
                token.CreatedByActorId,
                token.CreatedAtUtc,
                cancellationToken,
            },
            transaction: transaction);
        return ServiceAccountTokenInsertResult.Created;
    }

    public Task<int> RevokeTokenAsync(
        Guid serviceAccountId,
        Guid tokenId,
        Guid revokedByActorId,
        DateTime revokedAtUtc,
        CancellationToken cancellationToken)
        => db.ExecuteAsync(
            """
            UPDATE ServiceAccountTokens
            SET RevokedAtUtc = @RevokedAtUtc, RevokedByActorId = @RevokedByActorId
            WHERE Id = @TokenId AND ServiceAccountId = @ServiceAccountId AND RevokedAtUtc IS NULL
            """,
            new { ServiceAccountId = serviceAccountId, TokenId = tokenId, RevokedByActorId = revokedByActorId, RevokedAtUtc = revokedAtUtc, cancellationToken },
            transaction: tx());

    public Task<int> RevokeAllTokensAsync(
        Guid serviceAccountId,
        Guid revokedByActorId,
        DateTime revokedAtUtc,
        CancellationToken cancellationToken)
        => db.ExecuteAsync(
            """
            UPDATE ServiceAccountTokens
            SET RevokedAtUtc = @RevokedAtUtc, RevokedByActorId = @RevokedByActorId
            WHERE ServiceAccountId = @ServiceAccountId AND RevokedAtUtc IS NULL
            """,
            new { ServiceAccountId = serviceAccountId, RevokedByActorId = revokedByActorId, RevokedAtUtc = revokedAtUtc, cancellationToken },
            transaction: tx());

    public Task<int> UpdateLastUsedAsync(Guid tokenId, DateTime usedAtUtc, CancellationToken cancellationToken)
        => db.ExecuteAsync(
            """
            UPDATE ServiceAccountTokens
            SET LastUsedAtUtc = @UsedAtUtc
            WHERE Id = @TokenId AND (LastUsedAtUtc IS NULL OR LastUsedAtUtc < @UsedAtUtc)
            """,
            new { TokenId = tokenId, UsedAtUtc = usedAtUtc, cancellationToken },
            transaction: tx());

    public async Task<IReadOnlyList<RunAsActorUsage>> GetRunAsUsagesAsync(
        Guid actorId,
        CancellationToken cancellationToken)
    {
        var usages = await db.QueryAsync<RunAsActorUsage>(
            """
            SELECT Id, Name, @AutomationActionType AS ResourceType, Enabled AS IsActive
            FROM Actions
            WHERE RunAsActorId = @ActorId
            UNION ALL
            SELECT Id, Name, @BackupPolicyType AS ResourceType, (Enabled AND ArchivedAt IS NULL) AS IsActive
            FROM BackupPolicies
            WHERE RunAsActorId = @ActorId
            ORDER BY ResourceType, Name, Id
            """,
            new
            {
                ActorId = actorId,
                AutomationActionType = (int)ResourceType.AutomationAction,
                BackupPolicyType = (int)ResourceType.BackupPolicy,
                cancellationToken,
            },
            transaction: tx());
        return [.. usages];
    }
}
