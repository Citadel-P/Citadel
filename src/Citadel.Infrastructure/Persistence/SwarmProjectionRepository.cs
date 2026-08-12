using System.Data;
using System.Text.Json;
using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;

namespace Infrastructure.Persistence;

internal sealed class SwarmProjectionRepository(IDbConnection db, Func<IDbTransaction> tx)
    : ISwarmProjectionRepository
{
    public async Task<SwarmProjectionSummary> GetSummaryAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            WITH ServiceSummary AS (
                SELECT
                    COUNT(*)::int AS ServiceCount,
                    COUNT(*) FILTER (
                        WHERE NOT IsStale AND DesiredTaskCount > 0
                            AND RunningTaskCount >= DesiredTaskCount)::int AS HealthyServiceCount,
                    COUNT(*) FILTER (
                        WHERE NOT IsStale AND DesiredTaskCount > 0 AND RunningTaskCount > 0
                            AND RunningTaskCount < DesiredTaskCount)::int AS DegradedServiceCount,
                    COUNT(*) FILTER (
                        WHERE NOT IsStale AND DesiredTaskCount > 0
                            AND RunningTaskCount <= 0)::int AS FailedServiceCount,
                    COUNT(*) FILTER (
                        WHERE NOT IsStale AND DesiredTaskCount <= 0)::int AS StoppedServiceCount,
                    COUNT(*) FILTER (WHERE IsStale)::int AS UnknownServiceCount,
                    COALESCE(SUM(RunningTaskCount), 0)::int AS RunningTaskCount,
                    COALESCE(SUM(DesiredTaskCount), 0)::int AS DesiredTaskCount
                FROM SwarmServiceProjections
                WHERE PlatformId = @PlatformId AND Ownership <> 'System'
            )
            SELECT
                EXISTS (
                    SELECT 1 FROM SwarmNodeProjections WHERE PlatformId = @PlatformId AND IsStale
                    UNION ALL
                    SELECT 1 FROM SwarmServiceProjections WHERE PlatformId = @PlatformId AND IsStale
                    UNION ALL
                    SELECT 1 FROM SwarmTaskProjections WHERE PlatformId = @PlatformId AND IsStale
                    UNION ALL
                    SELECT 1 FROM SwarmNetworkProjections WHERE PlatformId = @PlatformId AND IsStale
                    UNION ALL
                    SELECT 1 FROM SwarmSecretProjections WHERE PlatformId = @PlatformId AND IsStale
                    UNION ALL
                    SELECT 1 FROM SwarmConfigProjections WHERE PlatformId = @PlatformId AND IsStale
                    UNION ALL
                    SELECT 1 FROM SwarmNodeImageProjections WHERE PlatformId = @PlatformId AND IsStale
                    UNION ALL
                    SELECT 1 FROM SwarmNodeVolumeProjections WHERE PlatformId = @PlatformId AND IsStale
                    UNION ALL
                    SELECT 1 FROM SwarmNodeNetworkProjections WHERE PlatformId = @PlatformId AND IsStale
                ) AS IsStale,
                (SELECT COUNT(*)::int FROM SwarmNodeProjections WHERE PlatformId = @PlatformId) AS NodeCount,
                (SELECT COUNT(*)::int FROM SwarmNodeProjections
                    WHERE PlatformId = @PlatformId AND lower(Role) = 'manager') AS ManagerCount,
                (SELECT COUNT(*)::int FROM SwarmNodeProjections
                    WHERE PlatformId = @PlatformId AND lower(Role) = 'manager'
                        AND NOT IsStale AND lower(Reachability) = 'reachable') AS ReachableManagerCount,
                EXISTS (SELECT 1 FROM SwarmNodeProjections
                    WHERE PlatformId = @PlatformId AND lower(Role) = 'manager'
                        AND NOT IsStale AND IsLeader) AS HasLeader,
                EXISTS (SELECT 1 FROM SwarmNodeProjections
                    WHERE PlatformId = @PlatformId AND lower(Role) = 'manager'
                        AND IsStale) AS IsManagerInventoryStale,
                ServiceCount,
                HealthyServiceCount,
                DegradedServiceCount,
                FailedServiceCount,
                StoppedServiceCount,
                UnknownServiceCount,
                RunningTaskCount,
                DesiredTaskCount,
                ((SELECT COUNT(*) FROM SwarmNetworkProjections WHERE PlatformId = @PlatformId)
                    + (SELECT COUNT(*) FROM SwarmNodeNetworkProjections WHERE PlatformId = @PlatformId))::int AS NetworkCount,
                (SELECT COUNT(*)::int FROM SwarmNodeNetworkProjections WHERE PlatformId = @PlatformId) AS LocalNetworkCount,
                (SELECT COUNT(*)::int FROM SwarmNodeVolumeProjections WHERE PlatformId = @PlatformId) AS VolumeCount,
                (SELECT COUNT(*)::int FROM SwarmNodeImageProjections WHERE PlatformId = @PlatformId) AS ImageCount
            FROM ServiceSummary
            """;
        var value = await db.QuerySingleAsync<SwarmProjectionSummaryDto>(
            sql,
            new { PlatformId = platformId },
            transaction: tx());
        return new SwarmProjectionSummary(
            value.IsStale,
            value.NodeCount,
            value.ManagerCount,
            value.ReachableManagerCount,
            value.HasLeader,
            value.IsManagerInventoryStale,
            value.ServiceCount,
            new PlatformWorkloadStatusCounts(
                value.ServiceCount,
                value.HealthyServiceCount,
                value.DegradedServiceCount,
                value.FailedServiceCount,
                value.StoppedServiceCount,
                Paused: 0,
                InProgress: 0,
                Unknown: value.UnknownServiceCount),
            value.RunningTaskCount,
            value.DesiredTaskCount,
            value.NetworkCount,
            value.LocalNetworkCount,
            value.VolumeCount,
            value.ImageCount);
    }

    public async Task<IReadOnlyList<SwarmNodeProjection>> GetNodesAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT PlatformId, DockerNodeId, VersionIndex, Hostname, Role, IsLeader,
                   Reachability, Status, StatusMessage, Availability, EngineVersion,
                   OperatingSystem, Architecture, Address, Labels, RunningTaskCount,
                   DesiredTaskCount, DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale
            FROM SwarmNodeProjections WHERE PlatformId = @PlatformId
            ORDER BY Hostname, DockerNodeId
            """;
        return (await db.QueryAsync<SwarmNodeProjectionDto>(sql, new { PlatformId = platformId }, transaction: tx()))
            .Select(static value => value.ToDomain()).ToArray();
    }

    public async Task<SwarmNodeProjection?> GetNodeAsync(Guid platformId, string dockerNodeId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT PlatformId, DockerNodeId, VersionIndex, Hostname, Role, IsLeader,
                   Reachability, Status, StatusMessage, Availability, EngineVersion,
                   OperatingSystem, Architecture, Address, Labels, RunningTaskCount,
                   DesiredTaskCount, DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale
            FROM SwarmNodeProjections WHERE PlatformId = @PlatformId AND DockerNodeId = @DockerNodeId
            """;
        var row = await db.QuerySingleOrDefaultAsync<SwarmNodeProjectionDto>(sql, new { PlatformId = platformId, DockerNodeId = dockerNodeId }, transaction: tx());
        return row?.ToDomain();
    }

    public async Task<IReadOnlyList<SwarmServiceProjection>> GetServicesAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT PlatformId, DockerServiceId, VersionIndex, Name, Mode, Image, RunningTaskCount,
                   DesiredTaskCount, UpdateState, UpdateMessage, Ports, NetworkIds, SecretIds,
                   ConfigIds, Labels, Ownership, DockerStackNamespace,
                   OwnershipDiagnostic, SwarmServiceId, StackId, LiveRuntimeHash, ForceUpdate,
                   DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale
            FROM SwarmServiceProjections
            WHERE PlatformId = @PlatformId AND Ownership <> 'System'
            ORDER BY Name, DockerServiceId
            """;
        return (await db.QueryAsync<SwarmServiceProjectionDto>(sql, new { PlatformId = platformId }, transaction: tx()))
            .Select(static value => value.ToDomain()).ToArray();
    }

    public async Task<SwarmServiceProjection?> GetServiceAsync(Guid platformId, string dockerServiceId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT PlatformId, DockerServiceId, VersionIndex, Name, Mode, Image, RunningTaskCount,
                   DesiredTaskCount, UpdateState, UpdateMessage, Ports, NetworkIds, SecretIds,
                   ConfigIds, Labels, Ownership, DockerStackNamespace,
                   OwnershipDiagnostic, SwarmServiceId, StackId, LiveRuntimeHash, ForceUpdate,
                   DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale
            FROM SwarmServiceProjections WHERE PlatformId = @PlatformId AND DockerServiceId = @DockerServiceId
            """;
        var row = await db.QuerySingleOrDefaultAsync<SwarmServiceProjectionDto>(sql, new { PlatformId = platformId, DockerServiceId = dockerServiceId }, transaction: tx());
        return row?.ToDomain();
    }

    public async Task<SwarmServiceProjection?> GetServiceByNameAsync(
        Guid platformId,
        string name,
        CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT PlatformId, DockerServiceId, VersionIndex, Name, Mode, Image, RunningTaskCount,
                   DesiredTaskCount, UpdateState, UpdateMessage, Ports, NetworkIds, SecretIds,
                   ConfigIds, Labels, Ownership, DockerStackNamespace,
                   OwnershipDiagnostic, SwarmServiceId, StackId, LiveRuntimeHash, ForceUpdate,
                   DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale
            FROM SwarmServiceProjections WHERE PlatformId = @PlatformId AND Name = @Name
            """;
        var row = await db.QuerySingleOrDefaultAsync<SwarmServiceProjectionDto>(
            sql,
            new { PlatformId = platformId, Name = name },
            transaction: tx());
        return row?.ToDomain();
    }

    public Task<int> TryAssignStackNamespaceAsync(
        Guid platformId,
        string stackNamespace,
        IReadOnlyCollection<string> dockerServiceIds,
        Guid stackId,
        CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE SwarmServiceProjections
            SET StackId = @StackId,
                Ownership = 'CitadelStack',
                OwnershipDiagnostic = NULL
            WHERE PlatformId = @PlatformId
              AND DockerStackNamespace = @Namespace
              AND DockerServiceId = ANY(@DockerServiceIds)
              AND StackId IS NULL
              AND Ownership = 'DockerStackExternal'
            """;

        return db.ExecuteAsync(sql, new
        {
            PlatformId = platformId,
            Namespace = stackNamespace,
            DockerServiceIds = dockerServiceIds.ToArray(),
            StackId = stackId
        }, transaction: tx());
    }

    public async Task<IReadOnlyList<SwarmTaskProjection>> GetTasksAsync(
        Guid platformId,
        int limit,
        CancellationToken cancellationToken,
        string? dockerServiceId = null)
    {
        const string sql = """
            SELECT PlatformId, DockerTaskId, VersionIndex, Name, DockerServiceId, ServiceName, Slot,
                   DockerNodeId, NodeHostname, DesiredState, State, StatusMessage, Error, Image, Ports,
                   StatusTimestamp, DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale, DockerContainerId
            FROM SwarmTaskProjections
            WHERE PlatformId = @PlatformId
              AND (@DockerServiceId IS NULL OR DockerServiceId = @DockerServiceId)
              AND (@DockerServiceId IS NOT NULL OR NOT EXISTS (
                    SELECT 1 FROM SwarmServiceProjections service
                    WHERE service.PlatformId = @PlatformId
                      AND service.DockerServiceId = SwarmTaskProjections.DockerServiceId
                      AND service.Ownership = 'System'))
            ORDER BY StatusTimestamp DESC NULLS LAST, DockerTaskId LIMIT @Limit
            """;
        return (await db.QueryAsync<SwarmTaskProjectionDto>(sql, new
        {
            PlatformId = platformId,
            Limit = limit,
            DockerServiceId = dockerServiceId
        }, transaction: tx()))
            .Select(static value => value.ToDomain()).ToArray();
    }

    public async Task<SwarmTaskProjection?> GetTaskAsync(Guid platformId, string dockerTaskId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT PlatformId, DockerTaskId, VersionIndex, Name, DockerServiceId, ServiceName, Slot,
                   DockerNodeId, NodeHostname, DesiredState, State, StatusMessage, Error, Image, Ports,
                   StatusTimestamp, DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale, DockerContainerId
            FROM SwarmTaskProjections WHERE PlatformId = @PlatformId AND DockerTaskId = @DockerTaskId
            """;
        var row = await db.QuerySingleOrDefaultAsync<SwarmTaskProjectionDto>(sql, new { PlatformId = platformId, DockerTaskId = dockerTaskId }, transaction: tx());
        return row?.ToDomain();
    }

    public async Task<IReadOnlyList<SwarmNetworkProjection>> GetNetworksAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT PlatformId, DockerNetworkId, Name, Scope, Driver, IsAttachable, IsInternal,
                   IsIngress, IsEncrypted, EnableIPv6, Subnets, ServiceNames, Labels, DockerCreatedAt, ObservedAt, IsStale
            FROM SwarmNetworkProjections WHERE PlatformId = @PlatformId ORDER BY Name, DockerNetworkId
            """;
        return (await db.QueryAsync<SwarmNetworkProjectionDto>(sql, new { PlatformId = platformId }, transaction: tx()))
            .Select(static value => value.ToDomain()).ToArray();
    }

    public async Task<SwarmNetworkProjection?> GetNetworkAsync(Guid platformId, string dockerNetworkId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT PlatformId, DockerNetworkId, Name, Scope, Driver, IsAttachable, IsInternal,
                   IsIngress, IsEncrypted, EnableIPv6, Subnets, ServiceNames, Labels, DockerCreatedAt, ObservedAt, IsStale
            FROM SwarmNetworkProjections WHERE PlatformId = @PlatformId AND DockerNetworkId = @DockerNetworkId
            """;
        var row = await db.QuerySingleOrDefaultAsync<SwarmNetworkProjectionDto>(sql, new { PlatformId = platformId, DockerNetworkId = dockerNetworkId }, transaction: tx());
        return row?.ToDomain();
    }

    public async Task<IReadOnlyList<SwarmSecretProjection>> GetSecretsAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT PlatformId, DockerSecretId, VersionIndex, Name, Driver, ServiceNames, Labels, DockerCreatedAt,
                   DockerUpdatedAt, ObservedAt, IsStale
            FROM SwarmSecretProjections WHERE PlatformId = @PlatformId ORDER BY Name, DockerSecretId
            """;
        return (await db.QueryAsync<SwarmSecretProjectionDto>(sql, new { PlatformId = platformId }, transaction: tx()))
            .Select(static value => value.ToDomain()).ToArray();
    }

    public async Task<SwarmSecretProjection?> GetSecretAsync(Guid platformId, string dockerSecretId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT PlatformId, DockerSecretId, VersionIndex, Name, Driver, ServiceNames, Labels, DockerCreatedAt,
                   DockerUpdatedAt, ObservedAt, IsStale
            FROM SwarmSecretProjections WHERE PlatformId = @PlatformId AND DockerSecretId = @DockerSecretId
            """;
        var row = await db.QuerySingleOrDefaultAsync<SwarmSecretProjectionDto>(sql, new { PlatformId = platformId, DockerSecretId = dockerSecretId }, transaction: tx());
        return row?.ToDomain();
    }

    public async Task<IReadOnlyList<SwarmConfigProjection>> GetConfigsAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT PlatformId, DockerConfigId, VersionIndex, Name, TemplatingDriver, ServiceNames, Labels,
                   DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale
            FROM SwarmConfigProjections WHERE PlatformId = @PlatformId ORDER BY Name, DockerConfigId
            """;
        return (await db.QueryAsync<SwarmConfigProjectionDto>(sql, new { PlatformId = platformId }, transaction: tx()))
            .Select(static value => value.ToDomain()).ToArray();
    }

    public async Task<SwarmConfigProjection?> GetConfigAsync(Guid platformId, string dockerConfigId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT PlatformId, DockerConfigId, VersionIndex, Name, TemplatingDriver, ServiceNames, Labels,
                   DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale
            FROM SwarmConfigProjections WHERE PlatformId = @PlatformId AND DockerConfigId = @DockerConfigId
            """;
        var row = await db.QuerySingleOrDefaultAsync<SwarmConfigProjectionDto>(sql, new { PlatformId = platformId, DockerConfigId = dockerConfigId }, transaction: tx());
        return row?.ToDomain();
    }

    public async Task<IReadOnlyList<SwarmNodeRuntimeProjectionState>> GetNodeRuntimeStatesAsync(
        Guid platformId,
        CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT PlatformId, DockerNodeId, ReconciliationGeneration, ReconciliationStartedAt,
                   ReconciliationCompletedAt, LastSuccessfulReconciliationAt, IsStale, StaleSince,
                   StaleReason, LastEventStreamConnectedAt, LastEventGapAt, LastStatsSampleAt,
                   AgentVersion, DockerVersion
            FROM SwarmNodeRuntimeProjectionStates
            WHERE PlatformId = @PlatformId
            ORDER BY DockerNodeId
            """;
        return (await db.QueryAsync<SwarmNodeRuntimeProjectionStateDto>(
                sql,
                new { PlatformId = platformId },
                transaction: tx()))
            .Select(static value => value.ToDomain())
            .ToArray();
    }

    public async Task<SwarmNodeRuntimeProjectionState?> GetNodeRuntimeStateAsync(
        Guid platformId,
        string dockerNodeId,
        CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT PlatformId, DockerNodeId, ReconciliationGeneration, ReconciliationStartedAt,
                   ReconciliationCompletedAt, LastSuccessfulReconciliationAt, IsStale, StaleSince,
                   StaleReason, LastEventStreamConnectedAt, LastEventGapAt, LastStatsSampleAt,
                   AgentVersion, DockerVersion
            FROM SwarmNodeRuntimeProjectionStates
            WHERE PlatformId = @PlatformId AND DockerNodeId = @DockerNodeId
            """;
        var row = await db.QuerySingleOrDefaultAsync<SwarmNodeRuntimeProjectionStateDto>(
            sql,
            new { PlatformId = platformId, DockerNodeId = dockerNodeId },
            transaction: tx());
        return row?.ToDomain();
    }

    public Task<int> UpsertNodeRuntimeStateAsync(
        SwarmNodeRuntimeProjectionState state,
        CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO SwarmNodeRuntimeProjectionStates (
                PlatformId, DockerNodeId, ReconciliationGeneration, ReconciliationStartedAt,
                ReconciliationCompletedAt, LastSuccessfulReconciliationAt, IsStale, StaleSince,
                StaleReason, LastEventStreamConnectedAt, LastEventGapAt, LastStatsSampleAt,
                AgentVersion, DockerVersion)
            VALUES (
                @PlatformId, @DockerNodeId, @ReconciliationGeneration, @ReconciliationStartedAt,
                @ReconciliationCompletedAt, @LastSuccessfulReconciliationAt, @IsStale, @StaleSince,
                @StaleReason, @LastEventStreamConnectedAt, @LastEventGapAt, @LastStatsSampleAt,
                @AgentVersion, @DockerVersion)
            ON CONFLICT (PlatformId, DockerNodeId) DO UPDATE SET
                ReconciliationGeneration = excluded.ReconciliationGeneration,
                ReconciliationStartedAt = excluded.ReconciliationStartedAt,
                ReconciliationCompletedAt = excluded.ReconciliationCompletedAt,
                LastSuccessfulReconciliationAt = excluded.LastSuccessfulReconciliationAt,
                IsStale = excluded.IsStale,
                StaleSince = excluded.StaleSince,
                StaleReason = excluded.StaleReason,
                LastEventStreamConnectedAt = excluded.LastEventStreamConnectedAt,
                LastEventGapAt = excluded.LastEventGapAt,
                LastStatsSampleAt = excluded.LastStatsSampleAt,
                AgentVersion = excluded.AgentVersion,
                DockerVersion = excluded.DockerVersion
            """;
        return db.ExecuteAsync(
            sql,
            new
            {
                state.PlatformId,
                state.DockerNodeId,
                state.ReconciliationGeneration,
                ReconciliationStartedAt = state.ReconciliationStartedAt?.UtcDateTime,
                ReconciliationCompletedAt = state.ReconciliationCompletedAt?.UtcDateTime,
                LastSuccessfulReconciliationAt = state.LastSuccessfulReconciliationAt?.UtcDateTime,
                state.IsStale,
                StaleSince = state.StaleSince?.UtcDateTime,
                state.StaleReason,
                LastEventStreamConnectedAt = state.LastEventStreamConnectedAt?.UtcDateTime,
                LastEventGapAt = state.LastEventGapAt?.UtcDateTime,
                LastStatsSampleAt = state.LastStatsSampleAt?.UtcDateTime,
                state.AgentVersion,
                state.DockerVersion
            },
            transaction: tx());
    }

    public async Task<IReadOnlyList<SwarmNodeImageProjection>> GetNodeImagesAsync(
        Guid platformId,
        CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT image.Id, image.PlatformId, image.DockerNodeId, image.DockerImageId,
                   image.ContentIdentity, image.Resource, image.ObservedAt, image.IsStale,
                   node.Hostname AS NodeHostname, state.StaleReason
            FROM SwarmNodeImageProjections image
            LEFT JOIN SwarmNodeProjections node
              ON node.PlatformId = image.PlatformId AND node.DockerNodeId = image.DockerNodeId
            LEFT JOIN SwarmNodeRuntimeProjectionStates state
              ON state.PlatformId = image.PlatformId AND state.DockerNodeId = image.DockerNodeId
            WHERE image.PlatformId = @PlatformId
            ORDER BY image.ContentIdentity, node.Hostname, image.DockerNodeId
            """;
        return (await db.QueryAsync<SwarmNodeImageProjectionDto>(
                sql,
                new { PlatformId = platformId },
                transaction: tx()))
            .Select(static value => value.ToDomain())
            .ToArray();
    }

    public async Task<IReadOnlyList<SwarmNodeVolumeProjection>> GetNodeVolumesAsync(
        Guid platformId,
        CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT volume.PlatformId, volume.DockerNodeId, volume.VolumeName, volume.Resource,
                   volume.ObservedAt, volume.IsStale, node.Hostname AS NodeHostname, state.StaleReason
            FROM SwarmNodeVolumeProjections volume
            LEFT JOIN SwarmNodeProjections node
              ON node.PlatformId = volume.PlatformId AND node.DockerNodeId = volume.DockerNodeId
            LEFT JOIN SwarmNodeRuntimeProjectionStates state
              ON state.PlatformId = volume.PlatformId AND state.DockerNodeId = volume.DockerNodeId
            WHERE volume.PlatformId = @PlatformId
            ORDER BY volume.VolumeName, node.Hostname, volume.DockerNodeId
            """;
        return (await db.QueryAsync<SwarmNodeVolumeProjectionDto>(
                sql,
                new { PlatformId = platformId },
                transaction: tx()))
            .Select(static value => value.ToDomain())
            .ToArray();
    }

    public async Task<IReadOnlyList<SwarmNodeNetworkProjection>> GetNodeNetworksAsync(
        Guid platformId,
        CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT network.PlatformId, network.DockerNodeId, network.DockerNetworkId, network.Resource,
                   network.ObservedAt, network.IsStale, node.Hostname AS NodeHostname, state.StaleReason
            FROM SwarmNodeNetworkProjections network
            LEFT JOIN SwarmNodeProjections node
              ON node.PlatformId = network.PlatformId AND node.DockerNodeId = network.DockerNodeId
            LEFT JOIN SwarmNodeRuntimeProjectionStates state
              ON state.PlatformId = network.PlatformId AND state.DockerNodeId = network.DockerNodeId
            WHERE network.PlatformId = @PlatformId
            ORDER BY (network.Resource->>'Name'), node.Hostname, network.DockerNodeId
            """;
        return (await db.QueryAsync<SwarmNodeNetworkProjectionDto>(
                sql,
                new { PlatformId = platformId },
                transaction: tx()))
            .Select(static value => value.ToDomain())
            .ToArray();
    }

    public async Task<int> ReplaceNodeLocalResourcesAsync(
        Guid platformId,
        string dockerNodeId,
        IReadOnlyList<SwarmNodeImageProjection> images,
        IReadOnlyList<SwarmNodeVolumeProjection> volumes,
        IReadOnlyList<SwarmNodeNetworkProjection> networks,
        DateTimeOffset snapshotStartedAt,
        CancellationToken cancellationToken)
    {
        var affected = await ReplaceNodeImagesAsync(platformId, dockerNodeId, images, snapshotStartedAt);
        affected += await ReplaceNodeVolumesAsync(platformId, dockerNodeId, volumes, snapshotStartedAt);
        affected += await ReplaceNodeNetworksAsync(platformId, dockerNodeId, networks, snapshotStartedAt);
        return affected;
    }

    public Task<int> MarkNodeLocalResourcesStaleAsync(
        Guid platformId,
        string dockerNodeId,
        CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE SwarmNodeImageProjections SET IsStale = true
            WHERE PlatformId = @PlatformId AND DockerNodeId = @DockerNodeId AND IsStale = false;
            UPDATE SwarmNodeVolumeProjections SET IsStale = true
            WHERE PlatformId = @PlatformId AND DockerNodeId = @DockerNodeId AND IsStale = false;
            UPDATE SwarmNodeNetworkProjections SET IsStale = true
            WHERE PlatformId = @PlatformId AND DockerNodeId = @DockerNodeId AND IsStale = false;
            """;
        return db.ExecuteAsync(sql, new { PlatformId = platformId, DockerNodeId = dockerNodeId }, transaction: tx());
    }

    private async Task<int> ReplaceNodeImagesAsync(
        Guid platformId,
        string dockerNodeId,
        IReadOnlyList<SwarmNodeImageProjection> values,
        DateTimeOffset snapshotStartedAt)
    {
        const string sql = """
            INSERT INTO SwarmNodeImageProjections (
                Id, PlatformId, DockerNodeId, DockerImageId, ContentIdentity, Resource, ObservedAt, IsStale)
            SELECT value."Id", @PlatformId, @DockerNodeId, value."DockerImageId",
                   value."ContentIdentity", value."Resource", value."ObservedAt", false
            FROM jsonb_to_recordset(@Rows::jsonb) AS value(
                "Id" uuid, "DockerImageId" text, "ContentIdentity" text,
                "Resource" jsonb, "ObservedAt" timestamptz)
            ON CONFLICT (PlatformId, DockerNodeId, DockerImageId) DO UPDATE SET
                ContentIdentity = excluded.ContentIdentity,
                Resource = excluded.Resource,
                ObservedAt = excluded.ObservedAt,
                IsStale = false
            WHERE SwarmNodeImageProjections.ObservedAt < @SnapshotStartedAt;
            DELETE FROM SwarmNodeImageProjections
            WHERE PlatformId = @PlatformId AND DockerNodeId = @DockerNodeId
              AND ObservedAt < @SnapshotStartedAt
              AND NOT (DockerImageId = ANY(@Ids));
            """;
        var rows = JsonSerializer.Serialize(values, PlatformJsonContext.Default.IReadOnlyListSwarmNodeImageProjection);
        return await db.ExecuteAsync(sql, new
        {
            PlatformId = platformId,
            DockerNodeId = dockerNodeId,
            Rows = rows,
            SnapshotStartedAt = snapshotStartedAt.UtcDateTime,
            Ids = values.Select(static value => value.DockerImageId).ToArray()
        }, transaction: tx());
    }

    private async Task<int> ReplaceNodeVolumesAsync(
        Guid platformId,
        string dockerNodeId,
        IReadOnlyList<SwarmNodeVolumeProjection> values,
        DateTimeOffset snapshotStartedAt)
    {
        const string sql = """
            INSERT INTO SwarmNodeVolumeProjections (
                PlatformId, DockerNodeId, VolumeName, Resource, ObservedAt, IsStale)
            SELECT @PlatformId, @DockerNodeId, value."VolumeName", value."Resource", value."ObservedAt", false
            FROM jsonb_to_recordset(@Rows::jsonb) AS value(
                "VolumeName" text, "Resource" jsonb, "ObservedAt" timestamptz)
            ON CONFLICT (PlatformId, DockerNodeId, VolumeName) DO UPDATE SET
                Resource = excluded.Resource,
                ObservedAt = excluded.ObservedAt,
                IsStale = false
            WHERE SwarmNodeVolumeProjections.ObservedAt < @SnapshotStartedAt;
            DELETE FROM SwarmNodeVolumeProjections
            WHERE PlatformId = @PlatformId AND DockerNodeId = @DockerNodeId
              AND ObservedAt < @SnapshotStartedAt
              AND NOT (VolumeName = ANY(@Names));
            """;
        var rows = JsonSerializer.Serialize(values, PlatformJsonContext.Default.IReadOnlyListSwarmNodeVolumeProjection);
        return await db.ExecuteAsync(sql, new
        {
            PlatformId = platformId,
            DockerNodeId = dockerNodeId,
            Rows = rows,
            SnapshotStartedAt = snapshotStartedAt.UtcDateTime,
            Names = values.Select(static value => value.VolumeName).ToArray()
        }, transaction: tx());
    }

    private async Task<int> ReplaceNodeNetworksAsync(
        Guid platformId,
        string dockerNodeId,
        IReadOnlyList<SwarmNodeNetworkProjection> values,
        DateTimeOffset snapshotStartedAt)
    {
        const string sql = """
            INSERT INTO SwarmNodeNetworkProjections (
                PlatformId, DockerNodeId, DockerNetworkId, Resource, ObservedAt, IsStale)
            SELECT @PlatformId, @DockerNodeId, value."DockerNetworkId", value."Resource", value."ObservedAt", false
            FROM jsonb_to_recordset(@Rows::jsonb) AS value(
                "DockerNetworkId" text, "Resource" jsonb, "ObservedAt" timestamptz)
            ON CONFLICT (PlatformId, DockerNodeId, DockerNetworkId) DO UPDATE SET
                Resource = excluded.Resource,
                ObservedAt = excluded.ObservedAt,
                IsStale = false
            WHERE SwarmNodeNetworkProjections.ObservedAt < @SnapshotStartedAt;
            DELETE FROM SwarmNodeNetworkProjections
            WHERE PlatformId = @PlatformId AND DockerNodeId = @DockerNodeId
              AND ObservedAt < @SnapshotStartedAt
              AND NOT (DockerNetworkId = ANY(@Ids));
            """;
        var rows = JsonSerializer.Serialize(values, PlatformJsonContext.Default.IReadOnlyListSwarmNodeNetworkProjection);
        return await db.ExecuteAsync(sql, new
        {
            PlatformId = platformId,
            DockerNodeId = dockerNodeId,
            Rows = rows,
            SnapshotStartedAt = snapshotStartedAt.UtcDateTime,
            Ids = values.Select(static value => value.DockerNetworkId).ToArray()
        }, transaction: tx());
    }

    public async Task<int> ReplaceAsync(Guid platformId, SwarmProjectionSnapshot snapshot, CancellationToken cancellationToken)
    {
        var affected = await ReplaceNodesAsync(platformId, snapshot.Nodes);
        affected += await ReplaceServicesAsync(platformId, snapshot.Services);
        affected += await ReplaceTasksAsync(platformId, snapshot.Tasks);
        affected += await ReplaceNetworksAsync(platformId, snapshot.Networks);
        affected += await ReplaceSecretsAsync(platformId, snapshot.Secrets);
        affected += await ReplaceConfigsAsync(platformId, snapshot.Configs);
        return affected;
    }

    public Task<int> MarkStaleAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE SwarmNodeProjections SET IsStale = true WHERE PlatformId = @PlatformId AND IsStale = false;
            UPDATE SwarmServiceProjections SET IsStale = true WHERE PlatformId = @PlatformId AND IsStale = false;
            UPDATE SwarmTaskProjections SET IsStale = true WHERE PlatformId = @PlatformId AND IsStale = false;
            UPDATE SwarmNetworkProjections SET IsStale = true WHERE PlatformId = @PlatformId AND IsStale = false;
            UPDATE SwarmSecretProjections SET IsStale = true WHERE PlatformId = @PlatformId AND IsStale = false;
            UPDATE SwarmConfigProjections SET IsStale = true WHERE PlatformId = @PlatformId AND IsStale = false;
            """;
        return db.ExecuteAsync(sql, new { PlatformId = platformId }, transaction: tx());
    }

    private async Task<int> ReplaceNodesAsync(Guid platformId, IReadOnlyList<SwarmNodeProjection> values)
    {
        const string sql = """
            INSERT INTO SwarmNodeProjections (PlatformId, DockerNodeId, VersionIndex, Hostname, Role,
                IsLeader, Reachability, Status, StatusMessage, Availability, EngineVersion, OperatingSystem,
                Architecture, Address, Labels, RunningTaskCount, DesiredTaskCount, DockerCreatedAt,
                DockerUpdatedAt, ObservedAt, IsStale)
            SELECT @PlatformId, value."DockerNodeId", value."VersionIndex", value."Hostname", value."Role",
                value."IsLeader", value."Reachability", value."Status", value."StatusMessage",
                value."Availability", value."EngineVersion", value."OperatingSystem", value."Architecture",
                value."Address", value."Labels", value."RunningTaskCount", value."DesiredTaskCount",
                value."DockerCreatedAt", value."DockerUpdatedAt", value."ObservedAt", false
            FROM jsonb_to_recordset(@Rows::jsonb) AS value(
                "DockerNodeId" text, "VersionIndex" bigint, "Hostname" text, "Role" text,
                "IsLeader" boolean, "Reachability" text, "Status" text, "StatusMessage" text,
                "Availability" text, "EngineVersion" text, "OperatingSystem" text, "Architecture" text,
                "Address" text, "Labels" jsonb, "RunningTaskCount" integer, "DesiredTaskCount" integer,
                "DockerCreatedAt" timestamptz, "DockerUpdatedAt" timestamptz, "ObservedAt" timestamptz)
            ON CONFLICT (PlatformId, DockerNodeId) DO UPDATE SET VersionIndex=excluded.VersionIndex,
                Hostname=excluded.Hostname, Role=excluded.Role, IsLeader=excluded.IsLeader,
                Reachability=excluded.Reachability, Status=excluded.Status, StatusMessage=excluded.StatusMessage,
                Availability=excluded.Availability, EngineVersion=excluded.EngineVersion,
                OperatingSystem=excluded.OperatingSystem, Architecture=excluded.Architecture,
                Address=excluded.Address, Labels=excluded.Labels, RunningTaskCount=excluded.RunningTaskCount,
                DesiredTaskCount=excluded.DesiredTaskCount, DockerCreatedAt=excluded.DockerCreatedAt,
                DockerUpdatedAt=excluded.DockerUpdatedAt, ObservedAt=excluded.ObservedAt, IsStale=false;
            DELETE FROM SwarmNodeProjections
            WHERE PlatformId = @PlatformId AND NOT (DockerNodeId = ANY(@Ids));
            """;
        var rows = JsonSerializer.Serialize(values, PlatformJsonContext.Default.IReadOnlyListSwarmNodeProjection);
        return await db.ExecuteAsync(sql, new { PlatformId = platformId, Rows = rows, Ids = values.Select(static value => value.DockerNodeId).ToArray() }, transaction: tx());
    }

    private async Task<int> ReplaceServicesAsync(Guid platformId, IReadOnlyList<SwarmServiceProjection> values)
    {
        const string sql = """
            INSERT INTO SwarmServiceProjections (PlatformId, DockerServiceId, VersionIndex, Name, Mode, Image,
                RunningTaskCount, DesiredTaskCount, UpdateState, UpdateMessage, Ports, NetworkIds, SecretIds,
                ConfigIds, Labels, Ownership, DockerStackNamespace,
                OwnershipDiagnostic, SwarmServiceId, StackId, LiveRuntimeHash, ForceUpdate,
                DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale)
            SELECT @PlatformId, value."DockerServiceId", value."VersionIndex", value."Name", value."Mode",
                value."Image", value."RunningTaskCount", value."DesiredTaskCount", value."UpdateState",
                value."UpdateMessage", value."Ports", value."NetworkIds", value."SecretIds", value."ConfigIds",
                value."Labels", value."Ownership", value."DockerStackNamespace",
                value."OwnershipDiagnostic", value."SwarmServiceId", value."StackId", value."LiveRuntimeHash", value."ForceUpdate", value."DockerCreatedAt",
                value."DockerUpdatedAt", value."ObservedAt", false
            FROM jsonb_to_recordset(@Rows::jsonb) AS value(
                "DockerServiceId" text, "VersionIndex" bigint, "Name" text, "Mode" text, "Image" text,
                "RunningTaskCount" integer, "DesiredTaskCount" integer, "UpdateState" text,
                "UpdateMessage" text, "Ports" jsonb, "NetworkIds" jsonb, "SecretIds" jsonb,
                "ConfigIds" jsonb, "Labels" jsonb, "Ownership" text,
                "DockerStackNamespace" text, "OwnershipDiagnostic" text,
                "SwarmServiceId" uuid, "StackId" uuid, "LiveRuntimeHash" text, "ForceUpdate" bigint,
                "DockerCreatedAt" timestamptz,
                "DockerUpdatedAt" timestamptz, "ObservedAt" timestamptz)
            ON CONFLICT (PlatformId, DockerServiceId) DO UPDATE SET VersionIndex=excluded.VersionIndex,
                Name=excluded.Name, Mode=excluded.Mode, Image=excluded.Image,
                RunningTaskCount=excluded.RunningTaskCount, DesiredTaskCount=excluded.DesiredTaskCount,
                UpdateState=excluded.UpdateState, UpdateMessage=excluded.UpdateMessage, Ports=excluded.Ports,
                NetworkIds=excluded.NetworkIds, SecretIds=excluded.SecretIds, ConfigIds=excluded.ConfigIds,
                Labels=excluded.Labels,
                Ownership=CASE WHEN SwarmServiceProjections.StackId IS NULL THEN excluded.Ownership ELSE 'CitadelStack' END,
                DockerStackNamespace=excluded.DockerStackNamespace,
                OwnershipDiagnostic=CASE WHEN SwarmServiceProjections.StackId IS NULL THEN excluded.OwnershipDiagnostic ELSE NULL END,
                SwarmServiceId=excluded.SwarmServiceId,
                StackId=COALESCE(SwarmServiceProjections.StackId, excluded.StackId),
                LiveRuntimeHash=excluded.LiveRuntimeHash, ForceUpdate=excluded.ForceUpdate,
                DockerCreatedAt=excluded.DockerCreatedAt,
                DockerUpdatedAt=excluded.DockerUpdatedAt, ObservedAt=excluded.ObservedAt, IsStale=false;
            UPDATE SwarmServiceProjections
            SET IsStale = true
            WHERE PlatformId = @PlatformId
              AND NOT (DockerServiceId = ANY(@Ids))
              AND StackId IS NOT NULL;
            DELETE FROM SwarmServiceProjections
            WHERE PlatformId = @PlatformId
              AND NOT (DockerServiceId = ANY(@Ids))
              AND StackId IS NULL;
            """;
        var rows = JsonSerializer.Serialize(values, PlatformJsonContext.Default.IReadOnlyListSwarmServiceProjection);
        return await db.ExecuteAsync(sql, new { PlatformId = platformId, Rows = rows, Ids = values.Select(static value => value.DockerServiceId).ToArray() }, transaction: tx());
    }

    private async Task<int> ReplaceTasksAsync(Guid platformId, IReadOnlyList<SwarmTaskProjection> values)
    {
        const string sql = """
            INSERT INTO SwarmTaskProjections (PlatformId, DockerTaskId, VersionIndex, Name, DockerServiceId,
                ServiceName, Slot, DockerNodeId, NodeHostname, DesiredState, State, StatusMessage, Error,
                Image, Ports, StatusTimestamp, DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale,
                DockerContainerId)
            SELECT @PlatformId, value."DockerTaskId", value."VersionIndex", value."Name",
                value."DockerServiceId", value."ServiceName", value."Slot", value."DockerNodeId",
                value."NodeHostname", value."DesiredState", value."State", value."StatusMessage", value."Error",
                value."Image", value."Ports", value."StatusTimestamp", value."DockerCreatedAt",
                value."DockerUpdatedAt", value."ObservedAt", false, value."DockerContainerId"
            FROM jsonb_to_recordset(@Rows::jsonb) AS value(
                "DockerTaskId" text, "VersionIndex" bigint, "Name" text, "DockerServiceId" text,
                "ServiceName" text, "Slot" integer, "DockerNodeId" text, "NodeHostname" text,
                "DesiredState" text, "State" text, "StatusMessage" text, "Error" text, "Image" text,
                "Ports" jsonb, "StatusTimestamp" timestamptz, "DockerCreatedAt" timestamptz,
                "DockerUpdatedAt" timestamptz, "ObservedAt" timestamptz, "DockerContainerId" text)
            ON CONFLICT (PlatformId, DockerTaskId) DO UPDATE SET VersionIndex=excluded.VersionIndex,
                Name=excluded.Name, DockerServiceId=excluded.DockerServiceId, ServiceName=excluded.ServiceName,
                Slot=excluded.Slot, DockerNodeId=excluded.DockerNodeId, NodeHostname=excluded.NodeHostname,
                DesiredState=excluded.DesiredState, State=excluded.State, StatusMessage=excluded.StatusMessage,
                Error=excluded.Error, Image=excluded.Image, Ports=excluded.Ports,
                StatusTimestamp=excluded.StatusTimestamp, DockerCreatedAt=excluded.DockerCreatedAt,
                DockerUpdatedAt=excluded.DockerUpdatedAt, ObservedAt=excluded.ObservedAt, IsStale=false,
                DockerContainerId=excluded.DockerContainerId;
            DELETE FROM SwarmTaskProjections
            WHERE PlatformId = @PlatformId AND NOT (DockerTaskId = ANY(@Ids));
            """;
        var rows = JsonSerializer.Serialize(values, PlatformJsonContext.Default.IReadOnlyListSwarmTaskProjection);
        return await db.ExecuteAsync(sql, new { PlatformId = platformId, Rows = rows, Ids = values.Select(static value => value.DockerTaskId).ToArray() }, transaction: tx());
    }

    private async Task<int> ReplaceNetworksAsync(Guid platformId, IReadOnlyList<SwarmNetworkProjection> values)
    {
        const string sql = """
            INSERT INTO SwarmNetworkProjections (PlatformId, DockerNetworkId, Name, Scope, Driver,
                IsAttachable, IsInternal, IsIngress, IsEncrypted, EnableIPv6, Subnets, ServiceNames, Labels,
                DockerCreatedAt, ObservedAt, IsStale)
            SELECT @PlatformId, value."DockerNetworkId", value."Name", value."Scope", value."Driver",
                value."IsAttachable", value."IsInternal", value."IsIngress", value."IsEncrypted",
                value."EnableIPv6", value."Subnets", value."ServiceNames", value."Labels",
                value."DockerCreatedAt", value."ObservedAt", false
            FROM jsonb_to_recordset(@Rows::jsonb) AS value(
                "DockerNetworkId" text, "Name" text, "Scope" text, "Driver" text,
                "IsAttachable" boolean, "IsInternal" boolean, "IsIngress" boolean,
                "IsEncrypted" boolean, "EnableIPv6" boolean, "Subnets" jsonb, "ServiceNames" jsonb,
                "Labels" jsonb, "DockerCreatedAt" timestamptz, "ObservedAt" timestamptz)
            ON CONFLICT (PlatformId, DockerNetworkId) DO UPDATE SET Name=excluded.Name, Scope=excluded.Scope,
                Driver=excluded.Driver, IsAttachable=excluded.IsAttachable, IsInternal=excluded.IsInternal,
                IsIngress=excluded.IsIngress, IsEncrypted=excluded.IsEncrypted, EnableIPv6=excluded.EnableIPv6,
                Subnets=excluded.Subnets, ServiceNames=excluded.ServiceNames, Labels=excluded.Labels, DockerCreatedAt=excluded.DockerCreatedAt,
                ObservedAt=excluded.ObservedAt, IsStale=false;
            DELETE FROM SwarmNetworkProjections
            WHERE PlatformId = @PlatformId AND NOT (DockerNetworkId = ANY(@Ids));
            """;
        var rows = JsonSerializer.Serialize(values, PlatformJsonContext.Default.IReadOnlyListSwarmNetworkProjection);
        return await db.ExecuteAsync(sql, new { PlatformId = platformId, Rows = rows, Ids = values.Select(static value => value.DockerNetworkId).ToArray() }, transaction: tx());
    }

    private async Task<int> ReplaceSecretsAsync(Guid platformId, IReadOnlyList<SwarmSecretProjection> values)
    {
        const string sql = """
            INSERT INTO SwarmSecretProjections (PlatformId, DockerSecretId, VersionIndex, Name, Driver, ServiceNames, Labels,
                DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale)
            SELECT @PlatformId, value."DockerSecretId", value."VersionIndex", value."Name", value."Driver",
                value."ServiceNames", value."Labels", value."DockerCreatedAt", value."DockerUpdatedAt",
                value."ObservedAt", false
            FROM jsonb_to_recordset(@Rows::jsonb) AS value(
                "DockerSecretId" text, "VersionIndex" bigint, "Name" text, "Driver" text,
                "ServiceNames" jsonb, "Labels" jsonb, "DockerCreatedAt" timestamptz,
                "DockerUpdatedAt" timestamptz, "ObservedAt" timestamptz)
            ON CONFLICT (PlatformId, DockerSecretId) DO UPDATE SET VersionIndex=excluded.VersionIndex,
                Name=excluded.Name, Driver=excluded.Driver, ServiceNames=excluded.ServiceNames, Labels=excluded.Labels,
                DockerCreatedAt=excluded.DockerCreatedAt, DockerUpdatedAt=excluded.DockerUpdatedAt,
                ObservedAt=excluded.ObservedAt, IsStale=false;
            DELETE FROM SwarmSecretProjections
            WHERE PlatformId = @PlatformId AND NOT (DockerSecretId = ANY(@Ids));
            """;
        var rows = JsonSerializer.Serialize(values, PlatformJsonContext.Default.IReadOnlyListSwarmSecretProjection);
        return await db.ExecuteAsync(sql, new { PlatformId = platformId, Rows = rows, Ids = values.Select(static value => value.DockerSecretId).ToArray() }, transaction: tx());
    }

    private async Task<int> ReplaceConfigsAsync(Guid platformId, IReadOnlyList<SwarmConfigProjection> values)
    {
        const string sql = """
            INSERT INTO SwarmConfigProjections (PlatformId, DockerConfigId, VersionIndex, Name, TemplatingDriver,
                ServiceNames, Labels, DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale)
            SELECT @PlatformId, value."DockerConfigId", value."VersionIndex", value."Name",
                value."TemplatingDriver", value."ServiceNames", value."Labels", value."DockerCreatedAt",
                value."DockerUpdatedAt", value."ObservedAt", false
            FROM jsonb_to_recordset(@Rows::jsonb) AS value(
                "DockerConfigId" text, "VersionIndex" bigint, "Name" text, "TemplatingDriver" text,
                "ServiceNames" jsonb, "Labels" jsonb, "DockerCreatedAt" timestamptz,
                "DockerUpdatedAt" timestamptz, "ObservedAt" timestamptz)
            ON CONFLICT (PlatformId, DockerConfigId) DO UPDATE SET VersionIndex=excluded.VersionIndex,
                Name=excluded.Name, TemplatingDriver=excluded.TemplatingDriver, ServiceNames=excluded.ServiceNames, Labels=excluded.Labels,
                DockerCreatedAt=excluded.DockerCreatedAt, DockerUpdatedAt=excluded.DockerUpdatedAt,
                ObservedAt=excluded.ObservedAt, IsStale=false;
            DELETE FROM SwarmConfigProjections
            WHERE PlatformId = @PlatformId AND NOT (DockerConfigId = ANY(@Ids));
            """;
        var rows = JsonSerializer.Serialize(values, PlatformJsonContext.Default.IReadOnlyListSwarmConfigProjection);
        return await db.ExecuteAsync(sql, new { PlatformId = platformId, Rows = rows, Ids = values.Select(static value => value.DockerConfigId).ToArray() }, transaction: tx());
    }

}
