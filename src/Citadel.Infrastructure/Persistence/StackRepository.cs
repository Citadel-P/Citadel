using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Entities.Stacks;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using System.Text.Json;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class StackRepository(IDbConnection db, Func<IDbTransaction> tx) : IStackRepository
{
    private const string BaseSelect = """
    SELECT
        s.Id,
        s.CurrentStackReleaseId,
        s.Name,
        s.Description,
        s.StackSource,
        s.StackUpdateState,
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

    private const string InfoSelect = """
    SELECT
        s.Id,
        s.CurrentStackReleaseId,
        s.Name,
        s.Description,
        s.StackSource,
        s.StackUpdateState,
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
        sr.CreatedAt,
        sr.CreatedByActorId,
        p.Name AS Platform_Name,
        p.Status AS Platform_Status
    FROM StackReleases sr
    LEFT JOIN Platforms p
        ON sr.PlatformId = p.Id
    """;

    public async Task<Stack?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = BaseSelect + " " + "WHERE s.Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<StackDto>(sql, new { Id = id, cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<Stack?> GetInfoAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = InfoSelect + " " + "WHERE s.Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<StackDto>(sql, new { Id = id, cancellationToken }, transaction: tx());
        return result?.ToDomain();
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
        const string sql = """
            SELECT p.Id, p.Name
            FROM Stacks s
            JOIN StackReleases sr ON sr.Id = s.CurrentStackReleaseId
            JOIN Platforms p ON p.Id = sr.PlatformId
            WHERE s.Id = @StackId
            LIMIT 1
         """;

        return db.QueryAsync<ResourceInfo>(sql, new
        {
            StackId = stackId,
            cancellationToken
        }, transaction: tx());
    }

    public Task<IEnumerable<ResourceInfo>> GetRegistryLookupAsync(Guid stackId, Guid userId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT DISTINCT r.Id, r.Name
            FROM Stacks s
            JOIN StackReleases sr ON sr.Id = s.CurrentStackReleaseId
            JOIN Containers c ON c.PlatformId = sr.PlatformId AND c.Stack = s.Name
            JOIN Images i ON i.Id = c.ImageId
            JOIN Registries r ON r.Id = i.RegistryId
            WHERE s.Id = @StackId
            ORDER BY r.Name
         """;

        return db.QueryAsync<ResourceInfo>(sql, new
        {
            StackId = stackId,
            cancellationToken
        }, transaction: tx());
    }

    public Task<IEnumerable<ResourceInfo>> GetGitRepositoryLookupAsync(Guid stackId, Guid userId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT gr.Id, gr.Name
            FROM Stacks s
            JOIN StackReleases sr ON sr.Id = s.CurrentStackReleaseId
            JOIN GitRepositories gr ON gr.Id = CAST(sr.Spec ->> 'GitRepoId' AS uuid)
            WHERE s.Id = @StackId
              AND s.StackSource = @StackSource
            LIMIT 1
         """;

        return db.QueryAsync<ResourceInfo>(sql, new
        {
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
                CreatedAt, CreatedByActorId, ControlState, ControlStartedAt, RowVersion, ControlTriggeredBy
            ) VALUES (
                @Id, @CurrentStackReleaseId, @Name, @Description, @StackSource, @StackUpdateState::json,
                @CreatedAt, @CreatedByActorId, @ControlState, @ControlStartedAt, @RowVersion, @ControlTriggeredBy
            )
        """;

        const string stackReleaseSql = """
            INSERT INTO StackReleases (
                Id, StackId, PlatformId, Status, Version, Spec, CreatedAt, CreatedByActorId
            ) VALUES (
                @ReleaseId, @ReleaseStackId, @ReleasePlatformId, @ReleaseStatus, @ReleaseVersion, @ReleaseSpec::json, @ReleaseCreatedAt, @ReleaseCreatedByActorId
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
            ReleaseCreatedAt = currentStackRelease.CreatedAt,
            ReleaseCreatedByActorId = currentStackRelease.CreatedByActorId
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
                StackUpdateState = @StackUpdateState::json
            WHERE Id = @Id
        """;

        const string stackReleaseSql = """
            INSERT INTO StackReleases (
                Id, StackId, PlatformId, Status, Version, Spec, CreatedAt, CreatedByActorId
            )
            SELECT @ReleaseId, @ReleaseStackId, @ReleasePlatformId, @ReleaseStatus, @ReleaseVersion, @ReleaseSpec::json, @ReleaseCreatedAt, @ReleaseCreatedByActorId
            WHERE NOT EXISTS (SELECT 1 FROM StackReleases WHERE Id = @ReleaseId)
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
            ReleaseId = currentStackRelease.Id,
            ReleaseStackId = currentStackRelease.StackId,
            ReleasePlatformId = currentStackRelease.PlatformId,
            ReleaseStatus = EnumFormatter<StackReleaseStatus>.GetValue(currentStackRelease.Status),
            ReleaseVersion = currentStackRelease.Version,
            ReleaseSpec = JsonSerializer.Serialize(currentStackRelease.Spec, StackJsonContext.Default.StackSpec),
            ReleaseCreatedAt = currentStackRelease.CreatedAt,
            ReleaseCreatedByActorId = currentStackRelease.CreatedByActorId
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
}