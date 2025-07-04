using System.Data;
using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;

namespace Infrastructure.Persistence;

internal class PlatformRepository(IDbConnection db, IDbTransaction tx) : IPlatformRepository 
{
    public async Task<Platform?> GetByIdAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Platforms WHERE Id = @PlatformId LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<PlatformDto?>(
            new CommandDefinition(sql, new { PlatformId = platformId }, transaction: tx, cancellationToken: cancellationToken));

        return result?.ToDomain();
    }

    public async Task<Platform?> GetByNameAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Platforms WHERE Name = @Name LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<PlatformDto>(new CommandDefinition(sql, new { Name = name }, transaction: tx, cancellationToken: cancellationToken));
        
        return result?.ToDomain();
    }

    public Task<int?> IsPlatformNameUniqueExceptForIdAsync(Guid platformId, string name, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT 1
            FROM Platforms
            WHERE Name = @Name AND Id != @Id
            LIMIT 1
        """;
        return db.ExecuteScalarAsync<int?>(
            new CommandDefinition(sql, new { Id = platformId, Name = name }, transaction: tx, cancellationToken: cancellationToken));
    }

    public Task<bool> NameOrAddressExistsAsync(string name, string address, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS(SELECT 1 FROM Platforms WHERE Address=@Address OR Name=@Name)";
        return db.ExecuteScalarAsync<bool>(
            new CommandDefinition(sql, new { Name = name, Address = address }, transaction: tx, cancellationToken: cancellationToken));
    }

    public Task<(string Address, PlatformConnectorType ConnectorType)?> GetPlatformInfoAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Address, ConnectorType
            FROM Platforms
            WHERE Id = @PlatformId
            LIMIT 1
        """;

        return db.QuerySingleOrDefaultAsync<(string, PlatformConnectorType)?>(
            new CommandDefinition(sql, new { PlatformId = platformId }, transaction: tx, cancellationToken: cancellationToken));
    }

    public Task<IEnumerable<(Guid Id, string Address, PlatformConnectorType ConnectorType)>> GetPlatformsInfoAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT Id, Address, ConnectorType FROM Platforms ORDER BY Name";
        return db.QueryAsync<(Guid Id, string Address, PlatformConnectorType ConnectorType)>(new CommandDefinition(sql, transaction: tx, cancellationToken: cancellationToken));
    }

    public Task<int> AddPlatformAsync(Platform platform, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO Platforms (
                Id, Name, Address, NetworkCount, VolumeCount,  ImageCount, CpuCount, MemTotal, ServerVersion, AgentVersion, Status, ConnectorType, PlatformDescriptor)
            VALUES (
                @Id, @Name, @Address, @NetworkCount, @VolumeCount, @ImageCount, @CpuCount, @MemTotal, @ServerVersion, @AgentVersion, @Status, @ConnectorType, @PlatformDescriptor)
        """;
        return db.ExecuteAsync(
            new CommandDefinition(sql, platform, transaction: tx, cancellationToken: cancellationToken));
    }

    public Task<int> DeleteAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM Platforms WHERE Id = @PlatformId";
        return db.ExecuteAsync(
            new CommandDefinition(sql, new { PlatformId = platformId }, transaction: tx, cancellationToken: cancellationToken));
    }

    public Task<int> UpdatePlatformAsync(Platform platform, CancellationToken cancellationToken)
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

        return db.ExecuteAsync(new CommandDefinition(updateSql, new
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
            platform.PlatformDescriptor,
            platform.Status,
            platform.Id
        }, transaction: tx, cancellationToken: cancellationToken));
    }

    public async Task<Platform?> GetPlatformWithLatestStatAsync(Guid platformId, CancellationToken cancellationToken)
    {
        var sql = """
            SELECT p.*, s.*
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

        var lookup = new Dictionary<Guid, PlatformDto>();
        var command = new CommandDefinition(
            sql,
            parameters: new { Id = platformId },
            cancellationToken: cancellationToken
        );

        var result = await db.QueryAsync<PlatformDto, PlatformStatDto, PlatformDto>(
            command,
            (platform, stat) =>
            {
                if (!lookup.TryGetValue(platform.Id, out var existing))
                {
                    existing = platform;
                    lookup[platform.Id] = existing;
                }

                if (stat != null)
                    existing.Stats?.Add(stat);

                return existing;
            },
            splitOn: "Id"
        );

        return lookup.Values.SingleOrDefault()?.ToDomain();
    }

    public async Task<IEnumerable<Platform>?> GetPlatformsWithLatestStatAsync(CancellationToken cancellationToken)
    {
        var sql = @"
            SELECT p.*, s.*
            FROM Platforms p
            LEFT JOIN PlatformStats s ON s.Id = (
                SELECT Id FROM PlatformStats
                WHERE PlatformId = p.Id
                ORDER BY Created DESC
                LIMIT 1
            )
            ORDER BY p.Name;
        ";

        var lookup = new Dictionary<Guid, PlatformDto>();

        var command = new CommandDefinition(
            sql,
            cancellationToken: cancellationToken
        );

        var result = await db.QueryAsync<PlatformDto, PlatformStatDto, PlatformDto>(
            command,
            (platform, stat) =>
            {
                if (!lookup.TryGetValue(platform.Id, out var existing))
                {
                    existing = platform;
                    lookup[platform.Id] = existing;
                }

                if (stat != null)
                    existing.Stats?.Add(stat);

                return existing;
            },
            splitOn: "Id"
        );

        return [.. lookup.Values.ToDomain()];
    }

}
