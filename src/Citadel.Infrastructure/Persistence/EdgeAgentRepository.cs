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
                Id, PlatformId, ResourceType, ResourceId, TokenHash, ExpiresAtUtc, UsedAtUtc, RevokedAtUtc, CreatedByActorId, CreatedAtUtc)
            VALUES (
                @Id, @PlatformId, @ResourceType, @ResourceId, @TokenHash, @ExpiresAtUtc, @UsedAtUtc, @RevokedAtUtc, @CreatedByActorId, @CreatedAtUtc)
        """;

        return db.ExecuteAsync(sql, new
        {
            enrollment.Id,
            enrollment.PlatformId,
            ResourceType = EnumFormatter<EdgeAgentResourceType>.GetValue(enrollment.NormalizedResourceType),
            ResourceId = enrollment.NormalizedResourceId,
            enrollment.TokenHash,
            enrollment.ExpiresAtUtc,
            enrollment.UsedAtUtc,
            enrollment.RevokedAtUtc,
            enrollment.CreatedByActorId,
            enrollment.CreatedAtUtc
        }, transaction: tx());
    }

    public async Task<EdgeAgentEnrollment?> GetActiveEnrollmentAsync(Guid platformId, DateTime utcNow, CancellationToken cancellationToken)
        => await GetActiveEnrollmentAsync(EdgeAgentResourceType.Platform, platformId, utcNow, cancellationToken);

    public async Task<EdgeAgentEnrollment?> GetActiveEnrollmentAsync(EdgeAgentResourceType resourceType, Guid resourceId, DateTime utcNow, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, PlatformId, ResourceType, ResourceId, TokenHash, ExpiresAtUtc, UsedAtUtc, RevokedAtUtc, CreatedByActorId, CreatedAtUtc
            FROM EdgeAgentEnrollments
            WHERE ResourceType = @ResourceType
              AND ResourceId = @ResourceId
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
                ResourceType = EnumFormatter<EdgeAgentResourceType>.GetValue(resourceType),
                ResourceId = resourceId,
                UtcNow = utcNow
            },
            transaction: tx());
        return dto?.ToDomain();
    }

    public async Task<EdgeAgentEnrollment?> GetEnrollmentByTokenHashAsync(string tokenHash, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, PlatformId, ResourceType, ResourceId, TokenHash, ExpiresAtUtc, UsedAtUtc, RevokedAtUtc, CreatedByActorId, CreatedAtUtc
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
        => await GetBindingByResourceAsync(EdgeAgentResourceType.Platform, platformId, cancellationToken);

    public async Task<EdgeAgentBinding?> GetBindingByResourceAsync(EdgeAgentResourceType resourceType, Guid resourceId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, PlatformId, ResourceType, ResourceId, AgentId, AgentPublicKey, AgentFingerprint, ConnectionStatus,
                   LastConnectedAtUtc, LastDisconnectedAtUtc, LastHeartbeatAtUtc,
                   LastSeenVersion, LastSeenHostname, CapabilitiesJson, ProtocolVersion,
                   RevokedAtUtc, CreatedAtUtc, UpdatedAtUtc
            FROM EdgeAgentBindings
            WHERE ResourceType = @ResourceType
              AND ResourceId = @ResourceId
            LIMIT 1
        """;

        var dto = await db.QuerySingleOrDefaultAsync<EdgeAgentBindingDto>(
            sql,
            new
            {
                ResourceType = EnumFormatter<EdgeAgentResourceType>.GetValue(resourceType),
                ResourceId = resourceId
            },
            transaction: tx());
        return dto?.ToDomain();
    }

    public async Task<EdgeAgentPlatformState?> GetPlatformStateByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                p.Id AS PlatformId,
                p.Name AS PlatformName,
                p.Address AS PlatformAddress,
                p.NetworkCount AS PlatformNetworkCount,
                p.VolumeCount AS PlatformVolumeCount,
                p.ImageCount AS PlatformImageCount,
                p.CpuCount AS PlatformCpuCount,
                p.MemTotal AS PlatformMemTotal,
                p.Status AS PlatformStatus,
                p.ConnectorType AS PlatformConnectorType,
                p.PlatformDescriptor AS PlatformDescriptor,
                p.ServerVersion AS PlatformServerVersion,
                p.AgentVersion AS PlatformAgentVersion,
                p.Description AS PlatformDescription,
                p.ClusterId AS PlatformClusterId,
                p.PruneHistoricalSwarmTaskContainers AS PlatformPruneHistoricalSwarmTaskContainers,
                e.Id AS BindingId,
                e.PlatformId AS BindingPlatformId,
                e.ResourceType AS BindingResourceType,
                e.ResourceId AS BindingResourceId,
                e.AgentId AS BindingAgentId,
                e.AgentPublicKey AS BindingAgentPublicKey,
                e.AgentFingerprint AS BindingAgentFingerprint,
                e.ConnectionStatus AS BindingConnectionStatus,
                e.LastConnectedAtUtc AS BindingLastConnectedAtUtc,
                e.LastDisconnectedAtUtc AS BindingLastDisconnectedAtUtc,
                e.LastHeartbeatAtUtc AS BindingLastHeartbeatAtUtc,
                e.LastSeenVersion AS BindingLastSeenVersion,
                e.LastSeenHostname AS BindingLastSeenHostname,
                e.CapabilitiesJson AS BindingCapabilitiesJson,
                e.ProtocolVersion AS BindingProtocolVersion,
                e.RevokedAtUtc AS BindingRevokedAtUtc,
                e.CreatedAtUtc AS BindingCreatedAtUtc,
                e.UpdatedAtUtc AS BindingUpdatedAtUtc
            FROM Platforms p
            LEFT JOIN EdgeAgentBindings e
              ON e.ResourceType = 'Platform'
             AND e.ResourceId = p.Id
            WHERE p.Id = @PlatformId
            LIMIT 1
        """;

        var dto = await db.QuerySingleOrDefaultAsync<EdgeAgentPlatformStateDto>(
            sql,
            new
            {
                PlatformId = platformId
            },
            transaction: tx());
        return dto?.ToDomain();
    }

    public async Task<EdgeAgentBinding?> GetBindingByAgentAsync(Guid platformId, Guid agentId, CancellationToken cancellationToken)
        => await GetBindingByAgentAsync(EdgeAgentResourceType.Platform, platformId, agentId, cancellationToken);

    public async Task<EdgeAgentBinding?> GetBindingByAgentAsync(EdgeAgentResourceType resourceType, Guid resourceId, Guid agentId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, PlatformId, ResourceType, ResourceId, AgentId, AgentPublicKey, AgentFingerprint, ConnectionStatus,
                   LastConnectedAtUtc, LastDisconnectedAtUtc, LastHeartbeatAtUtc,
                   LastSeenVersion, LastSeenHostname, CapabilitiesJson, ProtocolVersion,
                   RevokedAtUtc, CreatedAtUtc, UpdatedAtUtc
            FROM EdgeAgentBindings
            WHERE ResourceType = @ResourceType
              AND ResourceId = @ResourceId
              AND AgentId = @AgentId
            LIMIT 1
        """;

        var dto = await db.QuerySingleOrDefaultAsync<EdgeAgentBindingDto>(
            sql,
            new
            {
                ResourceType = EnumFormatter<EdgeAgentResourceType>.GetValue(resourceType),
                ResourceId = resourceId,
                AgentId = agentId
            },
            transaction: tx());
        return dto?.ToDomain();
    }

    public Task<int> AddBindingAsync(EdgeAgentBinding binding, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO EdgeAgentBindings (
                Id, PlatformId, ResourceType, ResourceId, AgentId, AgentPublicKey, AgentFingerprint, ConnectionStatus,
                LastConnectedAtUtc, LastDisconnectedAtUtc, LastHeartbeatAtUtc,
                LastSeenVersion, LastSeenHostname, CapabilitiesJson, ProtocolVersion,
                RevokedAtUtc, CreatedAtUtc, UpdatedAtUtc)
            VALUES (
                @Id, @PlatformId, @ResourceType, @ResourceId, @AgentId, @AgentPublicKey, @AgentFingerprint, @ConnectionStatus,
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
                ResourceType = EnumFormatter<EdgeAgentResourceType>.GetValue(binding.NormalizedResourceType),
                ResourceId = binding.NormalizedResourceId,
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
        => UpdateBindingConnectedAsync(EdgeAgentResourceType.Platform, platformId, connectedAtUtc, hostname, agentVersion, capabilitiesJson, cancellationToken);

    public Task<int> UpdateBindingConnectedAsync(EdgeAgentResourceType resourceType, Guid resourceId, DateTime connectedAtUtc, string hostname, string agentVersion, string capabilitiesJson, CancellationToken cancellationToken)
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
            WHERE ResourceType = @ResourceType
              AND ResourceId = @ResourceId
              AND RevokedAtUtc IS NULL
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                ResourceType = EnumFormatter<EdgeAgentResourceType>.GetValue(resourceType),
                ResourceId = resourceId,
                ConnectedAtUtc = connectedAtUtc,
                Hostname = hostname,
                AgentVersion = agentVersion,
                CapabilitiesJson = capabilitiesJson,
                ConnectionStatus = EnumFormatter<EdgeAgentConnectionStatus>.GetValue(EdgeAgentConnectionStatus.Connected)
            },
            transaction: tx());
    }

    public Task<int> UpdateBindingHeartbeatAsync(Guid platformId, DateTime heartbeatAtUtc, string? hostname, string? agentVersion, string? capabilitiesJson, CancellationToken cancellationToken)
        => UpdateBindingHeartbeatAsync(EdgeAgentResourceType.Platform, platformId, heartbeatAtUtc, hostname, agentVersion, capabilitiesJson, cancellationToken);

    public Task<int> UpdateBindingHeartbeatAsync(EdgeAgentResourceType resourceType, Guid resourceId, DateTime heartbeatAtUtc, string? hostname, string? agentVersion, string? capabilitiesJson, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE EdgeAgentBindings
            SET LastHeartbeatAtUtc = @HeartbeatAtUtc,
                LastSeenHostname = COALESCE(@Hostname, LastSeenHostname),
                LastSeenVersion = COALESCE(@AgentVersion, LastSeenVersion),
                CapabilitiesJson = COALESCE(@CapabilitiesJson::json, CapabilitiesJson),
                UpdatedAtUtc = @HeartbeatAtUtc
            WHERE ResourceType = @ResourceType
              AND ResourceId = @ResourceId
              AND RevokedAtUtc IS NULL
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                ResourceType = EnumFormatter<EdgeAgentResourceType>.GetValue(resourceType),
                ResourceId = resourceId,
                HeartbeatAtUtc = heartbeatAtUtc,
                Hostname = hostname,
                AgentVersion = agentVersion,
                CapabilitiesJson = capabilitiesJson
            },
            transaction: tx());
    }

    public Task<int> UpdateBindingDisconnectedAsync(Guid platformId, DateTime disconnectedAtUtc, CancellationToken cancellationToken)
        => UpdateBindingDisconnectedAsync(EdgeAgentResourceType.Platform, platformId, disconnectedAtUtc, cancellationToken);

    public Task<int> UpdateBindingDisconnectedAsync(EdgeAgentResourceType resourceType, Guid resourceId, DateTime disconnectedAtUtc, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE EdgeAgentBindings
            SET ConnectionStatus = @ConnectionStatus,
                LastDisconnectedAtUtc = @DisconnectedAtUtc,
                UpdatedAtUtc = @DisconnectedAtUtc
            WHERE ResourceType = @ResourceType
              AND ResourceId = @ResourceId
              AND RevokedAtUtc IS NULL
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                ResourceType = EnumFormatter<EdgeAgentResourceType>.GetValue(resourceType),
                ResourceId = resourceId,
                DisconnectedAtUtc = disconnectedAtUtc,
                ConnectionStatus = EnumFormatter<EdgeAgentConnectionStatus>.GetValue(EdgeAgentConnectionStatus.Offline)
            },
            transaction: tx());
    }

    public Task<int> RevokeBindingAsync(Guid platformId, DateTime revokedAtUtc, CancellationToken cancellationToken)
        => RevokeBindingAsync(EdgeAgentResourceType.Platform, platformId, revokedAtUtc, cancellationToken);

    public Task<int> RevokeBindingAsync(EdgeAgentResourceType resourceType, Guid resourceId, DateTime revokedAtUtc, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE EdgeAgentBindings
            SET ConnectionStatus = @ConnectionStatus,
                RevokedAtUtc = @RevokedAtUtc,
                LastDisconnectedAtUtc = @RevokedAtUtc,
                UpdatedAtUtc = @RevokedAtUtc
            WHERE ResourceType = @ResourceType
              AND ResourceId = @ResourceId
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                ResourceType = EnumFormatter<EdgeAgentResourceType>.GetValue(resourceType),
                ResourceId = resourceId,
                RevokedAtUtc = revokedAtUtc,
                ConnectionStatus = EnumFormatter<EdgeAgentConnectionStatus>.GetValue(EdgeAgentConnectionStatus.Revoked)
            },
            transaction: tx());
    }
}
