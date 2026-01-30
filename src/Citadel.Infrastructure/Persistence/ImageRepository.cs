using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using Infrastructure.TypeHandlers;
using System.Data;
using System.Text.Json;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal class ImageRepository(IDbConnection db, Func<IDbTransaction> tx) : IImageRepository
{
    public async Task<IEnumerable<Image>> GetByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT 
                i.Id,
                i.Name AS Name,
                i.Tags,
                i.DockerImageId,
                i.Size,
                i.Containers,
                i.PlatformId,
                i.CreatedAt,
                i.UpdatedAt,
                i.RegistryId,
                i.ControlState,
                i.ControlStartedAt,
                i.RowVersion,
                r.Name AS RegistryName,
                r.Status AS RegistryStatus,
                r.RegistryHost AS RegistryHost,
                r.CreatedAt As RegistryCreatedAt,
                r.Configuration AS RegistryConfiguration,
                r.CreatedByActorId AS RegistryCreatedByActorId
            FROM Images i
            LEFT JOIN Registries r
                ON i.RegistryId = r.Id
            WHERE PlatformId = @PlatformId
            ORDER BY 
                i.CreatedAt DESC,
                Name ASC
            """;
        var images = await db.QueryAsync<ImageDto>(sql, new { PlatformId = platformId.Format() }, tx());
        return images?.ToDomain() ?? [];
    }

    public async Task<Image?> GetByIdAsync(Guid id, Guid platformId, CancellationToken cancellationToken)
    {
        var sql = """
            SELECT 
                i.Id,
                i.Name AS Name,
                i.Tags,
                i.DockerImageId,
                i.Size,
                i.Containers,
                i.PlatformId,
                i.CreatedAt,
                i.UpdatedAt,
                i.RegistryId,
                i.ControlState,
                i.ControlStartedAt,
                i.RowVersion,
            FROM Images i
            WHERE PlatformId = @PlatformId AND i.Id = @Id
            LIMIT 1
            """;
        var result = await db.QuerySingleOrDefaultAsync<ImageDto>(sql, new { Id = id.Format(), PlatformId = platformId.Format() }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<Image>> GetByIdAsync(string[] ids, Guid platformId, CancellationToken cancellationToken)
    {
        var sql = """
             SELECT
                i.Id,
                i.Name AS Name,
                i.Tags,
                i.DockerImageId,
                i.Size,
                i.Containers,
                i.PlatformId,
                i.CreatedAt,
                i.UpdatedAt,
                i.RegistryId,
                i.ControlState,
                i.ControlStartedAt,
                i.RowVersion
            FROM Images i
            WHERE DockerImageId IN (SELECT value FROM json_each(@Ids)) AND PlatformId = @PlatformId
            """;
        var result = await db.QueryAsync<ImageDto>(sql, new
        {
            Ids = JsonSerializer.Serialize(ids, DeploymentJsonContext.Default.IEnumerableString),
            PlatformId = platformId.Format()
        }, transaction: tx());
        return result?.ToDomain() ?? [];
    }

    public Task<int> UpdateProcessingAsync(Guid id, ResourceControlState state, long? startedAt, long rowVersion, bool? checkRowVersion, CancellationToken cancellationToken)
    {
        var conditions = new List<string>
        {
            "Id = @Id"
        };

        if (checkRowVersion == true)
            conditions.Add("RowVersion = @RowVersion");

        var sql = $"""
            UPDATE Images
            SET
                ControlState = @State,
                ControlStartedAt = @StartedAt,
                RowVersion = RowVersion + 1
            WHERE {string.Join(" AND ", conditions)}
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id.Format(),
                State = EnumFormatter<ResourceControlState>.GetValue(state),
                RowVersion = rowVersion,
                StartedAt = startedAt
            },
            transaction: tx()
        );
    }

    public async Task<Image?> GetByDockerImageIdAsync(string dockerImageId, Guid platformId, CancellationToken cancellationToken)
    {
        if (string.IsNullOrEmpty(dockerImageId)) throw new ArgumentNullException(nameof(dockerImageId));

        var sql = """
            SELECT 
                i.Id,
                i.Name AS Name,
                i.Tags,
                i.DockerImageId,
                i.Size,
                i.Containers,
                i.PlatformId,
                i.CreatedAt,
                i.UpdatedAt,
                i.RegistryId,
                r.Name AS RegistryName,
                r.Status AS RegistryStatus,
                r.RegistryHost AS RegistryHost,
                r.CreatedAt As RegistryCreatedAt,
                r.Configuration AS RegistryConfiguration,
                r.CreatedByActorId AS RegistryCreatedByActorId
            FROM Images i
            LEFT JOIN Registries r
                ON i.RegistryId = r.Id
            WHERE PlatformId = @PlatformId AND DockerImageId LIKE @dockerImageIdPrefix || '%'
            LIMIT 1
            """;
        var result = await db.QuerySingleOrDefaultAsync<ImageDto>(sql, new { dockerImageIdPrefix = dockerImageId, PlatformId = platformId.Format() }, transaction: tx());
        return result?.ToDomain();
    }

    public Task<int> AddOrUpdateAsync(Image image, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO Images (
                Id, PlatformId, DockerImageId, Name, Containers, Tags, Size, RegistryId, CreatedAt, UpdatedAt
            ) VALUES (
                @Id, @PlatformId, @DockerImageId, @Name, @Containers, @Tags, @Size, @RegistryId, @CreatedAt, @UpdatedAt
            )
            ON CONFLICT(DockerImageId, PlatformId) DO UPDATE SET
                Id = excluded.Id,
                Name = excluded.Name,
                Containers = excluded.Containers,
                Tags = excluded.Tags,
                Size = excluded.Size,
                RegistryId = excluded.RegistryId,
                UpdatedAt = excluded.UpdatedAt
        """;

        return db.ExecuteAsync(sql, new
        {
            Id = image.Id.Format(),
            PlatformId = image.PlatformId.Format(),
            DockerImageId = image.DockerImageId,
            Name = image.Name,
            Containers = image.Containers,
            Tags = JsonSerializer.Serialize(image.Tags, ImagTagsContext.Default.IEnumerableString),
            Size = image.Size,
            RegistryId = image.RegistryId?.Format(),
            CreatedAt = image.CreatedAt,
            UpdatedAt = image.UpdatedAt
        }, transaction: tx());
    }

    public async Task<IEnumerable<Image>> GetStuckImagesAsync(int timeout_s = 60, CancellationToken cancellationToken = default)
    {
        var sql = """
            SELECT * FROM Images c
            WHERE ControlState = @ControlState
              AND ControlStartedAt IS NOT NULL
              AND ControlStartedAt < @TimeoutThreshold
            """;
        var timeoutThreshold = DateTimeOffset.UtcNow.ToUnixTimeSeconds() - timeout_s;
        var result = await db.QueryAsync<ImageDto>(sql, new
        {
            ControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Processing),
            TimeoutThreshold = timeoutThreshold
        }, transaction: tx());
        return result?.ToDomain() ?? [];
    }

    public Task<int> BulkUpsertAsync(IEnumerable<Image> images, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO Images (
                Id, PlatformId, DockerImageId, Name, Containers, Tags, Size, RegistryId, CreatedAt, UpdatedAt
            )
            VALUES (
                @Id, @PlatformId, @DockerImageId, @Name, @Containers, @Tags, @Size, @RegistryId, @CreatedAt, @UpdatedAt
            )
            ON CONFLICT(Id) DO UPDATE SET
                PlatformId  = excluded.PlatformId,
                Name        = excluded.Name,
                Containers  = excluded.Containers,
                Tags        = excluded.Tags,
                Size        = excluded.Size,
                RegistryId  = excluded.RegistryId,
                CreatedAt   = excluded.CreatedAt,
                UpdatedAt   = excluded.UpdatedAt;
        """;

        // Dapper will iterate and run the statement once per image, inside the same transaction.
        return db.ExecuteAsync(sql, images.Select(img => new
        {
            Id = img.Id.Format(),
            PlatformId = img.PlatformId.Format(),
            DockerImageId = img.DockerImageId,
            Name = img.Name,
            Containers = img.Containers,
            Tags = JsonSerializer.Serialize(img.Tags, ImagTagsContext.Default.IEnumerableString),
            Size = img.Size,
            RegistryId = img.RegistryId?.Format(),
            CreatedAt = img.CreatedAt,
            UpdatedAt = img.UpdatedAt
        }), transaction: tx());
    }

    public Task<int> DeleteAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = """
        DELETE FROM Images
        WHERE Id IN (
            SELECT value FROM json_each(@Ids)
        )
        """;

        return db.ExecuteAsync(
            sql,
            new { Ids = JsonSerializer.Serialize(ids, DeploymentJsonContext.Default.IEnumerableGuid) },
            transaction: tx()
        );
    }
}
