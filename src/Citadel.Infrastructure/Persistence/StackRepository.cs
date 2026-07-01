using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities;
using Domain.Entities.Stacks;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using System.Text.Json;
using Domain.Contracts.Resources.ResourceBindings;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class StackRepository(IDbConnection db, Func<IDbTransaction> tx) : IStackRepository
{
    private const string InfoSelect = """
    SELECT
        s.Id,
        s.CurrentStackReleaseId,
        s.Name,
        s.Description,
        s.StackSource,
        s.StackUpdateState,
        s.DriftPolicy,
        s.CreatedAt,
        s.CreatedByActorId,
        s.ControlState,
        s.ControlStartedAt,
        s.RowVersion,
        s.ControlTriggeredBy,
        sr.Id AS CurrentRelease_Id,
        sr.StackId AS CurrentRelease_StackId,
        sr.PlatformId AS CurrentRelease_PlatformId,
        sr.Status AS CurrentRelease_Status,
        sr.Version AS CurrentRelease_Version,
        sr.Source AS CurrentRelease_Source,
        sr.CreatedAt AS CurrentRelease_CreatedAt,
        sr.CreatedByActorId AS CurrentRelease_CreatedByActorId,
        p.Name AS Platform_Name,
        p.Status AS Platform_Status
    FROM Stacks s
    LEFT JOIN StackReleases sr
        ON s.CurrentStackReleaseId = sr.Id
    LEFT JOIN Platforms p
        ON sr.PlatformId = p.Id
    """;

    private const string ReleaseBaseSelect = """
    SELECT
        sr.Id,
        sr.StackId,
        sr.PlatformId,
        sr.Status,
        sr.Version,
        sr.Spec,
        sr.Source,
        sr.ResourceBindings,
        sr.CreatedAt,
        sr.CreatedByActorId,
        p.Name AS Platform_Name,
        p.Status AS Platform_Status,
        u.Name AS Actor_Name,
        ac.Type AS Actor_Type
    FROM StackReleases sr
    LEFT JOIN Platforms p
        ON sr.PlatformId = p.Id
    LEFT JOIN Actors ac
        ON sr.CreatedByActorId = ac.Id
    LEFT JOIN Users u
        ON sr.CreatedByActorId = u.ActorId
    """;

    public async Task<Stack?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                s.Id,
                s.CurrentStackReleaseId,
                s.Name,
                s.Description,
                s.StackSource,
                s.StackUpdateState,
                s.DriftPolicy,
                s.CreatedAt,
                s.CreatedByActorId,
                s.ControlState,
                s.ControlStartedAt,
                s.RowVersion,
                s.ControlTriggeredBy,
                sr.Id AS CurrentRelease_Id,
                sr.StackId AS CurrentRelease_StackId,
                sr.PlatformId AS CurrentRelease_PlatformId,
                sr.Status AS CurrentRelease_Status,
                sr.Version AS CurrentRelease_Version,
                sr.Spec AS CurrentRelease_Spec,
                sr.Source AS CurrentRelease_Source,
                sr.ResourceBindings AS CurrentRelease_ResourceBindings,
                sr.CreatedAt AS CurrentRelease_CreatedAt,
                sr.CreatedByActorId AS CurrentRelease_CreatedByActorId,
                p.Name AS Platform_Name,
                p.Status AS Platform_Status,
                ei.Info AS ActivityEvent_ActivityEventInfo,
                ei.EventType AS ActivityEvent_EventType,
                ei.Status AS ActivityEvent_Status,
                ei.Id AS ActivityEvent_Id,
                ei.CreatedAt AS ActivityEvent_CreatedAt
            FROM Stacks s
            LEFT JOIN StackReleases sr
                ON s.CurrentStackReleaseId = sr.Id
            LEFT JOIN Platforms p
                ON sr.PlatformId = p.Id
            LEFT JOIN LATERAL (
                SELECT e.Id, e.EventType, e.Status, e.Info, e.CreatedAt
                FROM ActivityEvents e
                WHERE e.ResourceId = s.Id 
                AND e.ResourceType = 'Stack'
                AND e.EventType NOT IN ('StackUpdated', 'StackRenamed')
                ORDER BY e.CreatedAt DESC
                LIMIT 1
            ) ei ON TRUE
            WHERE s.Id = @Id LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<StackDto>(sql, new { Id = id, cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<Stack?> GetInfoAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = InfoSelect + " " + "WHERE s.Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<StackDto>(sql, new { Id = id, cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<StackDriftStack?> GetDriftStackAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                s.Id,
                s.CurrentStackReleaseId,
                s.Name,
                s.StackSource,
                s.ControlState,
                s.DriftPolicy,
                sr.PlatformId,
                sr.Status,
                sr.Spec,
                sr.Source,
                p.Name AS PlatformName
            FROM Stacks s
            INNER JOIN StackReleases sr
                ON s.CurrentStackReleaseId = sr.Id
            LEFT JOIN Platforms p
                ON sr.PlatformId = p.Id
            WHERE s.Id = @Id
            LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<StackDriftStackDto>(sql, new { Id = id }, transaction: tx());

        return result?.ToDriftStack();
    }

    public async Task<IEnumerable<StackDriftStack>> GetDriftMonitorStacksAsync(CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                s.Id,
                s.CurrentStackReleaseId,
                s.Name,
                s.StackSource,
                s.ControlState,
                s.DriftPolicy,
                sr.PlatformId,
                sr.Status,
                sr.Spec,
                sr.Source,
                p.Name AS PlatformName
            FROM Stacks s
            INNER JOIN StackReleases sr
                ON s.CurrentStackReleaseId = sr.Id
            LEFT JOIN Platforms p
                ON sr.PlatformId = p.Id
            WHERE COALESCE(s.DriftPolicy ->> 'Mode', @DetectOnlyMode) <> @DisabledMode
              AND sr.Status = ANY(@MonitorStatuses)
            ORDER BY s.Id
            """;

        var result = await db.QueryAsync<StackDriftStackDto>(sql, new
        {
            DetectOnlyMode = EnumFormatter<StackDriftMode>.GetValue(StackDriftMode.DetectOnly),
            DisabledMode = EnumFormatter<StackDriftMode>.GetValue(StackDriftMode.Disabled),
            MonitorStatuses = new[]
            {
                EnumFormatter<StackReleaseStatus>.GetValue(StackReleaseStatus.Healthy),
                EnumFormatter<StackReleaseStatus>.GetValue(StackReleaseStatus.Degraded)
            }
        }, transaction: tx());

        return [.. result.Select(x => x.ToDriftStack())];
    }

    public async Task<IEnumerable<Stack>> GetAllAsync(CancellationToken cancellationToken)
    {
        const string sql = InfoSelect + " " + "ORDER BY s.CreatedAt DESC, s.Name ASC";
        var result = await db.QueryAsync<StackDto>(sql, transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<Stack>> GetInfoAsync(CancellationToken cancellationToken)
    {
        const string sql = InfoSelect + " " + "ORDER BY s.CreatedAt DESC, s.Name ASC";
        var result = await db.QueryAsync<StackDto>(sql, transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<Container>> GetContainersAsync(Guid stackId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Containers WHERE StackId = @StackId";
        var result = await db.QueryAsync<ContainerDto>(sql, new { StackId = stackId, cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public Task<IEnumerable<string>> GetContainerIdsAsync(Guid stackId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT DockerContainerId FROM Containers WHERE StackId = @StackId";
        var result = db.QueryAsync<string>(sql, new { StackId = stackId, cancellationToken }, transaction: tx());
        return result;
    }

    public async Task<IEnumerable<Stack>> GetAuthorizedInfoAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken)
    {
        const string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + InfoSelect + " WHERE "
                        + AuthorizationSql.ResourcePredicatePrefix + "s.Id" + AuthorizationSql.ResourcePredicateSuffix
                        + " ORDER BY s.CreatedAt DESC, s.Name ASC";

        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel);
        var result = await db.QueryAsync<StackDto>(sql, new
        {
            UserId = userId,
            ResourceType = (int)resourceType,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)specificPermission,
            cancellationToken
        }, transaction: tx());

        return result.ToDomain();
    }

    public Task<bool> CanAccessAsync(Guid userId, Guid stackId, CancellationToken cancellationToken)
    {
        const string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}} 
            SELECT EXISTS (SELECT 1 FROM Stacks s WHERE s.Id = @StackId AND 
            {{AuthorizationSql.ResourcePredicatePrefix}}s.Id{{AuthorizationSql.ResourcePredicateSuffix}})
        """;

        return db.ExecuteScalarAsync<bool>(sql, new
        {
            UserId = userId,
            StackId = stackId,
            ResourceType = (int)ResourceType.Stack,
            GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(PermissionLevel.Read),
            SpecificPermission = (int)SpecificPermission.None,
            cancellationToken
        }, transaction: tx());
    }

    public async Task<IEnumerable<Stack>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = InfoSelect + " " + "WHERE s.Id = ANY(@Ids) ORDER BY s.CreatedAt DESC, s.Name ASC";
        var idArray = ids as Guid[] ?? [.. ids];
        var result = await db.QueryAsync<StackDto>(sql, new { Ids = idArray, cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public Task<IEnumerable<ResourceInfo>> GetPlatformLookupAsync(Guid stackId, Guid userId, CancellationToken cancellationToken)
    {
        const string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}}
            SELECT lookup.Id, lookup.Name
            FROM (
                SELECT p.Id, p.Name
                FROM Platforms p
                WHERE {{AuthorizationSql.ResourcePredicatePrefix}}p.Id{{AuthorizationSql.ResourcePredicateSuffix}}

                UNION

                SELECT p.Id, p.Name
                FROM Stacks s
                JOIN StackReleases sr ON sr.Id = s.CurrentStackReleaseId
                JOIN Platforms p ON p.Id = sr.PlatformId
                WHERE s.Id = @StackId
            ) lookup
            ORDER BY lookup.Name
         """;

        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(PermissionLevel.Read);

        return db.QueryAsync<ResourceInfo>(sql, new
        {
            UserId = userId,
            ResourceType = (int)ResourceType.Platform,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)SpecificPermission.None,
            StackId = stackId,
            cancellationToken
        }, transaction: tx());
    }

    public Task<IEnumerable<ResourceInfo>> GetRegistryLookupAsync(Guid stackId, Guid userId, CancellationToken cancellationToken)
    {
        const string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}}
            SELECT lookup.Id, lookup.Name
            FROM (
                SELECT r.Id, r.Name
                FROM Registries r
                WHERE {{AuthorizationSql.ResourcePredicatePrefix}}r.Id{{AuthorizationSql.ResourcePredicateSuffix}}

                UNION

                SELECT r.Id, r.Name
                FROM Stacks s
                JOIN StackReleases sr ON sr.Id = s.CurrentStackReleaseId
                JOIN Registries r ON r.Id = CAST(sr.Spec ->> 'RegistryId' AS uuid)
                WHERE s.Id = @StackId
            ) lookup
            ORDER BY lookup.Name
         """;

        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(PermissionLevel.Read);

        return db.QueryAsync<ResourceInfo>(sql, new
        {
            UserId = userId,
            ResourceType = (int)ResourceType.Registry,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)SpecificPermission.None,
            StackId = stackId,
            cancellationToken
        }, transaction: tx());
    }

    public Task<IEnumerable<ResourceInfo>> GetGitRepositoryLookupAsync(Guid stackId, Guid userId, CancellationToken cancellationToken)
    {
        const string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}}
            SELECT lookup.Id, lookup.Name
            FROM (
                SELECT gr.Id, gr.Name
                FROM GitRepositories gr
                WHERE {{AuthorizationSql.ResourcePredicatePrefix}}gr.Id{{AuthorizationSql.ResourcePredicateSuffix}}

                UNION

                SELECT gr.Id, gr.Name
                FROM Stacks s
                JOIN StackReleases sr ON sr.Id = s.CurrentStackReleaseId
                JOIN GitRepositories gr ON gr.Id = CAST(sr.Spec ->> 'GitRepoId' AS uuid)
                WHERE s.Id = @StackId
                  AND s.StackSource = @StackSource
            ) lookup
            ORDER BY lookup.Name
         """;

        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(PermissionLevel.Read);

        return db.QueryAsync<ResourceInfo>(sql, new
        {
            UserId = userId,
            ResourceType = (int)ResourceType.GitRepository,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)SpecificPermission.None,
            StackId = stackId,
            StackSource = EnumFormatter<StackSource>.GetValue(StackSource.Git),
            cancellationToken
        }, transaction: tx());
    }

    public async Task<IEnumerable<StackRelease>> GetReleasesByStackIdAsync(Guid stackId, CancellationToken cancellationToken)
    {
        const string sql = ReleaseBaseSelect + " " + "WHERE sr.StackId = @StackId ORDER BY sr.CreatedAt DESC, sr.Version DESC";
        var result = await db.QueryAsync<StackReleaseDto>(sql, new { StackId = stackId, cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public Task<bool> ExistsAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Stacks WHERE Name = @Name)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, cancellationToken }, transaction: tx());
    }

    public Task<bool> ExistsAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Stacks WHERE Id = @Id)";
        return db.ExecuteScalarAsync<bool>(sql, new { Id = id, cancellationToken }, transaction: tx());
    }

    public Task<bool> ExistsAsync(Guid id, string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Stacks WHERE Name = @Name AND Id != @Id)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, Id = id, cancellationToken }, transaction: tx());
    }

    public Task<int> AddAsync(Stack stack, CancellationToken cancellationToken)
    {
        const string stackSql = """
            INSERT INTO Stacks (
                Id, CurrentStackReleaseId, Name, Description, StackSource, StackUpdateState,
                DriftPolicy, CreatedAt, CreatedByActorId, ControlState, ControlStartedAt, RowVersion, ControlTriggeredBy
            ) VALUES (
                @Id, @CurrentStackReleaseId, @Name, @Description, @StackSource, @StackUpdateState::json,
                @DriftPolicy::json, @CreatedAt, @CreatedByActorId, @ControlState, @ControlStartedAt, @RowVersion, @ControlTriggeredBy
            )
        """;

        const string stackReleaseSql = """
            INSERT INTO StackReleases (
                Id, StackId, PlatformId, Status, Version, Spec, Source, ResourceBindings, CreatedAt, CreatedByActorId
            ) VALUES (
                @ReleaseId, @ReleaseStackId, @ReleasePlatformId, @ReleaseStatus, @ReleaseVersion, @ReleaseSpec::json, @ReleaseSource::json, @ReleaseResourceBindings::json, @ReleaseCreatedAt, @ReleaseCreatedByActorId
            )
        """;

        var currentStackRelease = stack.CurrentStackRelease ?? throw new InvalidOperationException("Stack must have a current stack release.");

        return db.ExecuteAsync(stackSql + ";\n" + stackReleaseSql, new
        {
            Id = stack.Id,
            CurrentStackReleaseId = stack.CurrentStackReleaseId,
            Name = stack.Name,
            Description = stack.Description,
            StackSource = EnumFormatter<StackSource>.GetValue(stack.StackSource),
            StackUpdateState = JsonSerializer.Serialize(stack.StackUpdateState, StackJsonContext.Default.StackUpdateState),
            DriftPolicy = JsonSerializer.Serialize(stack.DriftPolicy, StackJsonContext.Default.StackDriftPolicy),
            CreatedAt = stack.CreatedAt,
            CreatedByActorId = stack.CreatedByActorId,
            ControlState = EnumFormatter<ResourceControlState>.GetValue(stack.ControlState),
            ControlStartedAt = stack.ControlStartedAt,
            RowVersion = stack.RowVersion,
            ControlTriggeredBy = stack.ControlTriggeredBy,
            ReleaseId = currentStackRelease.Id,
            ReleaseStackId = currentStackRelease.StackId,
            ReleasePlatformId = currentStackRelease.PlatformId,
            ReleaseStatus = EnumFormatter<StackReleaseStatus>.GetValue(currentStackRelease.Status),
            ReleaseVersion = currentStackRelease.Version,
            ReleaseSpec = JsonSerializer.Serialize(currentStackRelease.Spec, StackJsonContext.Default.StackSpec),
            ReleaseSource = currentStackRelease.Source is null ? null : JsonSerializer.Serialize(currentStackRelease.Source, StackJsonContext.Default.StackReleaseSource),
            ReleaseResourceBindings = SerializeResourceBindings(currentStackRelease.ResourceBindings),
            ReleaseCreatedAt = currentStackRelease.CreatedAt,
            ReleaseCreatedByActorId = currentStackRelease.CreatedByActorId
        }, transaction: tx());
    }

    public Task<int> AddReleaseAsync(StackRelease release, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO StackReleases (
                Id, StackId, PlatformId, Status, Version, Spec, Source, ResourceBindings, CreatedAt, CreatedByActorId
            ) VALUES (
                @ReleaseId, @ReleaseStackId, @ReleasePlatformId, @ReleaseStatus, @ReleaseVersion, @ReleaseSpec::json, @ReleaseSource::json, @ReleaseResourceBindings::json, @ReleaseCreatedAt, @ReleaseCreatedByActorId
            )
        """;

        return db.ExecuteAsync(sql, new
        {
            ReleaseId = release.Id,
            ReleaseStackId = release.StackId,
            ReleasePlatformId = release.PlatformId,
            ReleaseStatus = EnumFormatter<StackReleaseStatus>.GetValue(release.Status),
            ReleaseVersion = release.Version,
            ReleaseSpec = JsonSerializer.Serialize(release.Spec, StackJsonContext.Default.StackSpec),
            ReleaseSource = release.Source is null ? null : JsonSerializer.Serialize(release.Source, StackJsonContext.Default.StackReleaseSource),
            ReleaseResourceBindings = SerializeResourceBindings(release.ResourceBindings),
            ReleaseCreatedAt = release.CreatedAt,
            ReleaseCreatedByActorId = release.CreatedByActorId
        }, transaction: tx());
    }

    public Task<int> UpdateAsync(Stack stack, CancellationToken cancellationToken)
    {
        const string stackSql = """
            UPDATE Stacks
            SET Name = @Name,
                Description = @Description,
                CurrentStackReleaseId = @CurrentStackReleaseId,
                StackSource = @StackSource,
                StackUpdateState = @StackUpdateState::json,
                DriftPolicy = @DriftPolicy::json,
                ControlState = @ControlState,
                ControlStartedAt = @ControlStartedAt,
                ControlTriggeredBy = @ControlTriggeredBy
            WHERE Id = @Id
        """;

        const string stackReleaseSql = """
            INSERT INTO StackReleases (
                Id, StackId, PlatformId, Status, Version, Spec, Source, ResourceBindings, CreatedAt, CreatedByActorId
            )
            VALUES (@ReleaseId, @ReleaseStackId, @ReleasePlatformId, @ReleaseStatus, @ReleaseVersion, @ReleaseSpec::json, @ReleaseSource::json, @ReleaseResourceBindings::json, @ReleaseCreatedAt, @ReleaseCreatedByActorId)
            ON CONFLICT (Id) DO UPDATE
            SET PlatformId = EXCLUDED.PlatformId,
                Status = EXCLUDED.Status,
                Version = EXCLUDED.Version,
                Spec = EXCLUDED.Spec,
                Source = EXCLUDED.Source,
                ResourceBindings = EXCLUDED.ResourceBindings
        """;

        var currentStackRelease = stack.CurrentStackRelease ?? throw new InvalidOperationException("Stack must have a current stack release.");

        return db.ExecuteAsync(stackSql + ";\n" + stackReleaseSql, new
        {
            Id = stack.Id,
            Name = stack.Name,
            Description = stack.Description,
            CurrentStackReleaseId = stack.CurrentStackReleaseId,
            StackSource = EnumFormatter<StackSource>.GetValue(stack.StackSource),
            StackUpdateState = JsonSerializer.Serialize(stack.StackUpdateState, StackJsonContext.Default.StackUpdateState),
            DriftPolicy = JsonSerializer.Serialize(stack.DriftPolicy, StackJsonContext.Default.StackDriftPolicy),
            ControlState = EnumFormatter<ResourceControlState>.GetValue(stack.ControlState),
            ControlStartedAt = stack.ControlStartedAt,
            ControlTriggeredBy = stack.ControlTriggeredBy,
            ReleaseId = currentStackRelease.Id,
            ReleaseStackId = currentStackRelease.StackId,
            ReleasePlatformId = currentStackRelease.PlatformId,
            ReleaseStatus = EnumFormatter<StackReleaseStatus>.GetValue(currentStackRelease.Status),
            ReleaseVersion = currentStackRelease.Version,
            ReleaseSpec = JsonSerializer.Serialize(currentStackRelease.Spec, StackJsonContext.Default.StackSpec),
            ReleaseSource = currentStackRelease.Source is null ? null : JsonSerializer.Serialize(currentStackRelease.Source, StackJsonContext.Default.StackReleaseSource),
            ReleaseResourceBindings = SerializeResourceBindings(currentStackRelease.ResourceBindings),
            ReleaseCreatedAt = currentStackRelease.CreatedAt,
            ReleaseCreatedByActorId = currentStackRelease.CreatedByActorId
        }, transaction: tx());
    }

    public Task<int> UpdateReleaseStatusAsync(Guid releaseId, StackReleaseStatus status, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE StackReleases
            SET Status = @Status
            WHERE Id = @ReleaseId
        """;

        return db.ExecuteAsync(sql, new
        {
            ReleaseId = releaseId,
            Status = EnumFormatter<StackReleaseStatus>.GetValue(status),
            cancellationToken
        }, transaction: tx());
    }

    public Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM Stacks WHERE Id = ANY(@Ids)";
        var idArray = ids as Guid[] ?? [.. ids];

        return db.ExecuteAsync(
            sql,
            new { Ids = idArray, cancellationToken },
            transaction: tx());
    }

    public async Task<IEnumerable<Stack>> GetStuckStacksAsync(int timeout_s = 60, CancellationToken cancellationToken = default)
    {
        const string sql = InfoSelect + " " + """
            WHERE 
                s.ControlState = 'Processing'
                AND s.ControlStartedAt < @ControlStartedAt
            ORDER BY 
                s.ControlStartedAt ASC
            """;
        var result = await db.QueryAsync<StackDto>(sql, new
        {
            ControlStartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds() - timeout_s,
            cancellationToken
        }, transaction: tx());

        return result.ToDomain();
    }

    public Task<IEnumerable<GitStackBranchSubscription>> GetGitStackBranchSubscriptionsAsync(CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                s.Id AS StackId,
                CAST(sr.Spec ->> 'GitRepoId' AS uuid) AS GitRepositoryId,
                COALESCE(NULLIF(sr.Spec ->> 'Branch', ''), gr.DefaultBranch) AS Branch
            FROM Stacks s
            INNER JOIN StackReleases sr
                ON s.CurrentStackReleaseId = sr.Id
            INNER JOIN GitRepositories gr
                ON gr.Id = CAST(sr.Spec ->> 'GitRepoId' AS uuid)
            WHERE s.StackSource = @StackSource
              AND COALESCE(sr.Spec ->> 'CommitSha', '') = ''
              AND COALESCE(NULLIF(sr.Spec ->> 'Branch', ''), gr.DefaultBranch) <> ''
            ORDER BY s.Id
            """;

        return db.QueryAsync<GitStackBranchSubscription>(
            sql,
            new
            {
                StackSource = EnumFormatter<StackSource>.GetValue(StackSource.Git),
                cancellationToken
            },
            transaction: tx());
    }

    public async Task<IEnumerable<Stack>> GetBranchTrackingGitStacksAsync(Guid gitRepositoryId, string branch, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                s.Id,
                s.CurrentStackReleaseId,
                s.Name,
                s.Description,
                s.StackSource,
                s.StackUpdateState,
                s.DriftPolicy,
                s.CreatedAt,
                s.CreatedByActorId,
                s.ControlState,
                s.ControlStartedAt,
                s.RowVersion,
                s.ControlTriggeredBy,
                sr.Id AS CurrentRelease_Id,
                sr.StackId AS CurrentRelease_StackId,
                sr.PlatformId AS CurrentRelease_PlatformId,
                sr.Status AS CurrentRelease_Status,
                sr.Version AS CurrentRelease_Version,
                sr.Spec AS CurrentRelease_Spec,
                sr.Source AS CurrentRelease_Source,
                sr.ResourceBindings AS CurrentRelease_ResourceBindings,
                sr.CreatedAt AS CurrentRelease_CreatedAt,
                sr.CreatedByActorId AS CurrentRelease_CreatedByActorId,
                p.Name AS Platform_Name,
                p.Status AS Platform_Status
            FROM Stacks s
            INNER JOIN StackReleases sr
                ON s.CurrentStackReleaseId = sr.Id
            INNER JOIN GitRepositories gr
                ON gr.Id = CAST(sr.Spec ->> 'GitRepoId' AS uuid)
            LEFT JOIN Platforms p
                ON sr.PlatformId = p.Id
            WHERE s.StackSource = @StackSource
              AND s.ControlState <> @ProcessingState
              AND CAST(sr.Spec ->> 'GitRepoId' AS uuid) = @GitRepositoryId
              AND COALESCE(sr.Spec ->> 'CommitSha', '') = ''
              AND COALESCE(NULLIF(sr.Spec ->> 'Branch', ''), gr.DefaultBranch) = @Branch
              AND sr.Status <> ALL(@ExcludedStatuses)
            ORDER BY s.Id
            """;

        var result = await db.QueryAsync<StackDto>(
            sql,
            new
            {
                GitRepositoryId = gitRepositoryId,
                Branch = branch,
                StackSource = EnumFormatter<StackSource>.GetValue(StackSource.Git),
                ProcessingState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Processing),
                ExcludedStatuses = new[]
                {
                    EnumFormatter<StackReleaseStatus>.GetValue(StackReleaseStatus.Unknown),
                    EnumFormatter<StackReleaseStatus>.GetValue(StackReleaseStatus.Created),
                    EnumFormatter<StackReleaseStatus>.GetValue(StackReleaseStatus.Applying),
                    EnumFormatter<StackReleaseStatus>.GetValue(StackReleaseStatus.Pending)
                },
                cancellationToken
            },
            transaction: tx());

        return result.ToDomain();
    }

    public Task<bool> UpdateProcessingAsync(Guid id, StackReleaseStatus status, ResourceControlState state, long? startedAt, long rowVersion, bool? checkRowVersion, Guid? controlTriggeredBy, CancellationToken cancellationToken)
    {
        var sql = $"""
            WITH updated_stack AS (
                UPDATE Stacks
                SET
                    ControlState = @State,
                    ControlStartedAt = @StartedAt,
                    ControlTriggeredBy = @ControlTriggeredBy,
                    RowVersion = RowVersion + 1
                WHERE Id = @Id
                  AND (@CheckRowVersion = false OR RowVersion = @RowVersion)
                RETURNING Id, CurrentStackReleaseId
            ),
            updated_release AS (
                UPDATE StackReleases sr
                SET Status = @Status
                FROM updated_stack us
                WHERE sr.StackId = us.Id
                  AND sr.Id = us.CurrentStackReleaseId
                RETURNING sr.Id
            )
            SELECT EXISTS(SELECT 1 FROM updated_stack);
        """;

        return db.ExecuteScalarAsync<bool>(sql, new
        {
            Id = id,
            Status = EnumFormatter<StackReleaseStatus>.GetValue(status),
            State = EnumFormatter<ResourceControlState>.GetValue(state),
            StartedAt = startedAt,
            RowVersion = rowVersion,
            CheckRowVersion = checkRowVersion,
            ControlTriggeredBy = controlTriggeredBy
        }, tx());
    }

    private static string? SerializeResourceBindings(
        IReadOnlyList<ResourceBindingSnapshot>? configuration)
        => configuration is null
            ? null
            : JsonSerializer.Serialize(configuration, StackJsonContext.Default.IReadOnlyListResourceBindingSnapshot);
}
