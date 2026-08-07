using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.SwarmServices;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using System.Text.Json;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class SwarmServiceRepository(IDbConnection db, Func<IDbTransaction> tx) : ISwarmServiceRepository
{
    private const int MaximumListItems = 500;
    private static readonly string BaseSelect = $$"""
        SELECT s.*,
               p.Name AS Platform_Name,
               p.Status AS Platform_Status,
               p.PlatformDescriptor AS Platform_Descriptor,
               {{ResourceTagSql.TagAggregate("s")}}
        FROM SwarmServices s
        JOIN Platforms p ON p.Id = s.PlatformId
        """;

    public async Task<SwarmService?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        var sql = BaseSelect + " WHERE s.Id = @Id LIMIT 1";
        var value = await db.QuerySingleOrDefaultAsync<SwarmServiceDto>(sql, new
        {
            Id = id,
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.SwarmService)
        }, transaction: tx());
        return value?.ToDomain();
    }

    public async Task<IReadOnlyList<SwarmService>> GetAllAsync(CancellationToken cancellationToken)
    {
        var sql = BaseSelect + " ORDER BY s.CreatedAt";
        var values = await db.QueryAsync<SwarmServiceDto>(sql, new
        {
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.SwarmService)
        }, transaction: tx());
        return values.Select(static value => value.ToDomain()).ToArray();
    }

    public async Task<IReadOnlyList<SwarmService>> GetByPlatformAsync(Guid platformId, CancellationToken cancellationToken)
    {
        var sql = BaseSelect + " WHERE s.PlatformId = @PlatformId ORDER BY s.CreatedAt";
        var values = await db.QueryAsync<SwarmServiceDto>(sql, new
        {
            PlatformId = platformId,
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.SwarmService)
        }, transaction: tx());
        return values.Select(static value => value.ToDomain()).ToArray();
    }

    public async Task<IEnumerable<SwarmService>> GetInfoAsync(
        CancellationToken cancellationToken,
        IReadOnlyCollection<Guid>? tagIds = null,
        Guid? platformId = null)
    {
        var sql = BaseSelect + " WHERE " + ResourceTagSql.FilterPredicate("s")
            + " AND (@PlatformId IS NULL OR s.PlatformId = @PlatformId)"
            + " ORDER BY s.CreatedAt DESC, s.Name ASC LIMIT @Limit";
        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var values = await db.QueryAsync<SwarmServiceDto>(sql, new
        {
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.SwarmService),
            TagIds = tagIdArray,
            TagIdsLength = tagIdArray.Length,
            PlatformId = platformId,
            Limit = MaximumListItems
        }, transaction: tx());
        return values.ToDomain();
    }

    public async Task<IEnumerable<SwarmService>> GetAuthorizedInfoAsync(
        Guid userId,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        CancellationToken cancellationToken,
        IReadOnlyCollection<Guid>? tagIds = null,
        Guid? platformId = null)
    {
        var sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte
            + BaseSelect + " WHERE "
            + AuthorizationSql.ResourcePredicatePrefix + "s.Id" + AuthorizationSql.ResourcePredicateSuffix
            + " AND " + PlatformAccessPredicate
            + " AND " + ResourceTagSql.FilterPredicate("s")
            + " AND (@PlatformId IS NULL OR s.PlatformId = @PlatformId)"
            + " ORDER BY s.CreatedAt DESC, s.Name ASC LIMIT @Limit";
        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var values = await db.QueryAsync<SwarmServiceDto>(sql, new
        {
            UserId = userId,
            ResourceType = (int)ResourceType.SwarmService,
            PlatformResourceType = (int)ResourceType.Platform,
            GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel),
            SpecificPermission = (int)specificPermission,
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.SwarmService),
            TagIds = tagIdArray,
            TagIdsLength = tagIdArray.Length,
            PlatformId = platformId,
            Limit = MaximumListItems
        }, transaction: tx());
        return values.ToDomain();
    }

    public Task<bool> ExistsAsync(Guid platformId, string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM SwarmServices WHERE PlatformId = @PlatformId AND Name = @Name)";
        return db.ExecuteScalarAsync<bool>(sql, new { PlatformId = platformId, Name = name }, transaction: tx());
    }

    public Task<bool> ExistsAsync(Guid id, Guid platformId, string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM SwarmServices WHERE Id <> @Id AND PlatformId = @PlatformId AND Name = @Name)";
        return db.ExecuteScalarAsync<bool>(sql, new { Id = id, PlatformId = platformId, Name = name }, transaction: tx());
    }

    public Task<bool> DockerNameExistsAsync(Guid platformId, string dockerName, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM SwarmServices WHERE PlatformId = @PlatformId AND DockerName = @DockerName)";
        return db.ExecuteScalarAsync<bool>(sql, new { PlatformId = platformId, DockerName = dockerName }, transaction: tx());
    }

    public async Task<int> AddAsync(
        SwarmService service,
        CancellationToken cancellationToken,
        IReadOnlyCollection<Guid>? tagIds = null,
        Guid? tagCreatedByActorId = null)
    {
        var sql = ResourceTagSql.InputTagsCte + """
            inserted_service AS (
                INSERT INTO SwarmServices (
                    Id, PlatformId, Name, Description, DockerName, DockerServiceId, Spec,
                    AutoUpdateState_LastCheckedAt, AutoUpdateState_Status,
                    AutoUpdateState_CurrentDigest, AutoUpdateState_RemoteDigest,
                    AutoUpdateState_LastError, Health, SynchronizationState,
                    ControlState, ControlStartedAt, ControlTriggeredBy, DesiredSpecHash,
                    LastAppliedDesiredSpecHash, LastAppliedRuntimeHash, AppliedImageDigest,
                    DockerVersionIndex, CreatedByActorId, CreatedAt, UpdatedAt, RowVersion)
                SELECT @Id, @PlatformId, @Name, @Description, @DockerName, @DockerServiceId,
                    @Spec::jsonb, @AutoUpdateState_LastCheckedAt, @AutoUpdateState_Status,
                    @AutoUpdateState_CurrentDigest, @AutoUpdateState_RemoteDigest,
                    @AutoUpdateState_LastError, @Health, @SynchronizationState,
                    @ControlState, @ControlStartedAt, @ControlTriggeredBy, @DesiredSpecHash,
                    @LastAppliedDesiredSpecHash, @LastAppliedRuntimeHash, @AppliedImageDigest,
                    @DockerVersionIndex, @CreatedByActorId, @CreatedAt, @UpdatedAt, @RowVersion
                WHERE NOT EXISTS (SELECT 1 FROM missing_tags)
                ON CONFLICT DO NOTHING
                RETURNING Id
            ),
            """ + ResourceTagSql.InsertTagsCte("inserted_service", "s") + "\n"
            + ResourceTagSql.InsertResultSelect("inserted_service", "inserted_service", "inserted_tags");

        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QuerySingleAsync<ResourceInsertWithTagsResult>(sql, Parameters(
            service,
            ResourceTagSql.GetResourceTypeValue(TaggableResourceType.SwarmService),
            tagIdArray,
            tagCreatedByActorId ?? service.CreatedByActorId), transaction: tx());
        service.AssignTags(result.TagsJson.ToTagSummaries());
        return result.AffectedRows;
    }

    public async Task<int> UpdateAsync(SwarmService service, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE SwarmServices SET
                Name=@Name, Description=@Description, Spec=@Spec::jsonb,
                DockerServiceId=@DockerServiceId,
                AutoUpdateState_LastCheckedAt=@AutoUpdateState_LastCheckedAt,
                AutoUpdateState_Status=@AutoUpdateState_Status,
                AutoUpdateState_CurrentDigest=@AutoUpdateState_CurrentDigest,
                AutoUpdateState_RemoteDigest=@AutoUpdateState_RemoteDigest,
                AutoUpdateState_LastError=@AutoUpdateState_LastError,
                Health=@Health, SynchronizationState=@SynchronizationState,
                ControlState=@ControlState, ControlStartedAt=@ControlStartedAt,
                ControlTriggeredBy=@ControlTriggeredBy, DesiredSpecHash=@DesiredSpecHash,
                LastAppliedDesiredSpecHash=@LastAppliedDesiredSpecHash,
                LastAppliedRuntimeHash=@LastAppliedRuntimeHash,
                AppliedImageDigest=@AppliedImageDigest, DockerVersionIndex=@DockerVersionIndex,
                OperationId=@OperationId, OperationKind=@OperationKind, OperationState=@OperationState,
                BaseDockerVersion=@BaseDockerVersion,
                TargetDesiredSpecHash=@TargetDesiredSpecHash,
                TargetRuntimeHash=@TargetRuntimeHash, TargetRowVersion=@TargetRowVersion,
                ExpectedForceUpdate=@ExpectedForceUpdate, PreparedAt=@PreparedAt,
                AttemptedAt=@AttemptedAt, CompletedAt=@CompletedAt,
                ObservedDockerVersion=@ObservedDockerVersion, ResultCode=@ResultCode,
                Warnings=@Warnings::jsonb, ResultMessage=@ResultMessage,
                OperationClusterId=@OperationClusterId, OperationActorId=@OperationActorId,
                UpdatedAt=@UpdatedAt, RowVersion=RowVersion+1
            WHERE Id=@Id AND RowVersion=@RowVersion
            RETURNING RowVersion
            """;
        var rowVersion = await db.QuerySingleOrDefaultAsync<long>(sql, Parameters(service), transaction: tx());
        if (rowVersion == 0)
            return 0;

        service.AcceptPersistedRowVersion(rowVersion);
        return 1;
    }

    public Task<bool> CanAccessAsync(
        Guid userId,
        Guid serviceId,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        CancellationToken cancellationToken)
    {
        const string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}}
            SELECT EXISTS (
                SELECT 1 FROM SwarmServices s
                WHERE s.Id = @ServiceId
                  AND {{AuthorizationSql.ResourcePredicatePrefix}}s.Id{{AuthorizationSql.ResourcePredicateSuffix}}
                  AND {{PlatformAccessPredicate}})
            """;
        return db.ExecuteScalarAsync<bool>(sql, new
        {
            UserId = userId,
            ServiceId = serviceId,
            ResourceType = (int)ResourceType.SwarmService,
            PlatformResourceType = (int)ResourceType.Platform,
            GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel),
            SpecificPermission = (int)specificPermission
        }, transaction: tx());
    }

    private const string PlatformAccessPredicate = """
        (
            EXISTS (
                SELECT 1
                FROM ActorRoles actorRoles
                JOIN Permissions permissions ON permissions.RoleId = actorRoles.RoleId
                JOIN ActorScope actorScope ON actorScope.ActorId = actorRoles.ActorId
                WHERE permissions.ResourceType = @PlatformResourceType
                  AND (permissions.PermissionLevel & @GrantedPermissionMask) <> 0)
            OR EXISTS (
                SELECT 1
                FROM ResourceAccesses resourceAccesses
                JOIN ActorScope actorScope ON actorScope.ActorId = resourceAccesses.ActorId
                WHERE resourceAccesses.ResourceType = @PlatformResourceType
                  AND resourceAccesses.ResourceId = s.PlatformId
                  AND (resourceAccesses.PermissionLevel & @GrantedPermissionMask) <> 0)
        )
        """;

    public Task<int> RemoveAsync(Guid id, long rowVersion, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM SwarmServices WHERE Id=@Id AND RowVersion=@RowVersion";
        return db.ExecuteAsync(sql, new { Id = id, RowVersion = rowVersion }, transaction: tx());
    }

    public async Task<IReadOnlyList<SwarmService>> GetStuckOperationsAsync(
        int staleAfterSeconds = 300,
        CancellationToken cancellationToken = default)
    {
        var sql = BaseSelect + " " + """
            WHERE s.ControlState = 'Processing'
              AND s.ControlStartedAt < @ControlStartedAt
              AND s.OperationId IS NOT NULL
              AND s.OperationState NOT IN ('Completed', 'Canceled', 'Rejected', 'NotAccepted', 'OwnershipConflict')
            ORDER BY s.ControlStartedAt
            LIMIT 100
            """;
        var values = await db.QueryAsync<SwarmServiceDto>(sql, new
        {
            ControlStartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds() - staleAfterSeconds,
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.SwarmService)
        }, transaction: tx());
        return values.Select(static value => value.ToDomain()).ToArray();
    }

    private static SwarmServicePersistenceParameters Parameters(
        SwarmService service,
        string? tagResourceType = null,
        Guid[]? tagIds = null,
        Guid tagCreatedByActorId = default)
    {
        var operation = service.CurrentOperation;
        return new SwarmServicePersistenceParameters
        {
            Id = service.Id,
            PlatformId = service.PlatformId,
            Name = service.Name,
            Description = service.Description,
            DockerName = service.DockerName,
            DockerServiceId = service.DockerServiceId,
            Spec = JsonSerializer.Serialize(service.Spec, SwarmServiceJsonContext.Default.SwarmServiceSpec),
            AutoUpdateState_LastCheckedAt = service.AutoUpdateState.LastCheckedAt,
            AutoUpdateState_Status = EnumFormatter<AutoUpdateStatus>.GetValue(service.AutoUpdateState.Status),
            AutoUpdateState_CurrentDigest = service.AutoUpdateState.CurrentDigest,
            AutoUpdateState_RemoteDigest = service.AutoUpdateState.RemoteDigest,
            AutoUpdateState_LastError = service.AutoUpdateState.LastError,
            Health = EnumFormatter<SwarmServiceHealth>.GetValue(service.Health),
            SynchronizationState = EnumFormatter<SwarmServiceSynchronizationState>.GetValue(service.SynchronizationState),
            ControlState = EnumFormatter<ResourceControlState>.GetValue(service.ControlState),
            ControlStartedAt = service.ControlStartedAt,
            ControlTriggeredBy = service.ControlTriggeredBy,
            DesiredSpecHash = service.DesiredSpecHash,
            LastAppliedDesiredSpecHash = service.LastAppliedDesiredSpecHash,
            LastAppliedRuntimeHash = service.LastAppliedRuntimeHash,
            AppliedImageDigest = service.AppliedImageDigest,
            DockerVersionIndex = service.DockerVersionIndex,
            OperationId = operation?.Id,
            OperationKind = operation is null ? null : EnumFormatter<SwarmServiceOperationKind>.GetValue(operation.Kind),
            OperationState = operation is null ? null : EnumFormatter<SwarmServiceOperationState>.GetValue(operation.State),
            BaseDockerVersion = operation?.BaseDockerVersion,
            TargetDesiredSpecHash = operation?.TargetDesiredSpecHash,
            TargetRuntimeHash = operation?.TargetRuntimeHash,
            TargetRowVersion = operation?.TargetRowVersion,
            ExpectedForceUpdate = operation?.ExpectedForceUpdate,
            PreparedAt = operation?.PreparedAt,
            AttemptedAt = operation?.AttemptedAt,
            CompletedAt = operation?.CompletedAt,
            ObservedDockerVersion = operation?.ObservedDockerVersion,
            ResultCode = operation?.ResultCode,
            Warnings = operation is null ? null : JsonSerializer.Serialize(operation.Warnings ?? [], PlatformJsonContext.Default.StringArray),
            ResultMessage = operation?.ResultMessage,
            OperationClusterId = operation?.ClusterId,
            OperationActorId = operation?.ActorId,
            CreatedByActorId = service.CreatedByActorId,
            CreatedAt = service.CreatedAt,
            UpdatedAt = service.UpdatedAt,
            RowVersion = service.RowVersion,
            TagResourceType = tagResourceType ?? string.Empty,
            TagIds = tagIds ?? [],
            TagCreatedByActorId = tagCreatedByActorId
        };
    }
}
