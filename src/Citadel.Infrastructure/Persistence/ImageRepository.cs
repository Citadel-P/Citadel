using System.Data;
using Dapper;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using Infrastructure.TypeHandlers;
using Infrastructure.Persistence.Mappers;

using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal class ImageRepository(IDbConnection db, Func<IDbTransaction> tx) : IImageRepository
{
    public async Task<IEnumerable<Image>> GetByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT * FROM Images
            WHERE PlatformId = @PlatformId
            ORDER BY 
                CreatedAt DESC,
                Name ASC
            """;
        var images = await db.QueryAsync<ImageDto>(sql, new { PlatformId = platformId.Format() }, tx());
        return images?.ToDomain() ?? [];
    }

    public async Task<Image?> GetByImageIdAsync(string imageId, CancellationToken cancellationToken)
    {
        var sql = """
            SELECT * FROM Images
            WHERE ImageId LIKE @imageIdPrefix || '%'
            LIMIT 1
            """;
        var result = await db.QuerySingleOrDefaultAsync<ImageDto>(sql, new { imageIdPrefix = imageId }, transaction: tx());
        return result?.ToDomain();
    }

    public Task<int> AddAsync(Image image, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO Images (
                Id, PlatformId, ImageId, Name, IsInUse, IsUpToDate, Tag, Size, RegistryId, CreatedAt, UpdatedAt
            ) VALUES (
                @Id, @PlatformId, @ImageId, @Name, @IsInUse, @IsUpToDate, @Tag, @Size, @RegistryId, @CreatedAt, @UpdatedAt
            )
        """;
        return db.ExecuteAsync(sql, new
        {
            Id = image.Id.Format(),
            PlatformId = image.PlatformId.Format(),
            ImageId = image.ImageId,
            Name = image.Name,
            IsInUse = image.IsInUse,
            IsUpToDate = image.IsUpToDate,
            Tag = image.Tag,
            Size = image.Size,
            RegistryId = image.RegistryId?.Format(),
            CreatedAt = image.CreatedAt,
            UpdatedAt = image.UpdatedAt
        }, transaction: tx());
    }

    public Task<int> AddOrUpdateAsync(Image image, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO Images (
                Id, PlatformId, ImageId, Name, IsInUse, IsUpToDate, Tag, Size, RegistryId, CreatedAt, UpdatedAt
            ) VALUES (
                @Id, @PlatformId, @ImageId, @Name, @IsInUse, @IsUpToDate, @Tag, @Size, @RegistryId, @CreatedAt, @UpdatedAt
            )
            ON CONFLICT(PlatformId, ImageId) DO UPDATE SET
                Name = excluded.Name,
                IsInUse = excluded.IsInUse,
                IsUpToDate = excluded.IsUpToDate,
                Tag = excluded.Tag,
                Size = excluded.Size,
                RegistryId = excluded.RegistryId,
                UpdatedAt = excluded.UpdatedAt
        """;

        return db.ExecuteAsync(sql, new
        {
            Id = image.Id.Format(), // can keep Id for new inserts
            PlatformId = image.PlatformId.Format(),
            ImageId = image.ImageId,
            Name = image.Name,
            IsInUse = image.IsInUse,
            IsUpToDate = image.IsUpToDate,
            Tag = image.Tag,
            Size = image.Size,
            RegistryId = image.RegistryId?.Format(),
            CreatedAt = image.CreatedAt,
            UpdatedAt = image.UpdatedAt
        }, transaction: tx());
    }

    public Task<int> UpdateAsync(Image image, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Images
                SET PlatformId = @PlatformId, ImageId = @ImageId, Name = @Name, 
                    IsInUse = @IsInUse, IsUpToDate = @IsUpToDate, Tag = @Tag, Size = @Size, RegistryId = @RegistryId, CreatedAt = @CreatedAt, UpdatedAt = @UpdatedAt
            WHERE Id = @Id
        """;
        return db.ExecuteAsync(sql, new
        {
            Id = image.Id.Format(),
            PlatformId = image.PlatformId.Format(),
            ImageId = image.ImageId,
            Name = image.Name,
            IsInUse = image.IsInUse,
            IsUpToDate = image.IsUpToDate,
            Tag = image.Tag,
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
                Id, PlatformId, ImageId, Name, IsInUse, IsUpToDate, Tag, Size, RegistryId, CreatedAt, UpdatedAt
            )
            VALUES (
                @Id, @PlatformId, @ImageId, @Name, @IsInUse, @IsUpToDate, @Tag, @Size, @RegistryId, @CreatedAt, @UpdatedAt
            )
            ON CONFLICT(Id) DO UPDATE SET
                PlatformId  = excluded.PlatformId,
                ImageId     = excluded.ImageId,
                Name        = excluded.Name,
                IsInUse     = excluded.IsInUse,
                IsUpToDate  = excluded.IsUpToDate,
                Tag         = excluded.Tag,
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
            ImageId = img.ImageId,
            Name = img.Name,
            IsInUse = img.IsInUse,
            IsUpToDate = img.IsUpToDate,
            Tag = img.Tag,
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
