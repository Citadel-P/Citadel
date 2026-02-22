using System.Data;
using System.Text.Json;
using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using Infrastructure.TypeHandlers;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal class PlatformRepository(IDbConnection db, Func<IDbTransaction> tx) : IPlatformRepository 
{
    public async Task<Platform?> GetByIdAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Platforms WHERE Id = @PlatformId LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<PlatformDto>(sql, new { PlatformId = platformId.Format() }, transaction: tx());

        return result?.ToDomain();
    }

    public async Task<Platform?> GetByNameAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Platforms WHERE Name = @Name LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<PlatformDto>(sql, new { Name = name }, transaction: tx());

        return result?.ToDomain();
    }

    public Task<int?> PlatformNameExistsAsync(string name, Guid excludePlatformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT 1
            FROM Platforms
            WHERE Name = @Name AND Id != @Id
            LIMIT 1
        """;
        return db.ExecuteScalarAsync<int?>(sql, new { Id = excludePlatformId.Format(), Name = name }, transaction: tx());
    }

    public Task<bool> NameOrAddressExistsAsync(string name, string address, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS(SELECT 1 FROM Platforms WHERE Address=@Address OR Name=@Name)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, Address = address }, transaction: tx());
    }

    public async Task<PlatformConnectionInfo?> GetInfoAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, Address, ConnectorType
            FROM Platforms
            WHERE Id = @PlatformId
            LIMIT 1
        """;

        var result = await db.QuerySingleOrDefaultAsync<PlatformConnectionInfoDto>(sql, new { PlatformId = platformId.Format() }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<PlatformConnectionInfo>> GetPlatformsInfoAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT Id, Address, ConnectorType FROM Platforms ORDER BY Name";
        var result = await db.QueryAsync<PlatformConnectionInfoDto>(sql, transaction: tx());
        return result.Select(s => s.ToDomain());
    }

    public Task<int> AddAsync(Platform platform, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO Platforms (
                Id, Name, Address, NetworkCount, VolumeCount,  ImageCount, CpuCount, MemTotal, ServerVersion, AgentVersion, Status, ConnectorType, PlatformDescriptor)
            VALUES (
                @Id, @Name, @Address, @NetworkCount, @VolumeCount, @ImageCount, @CpuCount, @MemTotal, @ServerVersion, @AgentVersion, @Status, @ConnectorType, @PlatformDescriptor)
        """;
        return db.ExecuteAsync(sql, new
        {
            Id = platform.Id.Format(),
            Name = platform.Name,
            Address = platform.Address,
            NetworkCount = platform.NetworkCount,
            VolumeCount = platform.VolumeCount,
            ImageCount = platform.ImageCount,
            CpuCount = platform.CpuCount,
            MemTotal = platform.MemTotal,
            ServerVersion = platform.ServerVersion,
            AgentVersion = platform.AgentVersion,
            Status = EnumFormatter<PlatformStatus>.GetValue(platform.Status),
            ConnectorType = EnumFormatter<PlatformConnectorType>.GetValue(platform.ConnectorType),
            PlatformDescriptor = JsonSerializer.Serialize(platform.PlatformDescriptor, PlatformJsonContext.Default.PlatformDescriptor)
        }, transaction: tx());
    }

    public Task<int> DeleteAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM Platforms WHERE Id = @PlatformId";
        return db.ExecuteAsync(sql, new { PlatformId = platformId.Format() }, transaction: tx());
    }

    public Task<int> UpdateAsync(Platform platform, CancellationToken cancellationToken)
    {
        const string updateSql = """
            UPDATE Platforms SET
                Name = @Name,
                Address = @Address,
                NetworkCount = @NetworkCount,
                VolumeCount = @VolumeCount,
                ImageCount = @ImageCount,
                CpuCount = @CpuCount,
                MemTotal = @MemTotal,
                ServerVersion = @ServerVersion,
                AgentVersion = @AgentVersion,
                PlatformDescriptor = @PlatformDescriptor,
                Status = @Status
            WHERE Id = @Id
         """;

        return db.ExecuteAsync(updateSql, new
        {
            platform.Name,
            platform.Address,
            platform.NetworkCount,
            platform.VolumeCount,
            platform.ImageCount,
            platform.CpuCount,
            platform.MemTotal,
            platform.ServerVersion,
            platform.AgentVersion,
            PlatformDescriptor = JsonSerializer.Serialize(platform.PlatformDescriptor, PlatformJsonContext.Default.PlatformDescriptor),
            Status = EnumFormatter<PlatformStatus>.GetValue(platform.Status),
            Id = platform.Id.Format()
        }, transaction: tx());
    }

    public async Task<Platform?> GetPlatformWithLatestStatAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT p.*,
                s.Id as Stat_Id,
                s.Created as Stat_Created,
                s.CpuUsage as Stat_CpuUsage,
                s.MemoryUsage as Stat_MemoryUsage,
                s.RxBytes as Stat_RxBytes,
                s.TxBytes as Stat_TxBytes
            FROM Platforms p
            LEFT JOIN PlatformStats s ON s.Id = (
                SELECT Id FROM PlatformStats
                WHERE PlatformId = p.Id
                ORDER BY Created DESC
                LIMIT 1
            )
            WHERE p.Id = @Id
            LIMIT 1;
        """;

        var result = await db.QuerySingleOrDefaultAsync<PlatformWithSingleStatDto>(sql, new { Id = platformId.Format() }, tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<Platform>?> GetPlatformsWithLatestStatAsync(CancellationToken cancellationToken)
    {
        const string sql = @"
            SELECT p.*,
                s.Id as Stat_Id,
                s.Created as Stat_Created,
                s.CpuUsage as Stat_CpuUsage,
                s.MemoryUsage as Stat_MemoryUsage,
                s.RxBytes as Stat_RxBytes,
                s.TxBytes as Stat_TxBytes
            FROM Platforms p
            LEFT JOIN PlatformStats s ON s.Id = (
                SELECT Id FROM PlatformStats
                WHERE PlatformId = p.Id
                ORDER BY Created DESC
                LIMIT 1
            )
            ORDER BY p.Name;
        ";

        var result = await db.QueryAsync<PlatformWithSingleStatDto>(sql, transaction: tx());
        return result.ToDomain();
    }


    public async Task<PlatformConnectionInfo?> GetPlatformByContainerIdAsync(string dockerContainerId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT 
                Platforms.Id,
                Platforms.Name,
                Platforms.Address,
                Platforms.ConnectorType
            FROM Containers
            JOIN Platforms ON Platforms.Id = Containers.PlatformId
            WHERE Containers.DockerContainerId LIKE @ContainerIdPrefix || '%'
            LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<PlatformConnectionInfoDto>(sql, new { ContainerIdPrefix = dockerContainerId }, transaction: tx());
        return result?.ToDomain();
    }
}
