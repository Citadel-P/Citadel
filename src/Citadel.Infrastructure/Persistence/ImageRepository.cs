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
                i.IsUpToDate,
                i.UpdatedAt,
                i.RegistryId,
                r.Name AS RegistryName,
                r.Status AS RegistryStatus,
                r.RegistryHost AS RegistryHost,
                r.CreatedAt As RegistryCreatedAt,
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
                i.IsUpToDate,
                i.UpdatedAt,
                i.RegistryId
            FROM Images i
            WHERE PlatformId = @PlatformId AND i.Id = @Id
            LIMIT 1
            """;
        var result = await db.QuerySingleOrDefaultAsync<ImageDto>(sql, new { Id = id.Format(), PlatformId = platformId.Format() }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<Image?> GetByDockerImageIdAsync(string dockerImageId, Guid platformId, CancellationToken cancellationToken)
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
                i.IsUpToDate,
                i.UpdatedAt,
                i.RegistryId,
                r.Name AS RegistryName,
                r.Status AS RegistryStatus,
                r.RegistryHost AS RegistryHost,
                r.CreatedAt As RegistryCreatedAt,
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
                Id, PlatformId, DockerImageId, Name, Containers, IsUpToDate, Tags, Size, RegistryId, CreatedAt, UpdatedAt
            ) VALUES (
                @Id, @PlatformId, @DockerImageId, @Name, @Containers, @IsUpToDate, @Tags, @Size, @RegistryId, @CreatedAt, @UpdatedAt
            )
            ON CONFLICT(DockerImageId, PlatformId) DO UPDATE SET
                Name = excluded.Name,
                Containers = excluded.Containers,
                IsUpToDate = excluded.IsUpToDate,
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
            IsUpToDate = image.IsUpToDate,
            Tags = JsonSerializer.Serialize(image.Tags, ImagTagsContext.Default.IEnumerableString),
            Size = image.Size,
            RegistryId = image.RegistryId?.Format(),
            CreatedAt = image.CreatedAt,
            UpdatedAt = image.UpdatedAt
        }, transaction: tx());
    }

    public Task<int> BulkUpsertAsync(IEnumerable<Image> images, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO Images (
                Id, PlatformId, DockerImageId, Name, Containers, IsUpToDate, Tags, Size, RegistryId, CreatedAt, UpdatedAt
            )
            VALUES (
                @Id, @PlatformId, @DockerImageId, @Name, @Containers, @IsUpToDate, @Tags, @Size, @RegistryId, @CreatedAt, @UpdatedAt
            )
            ON CONFLICT(Id) DO UPDATE SET
                PlatformId  = excluded.PlatformId,
                Name        = excluded.Name,
                Containers  = excluded.Containers,
                IsUpToDate  = excluded.IsUpToDate,
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
            IsUpToDate = img.IsUpToDate,
            Tags = JsonSerializer.Serialize(img.Tags, ImagTagsContext.Default.IEnumerableString),
            Size = img.Size,
            RegistryId = img.RegistryId?.Format(),
            CreatedAt = img.CreatedAt,
            UpdatedAt = img.UpdatedAt
        }), transaction: tx());
    }

    public Task<int> DeleteAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        var (clause, parameters) = SqliteInClauseBuilder.BuildInClauseForGuids("Id", ids);
        string sql = $"DELETE FROM Images WHERE Id IN ({clause})";
        return db.ExecuteAsync(sql, parameters, transaction: tx());
    }
}
