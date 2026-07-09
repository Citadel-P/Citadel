using System.Data;
using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class EdgeAgentRepository(IDbConnection db, Func<IDbTransaction> tx) : IEdgeAgentRepository
{
    public Task<int> AddEnrollmentAsync(EdgeAgentEnrollment enrollment, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO EdgeAgentEnrollments (
                Id, PlatformId, TokenHash, ExpiresAtUtc, UsedAtUtc, RevokedAtUtc, CreatedByActorId, CreatedAtUtc)
            VALUES (
                @Id, @PlatformId, @TokenHash, @ExpiresAtUtc, @UsedAtUtc, @RevokedAtUtc, @CreatedByActorId, @CreatedAtUtc)
        """;

        return db.ExecuteAsync(sql, new
        {
            enrollment.Id,
            enrollment.PlatformId,
            enrollment.TokenHash,
            enrollment.ExpiresAtUtc,
            enrollment.UsedAtUtc,
            enrollment.RevokedAtUtc,
            enrollment.CreatedByActorId,
            enrollment.CreatedAtUtc
        }, transaction: tx());
    }

    public async Task<EdgeAgentEnrollment?> GetActiveEnrollmentAsync(Guid platformId, DateTime utcNow, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, PlatformId, TokenHash, ExpiresAtUtc, UsedAtUtc, RevokedAtUtc, CreatedByActorId, CreatedAtUtc
            FROM EdgeAgentEnrollments
            WHERE PlatformId = @PlatformId
              AND UsedAtUtc IS NULL
              AND RevokedAtUtc IS NULL
              AND ExpiresAtUtc > @UtcNow
            ORDER BY ExpiresAtUtc DESC
            LIMIT 1
        """;

        var dto = await db.QuerySingleOrDefaultAsync<EdgeAgentEnrollmentDto>(
            sql,
            new
            {
                PlatformId = platformId,
                UtcNow = utcNow
            },
            transaction: tx());
        return dto?.ToDomain();
    }

    public async Task<EdgeAgentEnrollment?> GetEnrollmentByTokenHashAsync(string tokenHash, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, PlatformId, TokenHash, ExpiresAtUtc, UsedAtUtc, RevokedAtUtc, CreatedByActorId, CreatedAtUtc
            FROM EdgeAgentEnrollments
            WHERE TokenHash = @TokenHash
            LIMIT 1
        """;

        var dto = await db.QuerySingleOrDefaultAsync<EdgeAgentEnrollmentDto>(
            sql,
            new
            {
                TokenHash = tokenHash
            },
            transaction: tx());
        return dto?.ToDomain();
    }

    public Task<int> MarkEnrollmentUsedAsync(Guid enrollmentId, DateTime usedAtUtc, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE EdgeAgentEnrollments
            SET UsedAtUtc = @UsedAtUtc
            WHERE Id = @EnrollmentId
              AND UsedAtUtc IS NULL
              AND RevokedAtUtc IS NULL
        """;

        return db.ExecuteAsync(sql, new
        {
            EnrollmentId = enrollmentId,
            UsedAtUtc = usedAtUtc
        }, transaction: tx());
    }

    public async Task<EdgeAgentBinding?> GetBindingByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, PlatformId, AgentId, AgentPublicKey, AgentFingerprint, ConnectionStatus,
                   LastConnectedAtUtc, LastDisconnectedAtUtc, LastHeartbeatAtUtc,
                   LastSeenVersion, LastSeenHostname, CapabilitiesJson, ProtocolVersion,
                   RevokedAtUtc, CreatedAtUtc, UpdatedAtUtc
            FROM EdgeAgentBindings
            WHERE PlatformId = @PlatformId
            LIMIT 1
        """;

        var dto = await db.QuerySingleOrDefaultAsync<EdgeAgentBindingDto>(
            sql,
            new
            {
                PlatformId = platformId
            },
            transaction: tx());
        return dto?.ToDomain();
    }

    public async Task<EdgeAgentBinding?> GetBindingByAgentAsync(Guid platformId, Guid agentId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, PlatformId, AgentId, AgentPublicKey, AgentFingerprint, ConnectionStatus,
                   LastConnectedAtUtc, LastDisconnectedAtUtc, LastHeartbeatAtUtc,
                   LastSeenVersion, LastSeenHostname, CapabilitiesJson, ProtocolVersion,
                   RevokedAtUtc, CreatedAtUtc, UpdatedAtUtc
            FROM EdgeAgentBindings
            WHERE PlatformId = @PlatformId AND AgentId = @AgentId
            LIMIT 1
        """;

        var dto = await db.QuerySingleOrDefaultAsync<EdgeAgentBindingDto>(
            sql,
            new
            {
                PlatformId = platformId,
                AgentId = agentId
            },
            transaction: tx());
        return dto?.ToDomain();
    }

    public Task<int> AddBindingAsync(EdgeAgentBinding binding, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO EdgeAgentBindings (
                Id, PlatformId, AgentId, AgentPublicKey, AgentFingerprint, ConnectionStatus,
                LastConnectedAtUtc, LastDisconnectedAtUtc, LastHeartbeatAtUtc,
                LastSeenVersion, LastSeenHostname, CapabilitiesJson, ProtocolVersion,
                RevokedAtUtc, CreatedAtUtc, UpdatedAtUtc)
            VALUES (
                @Id, @PlatformId, @AgentId, @AgentPublicKey, @AgentFingerprint, @ConnectionStatus,
                @LastConnectedAtUtc, @LastDisconnectedAtUtc, @LastHeartbeatAtUtc,
                @LastSeenVersion, @LastSeenHostname, @CapabilitiesJson::json, @ProtocolVersion,
                @RevokedAtUtc, @CreatedAtUtc, @UpdatedAtUtc)
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                binding.Id,
                binding.PlatformId,
                binding.AgentId,
                binding.AgentPublicKey,
                binding.AgentFingerprint,
                ConnectionStatus = EnumFormatter<EdgeAgentConnectionStatus>.GetValue(binding.ConnectionStatus),
                binding.LastConnectedAtUtc,
                binding.LastDisconnectedAtUtc,
                binding.LastHeartbeatAtUtc,
                binding.LastSeenVersion,
                binding.LastSeenHostname,
                binding.CapabilitiesJson,
                binding.ProtocolVersion,
                binding.RevokedAtUtc,
                binding.CreatedAtUtc,
                binding.UpdatedAtUtc
            },
            transaction: tx());
    }

    public Task<int> UpdateBindingConnectedAsync(Guid platformId, DateTime connectedAtUtc, string hostname, string agentVersion, string capabilitiesJson, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE EdgeAgentBindings
            SET ConnectionStatus = @ConnectionStatus,
                LastConnectedAtUtc = @ConnectedAtUtc,
                LastDisconnectedAtUtc = NULL,
                LastHeartbeatAtUtc = @ConnectedAtUtc,
                LastSeenHostname = @Hostname,
                LastSeenVersion = @AgentVersion,
                CapabilitiesJson = @CapabilitiesJson::json,
                UpdatedAtUtc = @ConnectedAtUtc
            WHERE PlatformId = @PlatformId AND RevokedAtUtc IS NULL
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                PlatformId = platformId,
                ConnectedAtUtc = connectedAtUtc,
                Hostname = hostname,
                AgentVersion = agentVersion,
                CapabilitiesJson = capabilitiesJson,
                ConnectionStatus = EnumFormatter<EdgeAgentConnectionStatus>.GetValue(EdgeAgentConnectionStatus.Connected)
            },
            transaction: tx());
    }

    public Task<int> UpdateBindingHeartbeatAsync(Guid platformId, DateTime heartbeatAtUtc, string? hostname, string? agentVersion, string? capabilitiesJson, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE EdgeAgentBindings
            SET LastHeartbeatAtUtc = @HeartbeatAtUtc,
                LastSeenHostname = COALESCE(@Hostname, LastSeenHostname),
                LastSeenVersion = COALESCE(@AgentVersion, LastSeenVersion),
                CapabilitiesJson = COALESCE(@CapabilitiesJson::json, CapabilitiesJson),
                UpdatedAtUtc = @HeartbeatAtUtc
            WHERE PlatformId = @PlatformId AND RevokedAtUtc IS NULL
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                PlatformId = platformId,
                HeartbeatAtUtc = heartbeatAtUtc,
                Hostname = hostname,
                AgentVersion = agentVersion,
                CapabilitiesJson = capabilitiesJson
            },
            transaction: tx());
    }

    public Task<int> UpdateBindingDisconnectedAsync(Guid platformId, DateTime disconnectedAtUtc, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE EdgeAgentBindings
            SET ConnectionStatus = @ConnectionStatus,
                LastDisconnectedAtUtc = @DisconnectedAtUtc,
                UpdatedAtUtc = @DisconnectedAtUtc
            WHERE PlatformId = @PlatformId AND RevokedAtUtc IS NULL
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                PlatformId = platformId,
                DisconnectedAtUtc = disconnectedAtUtc,
                ConnectionStatus = EnumFormatter<EdgeAgentConnectionStatus>.GetValue(EdgeAgentConnectionStatus.Offline)
            },
            transaction: tx());
    }

    public Task<int> RevokeBindingAsync(Guid platformId, DateTime revokedAtUtc, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE EdgeAgentBindings
            SET ConnectionStatus = @ConnectionStatus,
                RevokedAtUtc = @RevokedAtUtc,
                LastDisconnectedAtUtc = @RevokedAtUtc,
                UpdatedAtUtc = @RevokedAtUtc
            WHERE PlatformId = @PlatformId
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                PlatformId = platformId,
                RevokedAtUtc = revokedAtUtc,
                ConnectionStatus = EnumFormatter<EdgeAgentConnectionStatus>.GetValue(EdgeAgentConnectionStatus.Revoked)
            },
            transaction: tx());
    }
}
