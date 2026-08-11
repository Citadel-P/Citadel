using Application.Configs;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.Options;

namespace Application.Features.Platforms.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetSwarmNodeAgentCoverage(Guid PlatformId) : IQuery<Result<SwarmNodeAgentCoverageResult>>;

internal sealed class GetSwarmNodeAgentCoverageHandler(
    IUnitOfWork unitOfWork,
    IEdgeAgentSessionStatus sessionStatus,
    IOptions<EdgeAgentOptions> options)
    : IQueryHandler<GetSwarmNodeAgentCoverage, Result<SwarmNodeAgentCoverageResult>>
{
    public async ValueTask<Result<SwarmNodeAgentCoverageResult>> Handle(
        GetSwarmNodeAgentCoverage query,
        CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(query.PlatformId, cancellationToken);
        if (platform is null)
            return Result.Failure<SwarmNodeAgentCoverageResult>(new NotFoundError("Platform does not exist."));
        if (platform.PlatformDescriptor is not DockerSwarmPlatformDescriptor descriptor)
            return Result.Failure<SwarmNodeAgentCoverageResult>(new BadRequestError("Node Agent coverage is available only for Docker Swarm platforms."));

        var nodes = await unitOfWork.Swarm.GetNodesAsync(query.PlatformId, cancellationToken);
        var bindings = (await unitOfWork.EdgeAgents.GetNodeBindingsAsync(query.PlatformId, cancellationToken))
            .Where(static binding => !binding.IsRevoked && binding.DockerNodeId is not null)
            .ToDictionary(static binding => binding.DockerNodeId!, StringComparer.Ordinal);
        var installation = await unitOfWork.EdgeAgents.GetNodeAgentInstallationAsync(query.PlatformId, cancellationToken);
        var bootstrap = await unitOfWork.EdgeAgents.GetLatestNodeAgentBootstrapAsync(query.PlatformId, cancellationToken);
        var services = await unitOfWork.Swarm.GetServicesAsync(query.PlatformId, cancellationToken);
        var runtimeStates = (await unitOfWork.Swarm.GetNodeRuntimeStatesAsync(query.PlatformId, cancellationToken))
            .ToDictionary(static state => state.DockerNodeId, StringComparer.Ordinal);
        var serviceTasks = string.IsNullOrWhiteSpace(installation?.DockerServiceId)
            ? []
            : await unitOfWork.Swarm.GetTasksAsync(query.PlatformId, 10_000, cancellationToken, installation.DockerServiceId);
        var tasksByNode = serviceTasks
            .Where(static task => !task.IsStale)
            .GroupBy(static task => task.DockerNodeId, StringComparer.Ordinal)
            .ToDictionary(
                static group => group.Key,
                static group => group.OrderByDescending(task => task.DockerUpdatedAt ?? task.ObservedAt).First(),
                StringComparer.Ordinal);

        var supportedArchitectures = new HashSet<string>(
            options.Value.SupportedNodeArchitectures
                .Where(static value => !string.IsNullOrWhiteSpace(value))
                .Select(NormalizeArchitecture),
            StringComparer.OrdinalIgnoreCase);
        var now = DateTime.UtcNow;
        var results = new List<SwarmNodeAgentNodeCoverageResult>(nodes.Count);
        foreach (var node in nodes)
        {
            var isManagerSource = string.Equals(node.DockerNodeId, descriptor.NodeID, StringComparison.Ordinal);
            var supported = string.Equals(node.OperatingSystem, "linux", StringComparison.OrdinalIgnoreCase)
                            && supportedArchitectures.Contains(NormalizeArchitecture(node.Architecture));
            var schedulable = string.Equals(node.Status, "ready", StringComparison.OrdinalIgnoreCase)
                              && string.Equals(node.Availability, "active", StringComparison.OrdinalIgnoreCase);
            var eligible = isManagerSource || (supported && schedulable);
            bindings.TryGetValue(node.DockerNodeId, out var binding);
            tasksByNode.TryGetValue(node.DockerNodeId, out var task);
            runtimeStates.TryGetValue(node.DockerNodeId, out var runtimeState);
            var connected = isManagerSource
                ? platform.Status == PlatformStatus.Online
                : binding is not null && sessionStatus.IsNodeConnected(query.PlatformId, node.DockerNodeId);
            var compatible = binding is null || binding.ProtocolVersion == Constants.EdgeAgentProtocolVersion;
            var projectionStale = node.IsStale
                                  || (!isManagerSource && (runtimeState is null || runtimeState.IsStale))
                                  || (!isManagerSource
                                      && binding?.LastHeartbeatAtUtc is { } heartbeat
                                      && heartbeat < now.AddMinutes(-2));
            var reasons = new List<string>(3);
            if (!supported)
                reasons.Add(string.Equals(node.OperatingSystem, "linux", StringComparison.OrdinalIgnoreCase) ? "UnsupportedArchitecture" : "UnsupportedOperatingSystem");
            if (!schedulable)
                reasons.Add("Unschedulable");
            if (eligible && !connected)
                reasons.Add(binding is null ? "AgentMissing" : "AgentOffline");
            if (!compatible)
                reasons.Add("AgentIncompatible");
            if (projectionStale)
                reasons.Add("ProjectionStale");
            var dockerReachable = isManagerSource
                ? connected
                : connected
                  && runtimeState is { IsStale: false, LastSuccessfulReconciliationAt: not null };
            if (connected && !dockerReachable)
                reasons.Add("DockerRuntimeUnavailable");

            var connectionState = isManagerSource
                ? "ManagerConnector"
                : !supported
                    ? "Unsupported"
                    : !schedulable
                        ? "Unschedulable"
                        : !compatible
                            ? "Incompatible"
                            : connected && projectionStale
                                ? "Stale"
                                : connected
                                    ? "Connected"
                                    : binding is not null
                                        ? "Offline"
                                        : task is not null
                                            ? "Enrolling"
                                            : "Missing";

            results.Add(new SwarmNodeAgentNodeCoverageResult(
                node.DockerNodeId,
                node.Hostname,
                node.Role,
                node.Availability,
                node.Status,
                node.Architecture,
                isManagerSource ? "ManagerConnector" : "Satellite",
                eligible,
                supported,
                schedulable,
                task?.State,
                connectionState,
                dockerReachable,
                compatible,
                projectionStale,
                binding?.LastHeartbeatAtUtc,
                runtimeState?.LastSuccessfulReconciliationAt,
                runtimeState?.StaleSince,
                runtimeState?.StaleReason,
                reasons));
        }

        var eligibleResults = results.Where(static node => node.Eligible).ToArray();
        var covered = eligibleResults.Count(static node => node.DockerReachable && !node.ProjectionStale);
        var requiresSatellites = eligibleResults.Any(static node => node.DataSource == "Satellite");
        var isInstalled = installation is { DesiredState: SwarmNodeAgentDesiredState.Installed };
        var serviceDriftReason = isInstalled
                                 && (requiresSatellites || !string.IsNullOrWhiteSpace(installation!.DockerServiceId))
            ? SwarmNodeAgentInfrastructure.GetServiceDriftReason(platform, installation!, services)
            : null;
        var inventoryUnavailable = nodes.Count == 0;
        var aggregateReasons = new List<string>(2);
        if (inventoryUnavailable)
            aggregateReasons.Add("SwarmInventoryUnavailable");
        if (serviceDriftReason is not null)
            aggregateReasons.Add("NodeAgentServiceDrifted");
        var operationRunning = installation is { OperationState: SwarmNodeAgentOperationState.Running };
        var state = inventoryUnavailable
            ? "Unavailable"
            : operationRunning && installation!.OperationKind == SwarmNodeAgentOperationKind.Remove
            ? "Removing"
            : operationRunning
            ? "Installing"
            : serviceDriftReason is not null
            ? covered > 0 ? "Partial" : "Failed"
            : !requiresSatellites && covered == eligibleResults.Length
            ? "Complete"
            : !isInstalled
            ? "NotInstalled"
            : covered == eligibleResults.Length
                ? "Complete"
                : covered > 0
                    ? "Partial"
                    : "Failed";
        var operation = installation is { OperationId: { } operationId, OperationKind: { } operationKind, OperationState: { } operationState, OperationStartedAtUtc: { } startedAt }
            ? new SwarmNodeAgentOperationResult(operationId, operationKind.ToString(), operationState.ToString(), startedAt, installation.OperationError)
            : null;

        return Result.Success(new SwarmNodeAgentCoverageResult(
            state,
            isInstalled,
            covered,
            eligibleResults.Length,
            results.Count,
            results.Count(static node => node.AgentConnectionState is "ManagerConnector" or "Connected" or "Stale"),
            results.Count(static node => node.AgentConnectionState == "Offline"),
            results.Count(static node => node.AgentConnectionState == "Enrolling"),
            results.Count(static node => node.AgentConnectionState == "Missing"),
            results.Count(static node => !node.Compatible),
            results.Count(static node => !node.Supported),
            results.Count(static node => !node.Schedulable),
            results.Count(static node => node.ProjectionStale),
            nodes.Count == 0 ? null : nodes.Max(static node => node.ObservedAt),
            installation?.AgentImageReference,
            installation?.AgentImageDigest,
            bootstrap is not null && bootstrap.IsActive(now) ? bootstrap.ExpiresAtUtc : null,
            operation,
            aggregateReasons,
            results));
    }

    private static string NormalizeArchitecture(string architecture) => architecture.Trim().ToLowerInvariant() switch
    {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "armv7l" => "arm",
        var value => value
    };
}
