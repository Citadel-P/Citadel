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
                ) AS IsStale,
                (SELECT COUNT(*)::int FROM SwarmNodeProjections WHERE PlatformId = @PlatformId) AS NodeCount,
                (SELECT COUNT(*)::int FROM SwarmNodeProjections
                    WHERE PlatformId = @PlatformId AND lower(Role) = 'manager') AS ManagerCount,
                (SELECT COUNT(*)::int FROM SwarmServiceProjections WHERE PlatformId = @PlatformId) AS ServiceCount,
                (SELECT COALESCE(SUM(RunningTaskCount), 0)::int FROM SwarmServiceProjections
                    WHERE PlatformId = @PlatformId) AS RunningTaskCount,
                (SELECT COALESCE(SUM(DesiredTaskCount), 0)::int FROM SwarmServiceProjections
                    WHERE PlatformId = @PlatformId) AS DesiredTaskCount,
                (SELECT COUNT(*)::int FROM SwarmNetworkProjections WHERE PlatformId = @PlatformId) AS NetworkCount
            """;
        var value = await db.QuerySingleAsync<SwarmProjectionSummaryDto>(
            sql,
            new { PlatformId = platformId },
            transaction: tx());
        return new SwarmProjectionSummary(
            value.IsStale,
            value.NodeCount,
            value.ManagerCount,
            value.ServiceCount,
            value.RunningTaskCount,
            value.DesiredTaskCount,
            value.NetworkCount);
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
                   OwnershipDiagnostic, SwarmServiceId, LiveRuntimeHash, ForceUpdate,
                   DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale
            FROM SwarmServiceProjections WHERE PlatformId = @PlatformId ORDER BY Name, DockerServiceId
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
                   OwnershipDiagnostic, SwarmServiceId, LiveRuntimeHash, ForceUpdate,
                   DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale
            FROM SwarmServiceProjections WHERE PlatformId = @PlatformId AND DockerServiceId = @DockerServiceId
            """;
        var row = await db.QuerySingleOrDefaultAsync<SwarmServiceProjectionDto>(sql, new { PlatformId = platformId, DockerServiceId = dockerServiceId }, transaction: tx());
        return row?.ToDomain();
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
                   StatusTimestamp, DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale
            FROM SwarmTaskProjections
            WHERE PlatformId = @PlatformId
              AND (@DockerServiceId IS NULL OR DockerServiceId = @DockerServiceId)
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
                   StatusTimestamp, DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale
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
                OwnershipDiagnostic, SwarmServiceId, LiveRuntimeHash, ForceUpdate,
                DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale)
            SELECT @PlatformId, value."DockerServiceId", value."VersionIndex", value."Name", value."Mode",
                value."Image", value."RunningTaskCount", value."DesiredTaskCount", value."UpdateState",
                value."UpdateMessage", value."Ports", value."NetworkIds", value."SecretIds", value."ConfigIds",
                value."Labels", value."Ownership", value."DockerStackNamespace",
                value."OwnershipDiagnostic", value."SwarmServiceId", value."LiveRuntimeHash", value."ForceUpdate", value."DockerCreatedAt",
                value."DockerUpdatedAt", value."ObservedAt", false
            FROM jsonb_to_recordset(@Rows::jsonb) AS value(
                "DockerServiceId" text, "VersionIndex" bigint, "Name" text, "Mode" text, "Image" text,
                "RunningTaskCount" integer, "DesiredTaskCount" integer, "UpdateState" text,
                "UpdateMessage" text, "Ports" jsonb, "NetworkIds" jsonb, "SecretIds" jsonb,
                "ConfigIds" jsonb, "Labels" jsonb, "Ownership" text,
                "DockerStackNamespace" text, "OwnershipDiagnostic" text,
                "SwarmServiceId" uuid, "LiveRuntimeHash" text, "ForceUpdate" bigint,
                "DockerCreatedAt" timestamptz,
                "DockerUpdatedAt" timestamptz, "ObservedAt" timestamptz)
            ON CONFLICT (PlatformId, DockerServiceId) DO UPDATE SET VersionIndex=excluded.VersionIndex,
                Name=excluded.Name, Mode=excluded.Mode, Image=excluded.Image,
                RunningTaskCount=excluded.RunningTaskCount, DesiredTaskCount=excluded.DesiredTaskCount,
                UpdateState=excluded.UpdateState, UpdateMessage=excluded.UpdateMessage, Ports=excluded.Ports,
                NetworkIds=excluded.NetworkIds, SecretIds=excluded.SecretIds, ConfigIds=excluded.ConfigIds,
                Labels=excluded.Labels, Ownership=excluded.Ownership,
                DockerStackNamespace=excluded.DockerStackNamespace,
                OwnershipDiagnostic=excluded.OwnershipDiagnostic, SwarmServiceId=excluded.SwarmServiceId,
                LiveRuntimeHash=excluded.LiveRuntimeHash, ForceUpdate=excluded.ForceUpdate,
                DockerCreatedAt=excluded.DockerCreatedAt,
                DockerUpdatedAt=excluded.DockerUpdatedAt, ObservedAt=excluded.ObservedAt, IsStale=false;
            DELETE FROM SwarmServiceProjections
            WHERE PlatformId = @PlatformId AND NOT (DockerServiceId = ANY(@Ids));
            """;
        var rows = JsonSerializer.Serialize(values, PlatformJsonContext.Default.IReadOnlyListSwarmServiceProjection);
        return await db.ExecuteAsync(sql, new { PlatformId = platformId, Rows = rows, Ids = values.Select(static value => value.DockerServiceId).ToArray() }, transaction: tx());
    }

    private async Task<int> ReplaceTasksAsync(Guid platformId, IReadOnlyList<SwarmTaskProjection> values)
    {
        const string sql = """
            INSERT INTO SwarmTaskProjections (PlatformId, DockerTaskId, VersionIndex, Name, DockerServiceId,
                ServiceName, Slot, DockerNodeId, NodeHostname, DesiredState, State, StatusMessage, Error,
                Image, Ports, StatusTimestamp, DockerCreatedAt, DockerUpdatedAt, ObservedAt, IsStale)
            SELECT @PlatformId, value."DockerTaskId", value."VersionIndex", value."Name",
                value."DockerServiceId", value."ServiceName", value."Slot", value."DockerNodeId",
                value."NodeHostname", value."DesiredState", value."State", value."StatusMessage", value."Error",
                value."Image", value."Ports", value."StatusTimestamp", value."DockerCreatedAt",
                value."DockerUpdatedAt", value."ObservedAt", false
            FROM jsonb_to_recordset(@Rows::jsonb) AS value(
                "DockerTaskId" text, "VersionIndex" bigint, "Name" text, "DockerServiceId" text,
                "ServiceName" text, "Slot" integer, "DockerNodeId" text, "NodeHostname" text,
                "DesiredState" text, "State" text, "StatusMessage" text, "Error" text, "Image" text,
                "Ports" jsonb, "StatusTimestamp" timestamptz, "DockerCreatedAt" timestamptz,
                "DockerUpdatedAt" timestamptz, "ObservedAt" timestamptz)
            ON CONFLICT (PlatformId, DockerTaskId) DO UPDATE SET VersionIndex=excluded.VersionIndex,
                Name=excluded.Name, DockerServiceId=excluded.DockerServiceId, ServiceName=excluded.ServiceName,
                Slot=excluded.Slot, DockerNodeId=excluded.DockerNodeId, NodeHostname=excluded.NodeHostname,
                DesiredState=excluded.DesiredState, State=excluded.State, StatusMessage=excluded.StatusMessage,
                Error=excluded.Error, Image=excluded.Image, Ports=excluded.Ports,
                StatusTimestamp=excluded.StatusTimestamp, DockerCreatedAt=excluded.DockerCreatedAt,
                DockerUpdatedAt=excluded.DockerUpdatedAt, ObservedAt=excluded.ObservedAt, IsStale=false;
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
