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
                   RevokedAtUtc, CreatedAtUtc, UpdatedAtUtc, Profile, ClusterId, DockerNodeId, DockerDaemonId,
                   DockerHostname, SwarmRole, LastObservedServiceId, LastObservedTaskId,
                   FirstEnrolledAtUtc, LastAuthenticatedAtUtc, RevocationReason
            FROM EdgeAgentBindings
            WHERE ResourceType = @ResourceType
              AND ResourceId = @ResourceId
              AND DockerNodeId IS NULL
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
                ,e.Profile AS BindingProfile
                ,e.ClusterId AS BindingClusterId
                ,e.DockerNodeId AS BindingDockerNodeId
                ,e.DockerDaemonId AS BindingDockerDaemonId
                ,e.DockerHostname AS BindingDockerHostname
                ,e.SwarmRole AS BindingSwarmRole
                ,e.LastObservedServiceId AS BindingLastObservedServiceId
                ,e.LastObservedTaskId AS BindingLastObservedTaskId
                ,e.FirstEnrolledAtUtc AS BindingFirstEnrolledAtUtc
                ,e.LastAuthenticatedAtUtc AS BindingLastAuthenticatedAtUtc
                ,e.RevocationReason AS BindingRevocationReason
            FROM Platforms p
            LEFT JOIN EdgeAgentBindings e
              ON e.ResourceType = 'Platform'
             AND e.ResourceId = p.Id
             AND e.DockerNodeId IS NULL
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
                   RevokedAtUtc, CreatedAtUtc, UpdatedAtUtc, Profile, ClusterId, DockerNodeId, DockerDaemonId,
                   DockerHostname, SwarmRole, LastObservedServiceId, LastObservedTaskId,
                   FirstEnrolledAtUtc, LastAuthenticatedAtUtc, RevocationReason
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
                RevokedAtUtc, CreatedAtUtc, UpdatedAtUtc, Profile, ClusterId, DockerNodeId, DockerDaemonId,
                DockerHostname, SwarmRole, LastObservedServiceId, LastObservedTaskId,
                FirstEnrolledAtUtc, LastAuthenticatedAtUtc, RevocationReason)
            VALUES (
                @Id, @PlatformId, @ResourceType, @ResourceId, @AgentId, @AgentPublicKey, @AgentFingerprint, @ConnectionStatus,
                @LastConnectedAtUtc, @LastDisconnectedAtUtc, @LastHeartbeatAtUtc,
                @LastSeenVersion, @LastSeenHostname, @CapabilitiesJson::json, @ProtocolVersion,
                @RevokedAtUtc, @CreatedAtUtc, @UpdatedAtUtc, @Profile, @ClusterId, @DockerNodeId, @DockerDaemonId,
                @DockerHostname, @SwarmRole, @LastObservedServiceId, @LastObservedTaskId,
                @FirstEnrolledAtUtc, @LastAuthenticatedAtUtc, @RevocationReason)
            ON CONFLICT DO NOTHING
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
                binding.UpdatedAtUtc,
                Profile = EnumFormatter<EdgeAgentProfile>.GetValue(binding.Profile),
                binding.ClusterId,
                binding.DockerNodeId,
                binding.DockerDaemonId,
                binding.DockerHostname,
                binding.SwarmRole,
                binding.LastObservedServiceId,
                binding.LastObservedTaskId,
                binding.FirstEnrolledAtUtc,
                binding.LastAuthenticatedAtUtc,
                binding.RevocationReason
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
              AND DockerNodeId IS NULL
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
              AND DockerNodeId IS NULL
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
              AND DockerNodeId IS NULL
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
              AND DockerNodeId IS NULL
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

    public async Task<EdgeAgentBinding?> GetNodeBindingAsync(Guid platformId, string dockerNodeId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, PlatformId, ResourceType, ResourceId, AgentId, AgentPublicKey, AgentFingerprint, ConnectionStatus,
                   LastConnectedAtUtc, LastDisconnectedAtUtc, LastHeartbeatAtUtc,
                   LastSeenVersion, LastSeenHostname, CapabilitiesJson, ProtocolVersion,
                   RevokedAtUtc, CreatedAtUtc, UpdatedAtUtc, Profile, ClusterId, DockerNodeId, DockerDaemonId,
                   DockerHostname, SwarmRole, LastObservedServiceId, LastObservedTaskId,
                   FirstEnrolledAtUtc, LastAuthenticatedAtUtc, RevocationReason
            FROM EdgeAgentBindings
            WHERE ResourceType = 'Platform'
              AND ResourceId = @PlatformId
              AND DockerNodeId = @DockerNodeId
              AND RevokedAtUtc IS NULL
            LIMIT 1
        """;

        var dto = await db.QuerySingleOrDefaultAsync<EdgeAgentBindingDto>(
            sql,
            new { PlatformId = platformId, DockerNodeId = dockerNodeId },
            transaction: tx());
        return dto?.ToDomain();
    }

    public async Task<IEnumerable<EdgeAgentBinding>> GetNodeBindingsAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, PlatformId, ResourceType, ResourceId, AgentId, AgentPublicKey, AgentFingerprint, ConnectionStatus,
                   LastConnectedAtUtc, LastDisconnectedAtUtc, LastHeartbeatAtUtc,
                   LastSeenVersion, LastSeenHostname, CapabilitiesJson, ProtocolVersion,
                   RevokedAtUtc, CreatedAtUtc, UpdatedAtUtc, Profile, ClusterId, DockerNodeId, DockerDaemonId,
                   DockerHostname, SwarmRole, LastObservedServiceId, LastObservedTaskId,
                   FirstEnrolledAtUtc, LastAuthenticatedAtUtc, RevocationReason
            FROM EdgeAgentBindings
            WHERE ResourceType = 'Platform'
              AND ResourceId = @PlatformId
              AND DockerNodeId IS NOT NULL
            ORDER BY DockerNodeId
        """;

        var dtos = await db.QueryAsync<EdgeAgentBindingDto>(sql, new { PlatformId = platformId }, transaction: tx());
        return dtos.Select(static dto => dto.ToDomain());
    }

    public async Task<EdgeAgentBinding?> GetActiveBindingByDockerDaemonIdAsync(string dockerDaemonId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, PlatformId, ResourceType, ResourceId, AgentId, AgentPublicKey, AgentFingerprint, ConnectionStatus,
                   LastConnectedAtUtc, LastDisconnectedAtUtc, LastHeartbeatAtUtc,
                   LastSeenVersion, LastSeenHostname, CapabilitiesJson, ProtocolVersion,
                   RevokedAtUtc, CreatedAtUtc, UpdatedAtUtc, Profile, ClusterId, DockerNodeId, DockerDaemonId,
                   DockerHostname, SwarmRole, LastObservedServiceId, LastObservedTaskId,
                   FirstEnrolledAtUtc, LastAuthenticatedAtUtc, RevocationReason
            FROM EdgeAgentBindings
            WHERE DockerDaemonId = @DockerDaemonId
              AND RevokedAtUtc IS NULL
            LIMIT 1
        """;

        var dto = await db.QuerySingleOrDefaultAsync<EdgeAgentBindingDto>(
            sql,
            new { DockerDaemonId = dockerDaemonId },
            transaction: tx());
        return dto?.ToDomain();
    }

    public Task<int> UpdateNodeBindingConnectedAsync(
        Guid platformId,
        string dockerNodeId,
        DateTime connectedAtUtc,
        string hostname,
        string agentVersion,
        string capabilitiesJson,
        string? serviceId,
        string? taskId,
        CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE EdgeAgentBindings
            SET ConnectionStatus = @ConnectionStatus,
                LastConnectedAtUtc = @ConnectedAtUtc,
                LastAuthenticatedAtUtc = @ConnectedAtUtc,
                LastDisconnectedAtUtc = NULL,
                LastHeartbeatAtUtc = @ConnectedAtUtc,
                LastSeenHostname = @Hostname,
                LastSeenVersion = @AgentVersion,
                CapabilitiesJson = @CapabilitiesJson::json,
                LastObservedServiceId = COALESCE(@ServiceId, LastObservedServiceId),
                LastObservedTaskId = COALESCE(@TaskId, LastObservedTaskId),
                UpdatedAtUtc = @ConnectedAtUtc
            WHERE ResourceType = 'Platform'
              AND ResourceId = @PlatformId
              AND DockerNodeId = @DockerNodeId
              AND RevokedAtUtc IS NULL
        """;

        return db.ExecuteAsync(sql, new
        {
            PlatformId = platformId,
            DockerNodeId = dockerNodeId,
            ConnectedAtUtc = connectedAtUtc,
            Hostname = hostname,
            AgentVersion = agentVersion,
            CapabilitiesJson = capabilitiesJson,
            ServiceId = serviceId,
            TaskId = taskId,
            ConnectionStatus = EnumFormatter<EdgeAgentConnectionStatus>.GetValue(EdgeAgentConnectionStatus.Connected)
        }, transaction: tx());
    }

    public Task<int> UpdateNodeBindingHeartbeatAsync(
        Guid platformId,
        string dockerNodeId,
        DateTime heartbeatAtUtc,
        string? hostname,
        string? agentVersion,
        string? capabilitiesJson,
        string? serviceId,
        string? taskId,
        CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE EdgeAgentBindings
            SET LastHeartbeatAtUtc = @HeartbeatAtUtc,
                LastSeenHostname = COALESCE(@Hostname, LastSeenHostname),
                LastSeenVersion = COALESCE(@AgentVersion, LastSeenVersion),
                CapabilitiesJson = COALESCE(@CapabilitiesJson::json, CapabilitiesJson),
                LastObservedServiceId = COALESCE(@ServiceId, LastObservedServiceId),
                LastObservedTaskId = COALESCE(@TaskId, LastObservedTaskId),
                UpdatedAtUtc = @HeartbeatAtUtc
            WHERE ResourceType = 'Platform'
              AND ResourceId = @PlatformId
              AND DockerNodeId = @DockerNodeId
              AND RevokedAtUtc IS NULL
        """;

        return db.ExecuteAsync(sql, new
        {
            PlatformId = platformId,
            DockerNodeId = dockerNodeId,
            HeartbeatAtUtc = heartbeatAtUtc,
            Hostname = hostname,
            AgentVersion = agentVersion,
            CapabilitiesJson = capabilitiesJson,
            ServiceId = serviceId,
            TaskId = taskId
        }, transaction: tx());
    }

    public Task<int> UpdateNodeBindingDisconnectedAsync(Guid platformId, string dockerNodeId, DateTime disconnectedAtUtc, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE EdgeAgentBindings
            SET ConnectionStatus = @ConnectionStatus,
                LastDisconnectedAtUtc = @DisconnectedAtUtc,
                UpdatedAtUtc = @DisconnectedAtUtc
            WHERE ResourceType = 'Platform'
              AND ResourceId = @PlatformId
              AND DockerNodeId = @DockerNodeId
              AND RevokedAtUtc IS NULL
        """;

        return db.ExecuteAsync(sql, new
        {
            PlatformId = platformId,
            DockerNodeId = dockerNodeId,
            DisconnectedAtUtc = disconnectedAtUtc,
            ConnectionStatus = EnumFormatter<EdgeAgentConnectionStatus>.GetValue(EdgeAgentConnectionStatus.Offline)
        }, transaction: tx());
    }

    public Task<int> RevokeNodeBindingAsync(Guid platformId, string dockerNodeId, DateTime revokedAtUtc, string reason, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE EdgeAgentBindings
            SET ConnectionStatus = @ConnectionStatus,
                RevokedAtUtc = @RevokedAtUtc,
                RevocationReason = @Reason,
                LastDisconnectedAtUtc = @RevokedAtUtc,
                UpdatedAtUtc = @RevokedAtUtc
            WHERE ResourceType = 'Platform'
              AND ResourceId = @PlatformId
              AND DockerNodeId = @DockerNodeId
              AND RevokedAtUtc IS NULL
        """;

        return db.ExecuteAsync(sql, new
        {
            PlatformId = platformId,
            DockerNodeId = dockerNodeId,
            RevokedAtUtc = revokedAtUtc,
            Reason = reason,
            ConnectionStatus = EnumFormatter<EdgeAgentConnectionStatus>.GetValue(EdgeAgentConnectionStatus.Revoked)
        }, transaction: tx());
    }

    public Task<int> RebindNodeAsync(
        Guid bindingId,
        string previousDockerNodeId,
        string dockerNodeId,
        string dockerHostname,
        string swarmRole,
        string serviceId,
        string taskId,
        DateTime updatedAtUtc,
        CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE EdgeAgentBindings
            SET DockerNodeId = @DockerNodeId,
                DockerHostname = @DockerHostname,
                SwarmRole = @SwarmRole,
                LastObservedServiceId = @ServiceId,
                LastObservedTaskId = @TaskId,
                ConnectionStatus = @ConnectionStatus,
                LastDisconnectedAtUtc = NULL,
                UpdatedAtUtc = @UpdatedAtUtc
            WHERE Id = @BindingId
              AND DockerNodeId = @PreviousDockerNodeId
              AND RevokedAtUtc IS NULL
        """;

        return db.ExecuteAsync(sql, new
        {
            BindingId = bindingId,
            PreviousDockerNodeId = previousDockerNodeId,
            DockerNodeId = dockerNodeId,
            DockerHostname = dockerHostname,
            SwarmRole = swarmRole,
            ServiceId = serviceId,
            TaskId = taskId,
            UpdatedAtUtc = updatedAtUtc,
            ConnectionStatus = EnumFormatter<EdgeAgentConnectionStatus>.GetValue(EdgeAgentConnectionStatus.Offline)
        }, transaction: tx());
    }

    public async Task<SwarmNodeAgentInstallation?> GetNodeAgentInstallationAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT PlatformId, ClusterId, ManagerDockerNodeId, ManagerDockerDaemonId,
                   DockerServiceId, DockerServiceName, AgentImageReference, AgentImageDigest,
                   DockerCaConfigId, DockerCaConfigName, DesiredState, OperationId, OperationKind,
                   OperationState, OperationStartedAtUtc, OperationActorId, OperationError,
                   CreatedAtUtc, UpdatedAtUtc
            FROM SwarmNodeAgentInstallations
            WHERE PlatformId = @PlatformId
        """;

        var dto = await db.QuerySingleOrDefaultAsync<SwarmNodeAgentInstallationDto>(
            sql,
            new { PlatformId = platformId },
            transaction: tx());
        return dto?.ToDomain();
    }

    public Task<int> UpsertNodeAgentInstallationAsync(SwarmNodeAgentInstallation installation, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO SwarmNodeAgentInstallations (
                PlatformId, ClusterId, ManagerDockerNodeId, ManagerDockerDaemonId,
                DockerServiceId, DockerServiceName, AgentImageReference, AgentImageDigest,
                DockerCaConfigId, DockerCaConfigName, DesiredState, OperationId, OperationKind,
                OperationState, OperationStartedAtUtc, OperationActorId, OperationError,
                CreatedAtUtc, UpdatedAtUtc)
            VALUES (
                @PlatformId, @ClusterId, @ManagerDockerNodeId, @ManagerDockerDaemonId,
                @DockerServiceId, @DockerServiceName, @AgentImageReference, @AgentImageDigest,
                @DockerCaConfigId, @DockerCaConfigName, @DesiredState, @OperationId, @OperationKind,
                @OperationState, @OperationStartedAtUtc, @OperationActorId, @OperationError,
                @CreatedAtUtc, @UpdatedAtUtc)
            ON CONFLICT (PlatformId) DO UPDATE SET
                ClusterId = EXCLUDED.ClusterId,
                ManagerDockerNodeId = EXCLUDED.ManagerDockerNodeId,
                ManagerDockerDaemonId = EXCLUDED.ManagerDockerDaemonId,
                DockerServiceId = EXCLUDED.DockerServiceId,
                DockerServiceName = EXCLUDED.DockerServiceName,
                AgentImageReference = EXCLUDED.AgentImageReference,
                AgentImageDigest = EXCLUDED.AgentImageDigest,
                DockerCaConfigId = EXCLUDED.DockerCaConfigId,
                DockerCaConfigName = EXCLUDED.DockerCaConfigName,
                DesiredState = EXCLUDED.DesiredState,
                OperationId = EXCLUDED.OperationId,
                OperationKind = EXCLUDED.OperationKind,
                OperationState = EXCLUDED.OperationState,
                OperationStartedAtUtc = EXCLUDED.OperationStartedAtUtc,
                OperationActorId = EXCLUDED.OperationActorId,
                OperationError = EXCLUDED.OperationError,
                UpdatedAtUtc = EXCLUDED.UpdatedAtUtc
        """;

        return db.ExecuteAsync(sql, new
        {
            installation.PlatformId,
            installation.ClusterId,
            installation.ManagerDockerNodeId,
            installation.ManagerDockerDaemonId,
            installation.DockerServiceId,
            installation.DockerServiceName,
            installation.AgentImageReference,
            installation.AgentImageDigest,
            installation.DockerCaConfigId,
            installation.DockerCaConfigName,
            DesiredState = EnumFormatter<SwarmNodeAgentDesiredState>.GetValue(installation.DesiredState),
            installation.OperationId,
            OperationKind = installation.OperationKind is null ? null : EnumFormatter<SwarmNodeAgentOperationKind>.GetValue(installation.OperationKind.Value),
            OperationState = installation.OperationState is null ? null : EnumFormatter<SwarmNodeAgentOperationState>.GetValue(installation.OperationState.Value),
            installation.OperationStartedAtUtc,
            installation.OperationActorId,
            installation.OperationError,
            installation.CreatedAtUtc,
            installation.UpdatedAtUtc
        }, transaction: tx());
    }

    public Task<int> TryStartNodeAgentOperationAsync(SwarmNodeAgentInstallation installation, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO SwarmNodeAgentInstallations (
                PlatformId, ClusterId, ManagerDockerNodeId, ManagerDockerDaemonId,
                DockerServiceId, DockerServiceName, AgentImageReference, AgentImageDigest,
                DockerCaConfigId, DockerCaConfigName, DesiredState, OperationId, OperationKind,
                OperationState, OperationStartedAtUtc, OperationActorId, OperationError,
                CreatedAtUtc, UpdatedAtUtc)
            VALUES (
                @PlatformId, @ClusterId, @ManagerDockerNodeId, @ManagerDockerDaemonId,
                @DockerServiceId, @DockerServiceName, @AgentImageReference, @AgentImageDigest,
                @DockerCaConfigId, @DockerCaConfigName, @DesiredState, @OperationId, @OperationKind,
                @OperationState, @OperationStartedAtUtc, @OperationActorId, @OperationError,
                @CreatedAtUtc, @UpdatedAtUtc)
            ON CONFLICT (PlatformId) DO UPDATE SET
                ClusterId = EXCLUDED.ClusterId,
                ManagerDockerNodeId = EXCLUDED.ManagerDockerNodeId,
                ManagerDockerDaemonId = EXCLUDED.ManagerDockerDaemonId,
                DesiredState = EXCLUDED.DesiredState,
                OperationId = EXCLUDED.OperationId,
                OperationKind = EXCLUDED.OperationKind,
                OperationState = EXCLUDED.OperationState,
                OperationStartedAtUtc = EXCLUDED.OperationStartedAtUtc,
                OperationActorId = EXCLUDED.OperationActorId,
                OperationError = NULL,
                UpdatedAtUtc = EXCLUDED.UpdatedAtUtc
            WHERE SwarmNodeAgentInstallations.OperationState IS DISTINCT FROM 'Running'
               OR SwarmNodeAgentInstallations.OperationStartedAtUtc IS NULL
               OR SwarmNodeAgentInstallations.OperationStartedAtUtc < EXCLUDED.OperationStartedAtUtc - INTERVAL '30 minutes'
        """;

        return db.ExecuteAsync(sql, new
        {
            installation.PlatformId,
            installation.ClusterId,
            installation.ManagerDockerNodeId,
            installation.ManagerDockerDaemonId,
            installation.DockerServiceId,
            installation.DockerServiceName,
            installation.AgentImageReference,
            installation.AgentImageDigest,
            installation.DockerCaConfigId,
            installation.DockerCaConfigName,
            DesiredState = EnumFormatter<SwarmNodeAgentDesiredState>.GetValue(installation.DesiredState),
            installation.OperationId,
            OperationKind = installation.OperationKind is null ? null : EnumFormatter<SwarmNodeAgentOperationKind>.GetValue(installation.OperationKind.Value),
            OperationState = installation.OperationState is null ? null : EnumFormatter<SwarmNodeAgentOperationState>.GetValue(installation.OperationState.Value),
            installation.OperationStartedAtUtc,
            installation.OperationActorId,
            installation.OperationError,
            installation.CreatedAtUtc,
            installation.UpdatedAtUtc
        }, transaction: tx());
    }

    public Task<int> AddNodeAgentBootstrapAsync(SwarmNodeAgentBootstrap bootstrap, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO SwarmNodeAgentBootstraps (
                Id, PlatformId, ClusterId, Version, TokenHash, DockerSecretId, DockerSecretName,
                ExpiresAtUtc, RevokedAtUtc, CreatedByActorId, CreatedAtUtc, UpdatedAtUtc)
            VALUES (
                @Id, @PlatformId, @ClusterId, @Version, @TokenHash, @DockerSecretId, @DockerSecretName,
                @ExpiresAtUtc, @RevokedAtUtc, @CreatedByActorId, @CreatedAtUtc, @UpdatedAtUtc)
        """;

        return db.ExecuteAsync(sql, bootstrap, transaction: tx());
    }

    public async Task<SwarmNodeAgentBootstrap?> GetActiveNodeAgentBootstrapByTokenHashAsync(string tokenHash, DateTime utcNow, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, PlatformId, ClusterId, Version, TokenHash, DockerSecretId, DockerSecretName,
                   ExpiresAtUtc, RevokedAtUtc, CreatedByActorId, CreatedAtUtc, UpdatedAtUtc
            FROM SwarmNodeAgentBootstraps
            WHERE TokenHash = @TokenHash
              AND RevokedAtUtc IS NULL
              AND ExpiresAtUtc > @UtcNow
            LIMIT 1
        """;

        var dto = await db.QuerySingleOrDefaultAsync<SwarmNodeAgentBootstrapDto>(
            sql,
            new { TokenHash = tokenHash, UtcNow = utcNow },
            transaction: tx());
        return dto?.ToDomain();
    }

    public async Task<SwarmNodeAgentBootstrap?> GetLatestNodeAgentBootstrapAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, PlatformId, ClusterId, Version, TokenHash, DockerSecretId, DockerSecretName,
                   ExpiresAtUtc, RevokedAtUtc, CreatedByActorId, CreatedAtUtc, UpdatedAtUtc
            FROM SwarmNodeAgentBootstraps
            WHERE PlatformId = @PlatformId
            ORDER BY Version DESC
            LIMIT 1
        """;

        var dto = await db.QuerySingleOrDefaultAsync<SwarmNodeAgentBootstrapDto>(
            sql,
            new { PlatformId = platformId },
            transaction: tx());
        return dto?.ToDomain();
    }

    public async Task<IEnumerable<SwarmNodeAgentBootstrap>> GetNodeAgentBootstrapsAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, PlatformId, ClusterId, Version, TokenHash, DockerSecretId, DockerSecretName,
                   ExpiresAtUtc, RevokedAtUtc, CreatedByActorId, CreatedAtUtc, UpdatedAtUtc
            FROM SwarmNodeAgentBootstraps
            WHERE PlatformId = @PlatformId
            ORDER BY Version DESC
        """;

        var dtos = await db.QueryAsync<SwarmNodeAgentBootstrapDto>(
            sql,
            new { PlatformId = platformId },
            transaction: tx());
        return dtos.Select(static dto => dto.ToDomain());
    }

    public Task<int> UpdateNodeAgentBootstrapSecretAsync(Guid bootstrapId, string dockerSecretId, DateTime updatedAtUtc, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE SwarmNodeAgentBootstraps
            SET DockerSecretId = @DockerSecretId,
                UpdatedAtUtc = @UpdatedAtUtc
            WHERE Id = @BootstrapId
        """;

        return db.ExecuteAsync(sql, new
        {
            BootstrapId = bootstrapId,
            DockerSecretId = dockerSecretId,
            UpdatedAtUtc = updatedAtUtc
        }, transaction: tx());
    }

    public Task<int> RevokeNodeAgentBootstrapsAsync(Guid platformId, DateTime revokedAtUtc, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE SwarmNodeAgentBootstraps
            SET RevokedAtUtc = @RevokedAtUtc,
                UpdatedAtUtc = @RevokedAtUtc
            WHERE PlatformId = @PlatformId
              AND RevokedAtUtc IS NULL
        """;

        return db.ExecuteAsync(sql, new
        {
            PlatformId = platformId,
            RevokedAtUtc = revokedAtUtc
        }, transaction: tx());
    }

    public Task<int> RevokeOtherNodeAgentBootstrapsAsync(Guid platformId, Guid activeBootstrapId, DateTime revokedAtUtc, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE SwarmNodeAgentBootstraps
            SET RevokedAtUtc = @RevokedAtUtc,
                UpdatedAtUtc = @RevokedAtUtc
            WHERE PlatformId = @PlatformId
              AND Id <> @ActiveBootstrapId
              AND RevokedAtUtc IS NULL
        """;

        return db.ExecuteAsync(sql, new
        {
            PlatformId = platformId,
            ActiveBootstrapId = activeBootstrapId,
            RevokedAtUtc = revokedAtUtc
        }, transaction: tx());
    }
}
