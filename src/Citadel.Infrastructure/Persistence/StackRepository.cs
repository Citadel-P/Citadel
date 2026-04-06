using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using Infrastructure.TypeHandlers;
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
        var result = await db.QuerySingleOrDefaultAsync<StackDto>(sql, new { Id = id.Format(), cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<Stack?> GetInfoAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = InfoSelect + " " + "WHERE s.Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<StackDto>(sql, new { Id = id.Format(), cancellationToken }, transaction: tx());
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

    public async Task<IEnumerable<Stack>> GetAuthorizedInfoAsync(Guid userId, ResourceType resourceType, ResourceAction action, CancellationToken cancellationToken)
    {
        const string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + InfoSelect + " WHERE "
            + AuthorizationSql.ResourcePredicatePrefix + "s.Id" + AuthorizationSql.ResourcePredicateSuffix
            + " ORDER BY s.CreatedAt DESC, s.Name ASC";

        var result = await db.QueryAsync<StackDto>(sql, new
        {
            UserId = userId.Format(),
            ResourceType = EnumFormatter<ResourceType>.GetValue(resourceType),
            Action = EnumFormatter<ResourceAction>.GetValue(action),
            cancellationToken
        }, transaction: tx());

        return result.ToDomain();
    }

    public async Task<IEnumerable<Stack>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = InfoSelect + " " + "WHERE s.Id IN (SELECT value FROM json_each(@Ids)) ORDER BY s.CreatedAt DESC, s.Name ASC";
        var result = await db.QueryAsync<StackDto>(sql, new { Ids = JsonSerializer.Serialize(ids, StackJsonContext.Default.IEnumerableGuid), cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<StackRelease>> GetReleasesByStackIdAsync(Guid stackId, CancellationToken cancellationToken)
    {
        const string sql = ReleaseBaseSelect + " " + "WHERE sr.StackId = @StackId ORDER BY sr.CreatedAt DESC, sr.Version DESC";
        var result = await db.QueryAsync<StackReleaseDto>(sql, new { StackId = stackId.Format(), cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public Task<bool> ExistsAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Stacks WHERE Name = @Name)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, cancellationToken }, transaction: tx());
    }

    public Task<bool> ExistsAsync(Guid id, string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Stacks WHERE Name = @Name AND Id != @Id)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, Id = id.Format(), cancellationToken }, transaction: tx());
    }

    public Task<int> AddAsync(Stack stack, CancellationToken cancellationToken)
    {
        const string stackSql = """
            INSERT INTO Stacks (
                Id, CurrentStackReleaseId, Name, Description, StackSource, StackUpdateState,
                CreatedAt, CreatedByActorId, ControlState, ControlStartedAt, RowVersion, ControlTriggeredBy
            ) VALUES (
                @Id, @CurrentStackReleaseId, @Name, @Description, @StackSource, @StackUpdateState,
                @CreatedAt, @CreatedByActorId, @ControlState, @ControlStartedAt, @RowVersion, @ControlTriggeredBy
            )
        """;

        const string stackReleaseSql = """
            INSERT INTO StackReleases (
                Id, StackId, PlatformId, Status, Version, Spec, CreatedAt, CreatedByActorId
            ) VALUES (
                @ReleaseId, @ReleaseStackId, @ReleasePlatformId, @ReleaseStatus, @ReleaseVersion, @ReleaseSpec, @ReleaseCreatedAt, @ReleaseCreatedByActorId
            )
        """;

        var currentStackRelease = stack.CurrentStackRelease ?? throw new InvalidOperationException("Stack must have a current stack release.");

        return db.ExecuteAsync(stackSql + ";\n" + stackReleaseSql, new
        {
            Id = stack.Id.Format(),
            CurrentStackReleaseId = stack.CurrentStackReleaseId.Format(),
            Name = stack.Name,
            Description = stack.Description,
            StackSource = EnumFormatter<StackSource>.GetValue(stack.StackSource),
            StackUpdateState = JsonSerializer.Serialize(stack.StackUpdateState, StackJsonContext.Default.StackUpdateState),
            CreatedAt = stack.CreatedAt.ToString(),
            CreatedByActorId = stack.CreatedByActorId.Format(),
            ControlState = EnumFormatter<ResourceControlState>.GetValue(stack.ControlState),
            ControlStartedAt = stack.ControlStartedAt,
            RowVersion = stack.RowVersion,
            ControlTriggeredBy = stack.ControlTriggeredBy?.Format(),
            ReleaseId = currentStackRelease.Id.Format(),
            ReleaseStackId = currentStackRelease.StackId.Format(),
            ReleasePlatformId = currentStackRelease.PlatformId.Format(),
            ReleaseStatus = currentStackRelease.Status.ToString(),
            ReleaseVersion = currentStackRelease.Version,
            ReleaseSpec = JsonSerializer.Serialize(currentStackRelease.Spec, StackJsonContext.Default.StackSpec),
            ReleaseCreatedAt = currentStackRelease.CreatedAt.ToString(),
            ReleaseCreatedByActorId = currentStackRelease.CreatedByActorId.Format()
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
                StackUpdateState = @StackUpdateState
            WHERE Id = @Id
        """;

        const string stackReleaseSql = """
            INSERT INTO StackReleases (
                Id, StackId, PlatformId, Status, Version, Spec, CreatedAt, CreatedByActorId
            )
            SELECT @ReleaseId, @ReleaseStackId, @ReleasePlatformId, @ReleaseStatus, @ReleaseVersion, @ReleaseSpec, @ReleaseCreatedAt, @ReleaseCreatedByActorId
            WHERE NOT EXISTS (SELECT 1 FROM StackReleases WHERE Id = @ReleaseId)
        """;

        var currentStackRelease = stack.CurrentStackRelease ?? throw new InvalidOperationException("Stack must have a current stack release.");

        return db.ExecuteAsync(stackSql + ";\n" + stackReleaseSql, new
        {
            Id = stack.Id.Format(),
            Name = stack.Name,
            Description = stack.Description,
            CurrentStackReleaseId = stack.CurrentStackReleaseId.Format(),
            StackSource = EnumFormatter<StackSource>.GetValue(stack.StackSource),
            StackUpdateState = JsonSerializer.Serialize(stack.StackUpdateState, StackJsonContext.Default.StackUpdateState),
            ReleaseId = currentStackRelease.Id.Format(),
            ReleaseStackId = currentStackRelease.StackId.Format(),
            ReleasePlatformId = currentStackRelease.PlatformId.Format(),
            ReleaseStatus = currentStackRelease.Status.ToString(),
            ReleaseVersion = currentStackRelease.Version,
            ReleaseSpec = JsonSerializer.Serialize(currentStackRelease.Spec, StackJsonContext.Default.StackSpec),
            ReleaseCreatedAt = currentStackRelease.CreatedAt.ToString(),
            ReleaseCreatedByActorId = currentStackRelease.CreatedByActorId.Format()
        }, transaction: tx());
    }

    public Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = """
        DELETE FROM Stacks
        WHERE Id IN (
            SELECT value FROM json_each(@Ids)
        )
        """;

        return db.ExecuteAsync(
            sql,
            new { Ids = JsonSerializer.Serialize(ids, StackJsonContext.Default.IEnumerableGuid) },
            transaction: tx());
    }
}