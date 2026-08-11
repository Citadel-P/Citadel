using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Application.Configs;
using Application.Features.Deployments.Notifications;
using Application.Features.Platforms;
using Application.Services.Abstractions;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Builds;
using Domain.Entities.Platforms;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;

namespace Application.Services;

internal sealed class EdgeAgentManagementService(
    IServiceScopeFactory scopeFactory,
    INotificationQueue notificationQueue,
    IPlatformStreamManager platformStreamManager,
    IActivityStreamManager activityStreamManager,
    IApplicationHubDispatcher hubDispatcher,
    IOptions<EdgeAgentOptions> edgeAgentOptions) : IEdgeAgentManagementService
{
    private static readonly string[] RequiredBuildAgentPoolCommands =
    [
        "images.build",
        "images.push",
        "images.checkBuildHost"
    ];
    private static readonly string[] RequiredSwarmNodeCommands =
    [
        "platform.checkHealth",
        "platform.getInfo",
        "platform.events",
        "containers.list",
        "containers.logs",
        "containers.inspect",
        "containers.patch",
        "containers.delete",
        "containers.stats",
        "containers.exec"
    ];

    public async Task<Result<EdgeAgentEnrollmentResult>> CreateEnrollmentAsync(Guid platformId, string coreUrl, Guid actorId, TimeSpan ttl, CancellationToken cancellationToken)
        => await CreateEnrollmentAsync(
            new EdgeAgentTarget(EdgeAgentResourceType.Platform, platformId, platformId, "edge-agent", "/app/data/edge-agent.key", "/app/data/edge-agent.identity.json"),
            coreUrl,
            actorId,
            ttl,
            cancellationToken);

    public async Task<Result<EdgeAgentEnrollmentResult>> CreateBuildAgentPoolEnrollmentAsync(Guid buildAgentPoolId, string coreUrl, Guid actorId, TimeSpan ttl, CancellationToken cancellationToken)
        => await CreateEnrollmentAsync(
            new EdgeAgentTarget(EdgeAgentResourceType.BuildAgentPool, buildAgentPoolId, Guid.Empty, "edge-build-agent", "/app/data/edge-build-agent.key", "/app/data/edge-build-agent.identity.json"),
            coreUrl,
            actorId,
            ttl,
            cancellationToken);

    private async Task<Result<EdgeAgentEnrollmentResult>> CreateEnrollmentAsync(
        EdgeAgentTarget target,
        string coreUrl,
        Guid actorId,
        TimeSpan ttl,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var validation = await ValidateTargetAsync(target, unitOfWork, cancellationToken);
        if (validation.IsFailure(out var validationError))
        {
            return Result.Failure<EdgeAgentEnrollmentResult>(validationError!);
        }

        var binding = await unitOfWork.EdgeAgents.GetBindingByResourceAsync(target.ResourceType, target.ResourceId, cancellationToken);
        if (binding?.IsRevoked == true)
        {
            return Result.Failure<EdgeAgentEnrollmentResult>(new ConflictError("Edge Agent binding is revoked."));
        }

        var utcNow = DateTime.UtcNow;
        var token = GenerateToken();
        var expiresAt = utcNow.Add(ttl);
        var enrollment = new EdgeAgentEnrollment(
            Id: Guid.CreateVersion7(),
            PlatformId: target.PlatformId,
            ResourceType: target.ResourceType,
            ResourceId: target.ResourceId,
            TokenHash: HashToken(token),
            ExpiresAtUtc: expiresAt,
            UsedAtUtc: null,
            RevokedAtUtc: null,
            CreatedByActorId: actorId,
            CreatedAtUtc: utcNow);

        await unitOfWork.EdgeAgents.AddEnrollmentAsync(enrollment, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        var environment = new Dictionary<string, string>
        {
            ["CITADEL_AGENT_MODE"] = "edge",
            ["CITADEL_CORE_URL"] = coreUrl,
            ["CITADEL_EDGE_ENROLLMENT_TOKEN"] = token,
            ["CITADEL_EDGE_AGENT_KEY_PATH"] = target.KeyPath,
            ["CITADEL_EDGE_IDENTITY_PATH"] = target.IdentityPath
        };

        var agentImage = edgeAgentOptions.Value.GetAgentImage();
        var dockerRunCommand = AgentDockerCommandBuilder.BuildEdgeAgentCommand(
            agentImage,
            environment,
            includeHostRootMount: target.ResourceType == EdgeAgentResourceType.Platform,
            systemRole: target.ResourceType == EdgeAgentResourceType.Platform ? "edge-agent" : null,
            containerName: target.ContainerName,
            dataVolumeName: target.ContainerName.Replace("-", "_", StringComparison.Ordinal) + "_data");

        return Result.Success(new EdgeAgentEnrollmentResult(
            enrollment.Id,
            target.PlatformId,
            token,
            expiresAt,
            new EdgeAgentEnrollmentInstructions(coreUrl, environment, agentImage, dockerRunCommand),
            target.ResourceType,
            target.ResourceId));
    }

    public async Task<Result<EdgeAgentStatusResult>> GetStatusAsync(Guid platformId, DateTime utcNow, CancellationToken cancellationToken)
        => await GetStatusAsync(EdgeAgentResourceType.Platform, platformId, utcNow, cancellationToken);

    public async Task<Result<EdgeAgentStatusResult>> GetBuildAgentPoolStatusAsync(Guid buildAgentPoolId, DateTime utcNow, CancellationToken cancellationToken)
        => await GetStatusAsync(EdgeAgentResourceType.BuildAgentPool, buildAgentPoolId, utcNow, cancellationToken);

    private async Task<Result<EdgeAgentStatusResult>> GetStatusAsync(
        EdgeAgentResourceType resourceType,
        Guid resourceId,
        DateTime utcNow,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var target = new EdgeAgentTarget(resourceType, resourceId, resourceType == EdgeAgentResourceType.Platform ? resourceId : Guid.Empty, "edge-agent", "/app/data/edge-agent.key", "/app/data/edge-agent.identity.json");
        var validation = await ValidateTargetAsync(target, unitOfWork, cancellationToken);
        if (validation.IsFailure(out var validationError))
        {
            return Result.Failure<EdgeAgentStatusResult>(validationError!);
        }

        var binding = await unitOfWork.EdgeAgents.GetBindingByResourceAsync(resourceType, resourceId, cancellationToken);
        var activeEnrollment = await unitOfWork.EdgeAgents.GetActiveEnrollmentAsync(resourceType, resourceId, utcNow, cancellationToken);
        if (binding is null)
        {
            return Result.Success(new EdgeAgentStatusResult(
                "PendingEnrollment",
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                activeEnrollment?.ExpiresAtUtc));
        }

        var status = binding.IsRevoked
            ? "Revoked"
            : binding.ConnectionStatus.ToString();

        return Result.Success(new EdgeAgentStatusResult(
            status,
            binding.LastConnectedAtUtc,
            binding.LastDisconnectedAtUtc,
            binding.LastHeartbeatAtUtc,
            binding.LastSeenVersion,
            binding.LastSeenHostname,
            ShortFingerprint(binding.AgentFingerprint),
            binding.ProtocolVersion,
            binding.RevokedAtUtc,
            activeEnrollment?.ExpiresAtUtc));
    }

    public async Task<Result<EdgeAgentEnrollmentCompleteResult>> CompleteEnrollmentAsync(EdgeAgentEnrollmentRequest request, DateTime utcNow, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        if (request.Profile == EdgeAgentProfile.SwarmNode)
        {
            return await CompleteSwarmNodeEnrollmentAsync(request, utcNow, unitOfWork, cancellationToken);
        }

        var enrollment = await unitOfWork.EdgeAgents.GetEnrollmentByTokenHashAsync(HashToken(request.EnrollmentToken), cancellationToken);
        if (enrollment is null || !enrollment.IsActive(utcNow))
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(new UnauthorizedError("Enrollment token is invalid, expired, revoked, or already used."));
        }

        var target = new EdgeAgentTarget(
            enrollment.NormalizedResourceType,
            enrollment.NormalizedResourceId,
            enrollment.PlatformId,
            "edge-agent",
            "/app/data/edge-agent.key",
            "/app/data/edge-agent.identity.json");
        var validation = await ValidateTargetAsync(target, unitOfWork, cancellationToken);
        if (validation.IsFailure(out var validationError))
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(validationError!);
        }

        if (target.ResourceType == EdgeAgentResourceType.BuildAgentPool &&
            !HasRequiredCommands(request.CapabilitiesJson, RequiredBuildAgentPoolCommands, out var missingCommand))
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(
                new BadRequestError($"Build pool Edge Agent must advertise capability '{missingCommand}'."));
        }

        Platform? edgePlatform = null;
        if (target.ResourceType == EdgeAgentResourceType.Platform)
        {
            var daemonValidation = await ValidatePlatformDaemonAsync(
                unitOfWork,
                target.ResourceId,
                request.DaemonId,
                cancellationToken);
            if (!daemonValidation.IsSuccess(
                    out edgePlatform,
                    out var daemonError))
            {
                return Result.Failure<EdgeAgentEnrollmentCompleteResult>(
                    daemonError!);
            }
        }

        var existingBinding = await unitOfWork.EdgeAgents.GetBindingByResourceAsync(enrollment.NormalizedResourceType, enrollment.NormalizedResourceId, cancellationToken);
        if (existingBinding is not null && !existingBinding.IsRevoked)
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(new ConflictError("Edge Agent target is already enrolled."));
        }

        var publicKeyBytes = Convert.FromBase64String(request.AgentPublicKey);
        var fingerprint = GetFingerprint(publicKeyBytes);
        if (!string.IsNullOrWhiteSpace(request.AgentFingerprint) &&
            !string.Equals(request.AgentFingerprint, fingerprint, StringComparison.Ordinal))
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(new BadRequestError("Agent fingerprint does not match the provided public key."));
        }

        var agentId = Guid.CreateVersion7();
        var now = utcNow;
        var binding = new EdgeAgentBinding(
            Id: Guid.CreateVersion7(),
            PlatformId: enrollment.PlatformId,
            ResourceType: enrollment.NormalizedResourceType,
            ResourceId: enrollment.NormalizedResourceId,
            AgentId: agentId,
            AgentPublicKey: request.AgentPublicKey,
            AgentFingerprint: fingerprint,
            ConnectionStatus: EdgeAgentConnectionStatus.Offline,
            LastConnectedAtUtc: null,
            LastDisconnectedAtUtc: null,
            LastHeartbeatAtUtc: null,
            LastSeenVersion: request.AgentVersion,
            LastSeenHostname: request.Hostname,
            CapabilitiesJson: string.IsNullOrWhiteSpace(request.CapabilitiesJson) ? "{}" : request.CapabilitiesJson,
            ProtocolVersion: request.ProtocolVersion,
            RevokedAtUtc: null,
            CreatedAtUtc: now,
            UpdatedAtUtc: now);

        if (await unitOfWork.EdgeAgents.AddBindingAsync(binding, cancellationToken) != 1)
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(
                new ConflictError("Edge Agent target is already enrolled."));
        }
        await unitOfWork.EdgeAgents.MarkEnrollmentUsedAsync(enrollment.Id, now, cancellationToken);
        if (edgePlatform?.PlatformDescriptor is DockerPlatformDescriptor descriptor)
        {
            edgePlatform.PartialUpdate(
                descriptor: descriptor with { DaemonId = request.DaemonId.Trim() });
            await unitOfWork.Platforms.UpdateAsync(
                edgePlatform,
                cancellationToken);
        }

        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new EdgeAgentEnrollmentCompleteResult(
            enrollment.PlatformId,
            agentId,
            enrollment.NormalizedResourceType,
            enrollment.NormalizedResourceId));
    }

    private static async Task<Result<EdgeAgentEnrollmentCompleteResult>> CompleteSwarmNodeEnrollmentAsync(
        EdgeAgentEnrollmentRequest request,
        DateTime utcNow,
        IUnitOfWork unitOfWork,
        CancellationToken cancellationToken)
    {
        if (!HasRequiredCommands(request.CapabilitiesJson, RequiredSwarmNodeCommands, out var missingCommand))
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(
                new BadRequestError($"Swarm Node Agent must advertise capability '{missingCommand}'."));
        }

        var bootstrap = await unitOfWork.EdgeAgents.GetActiveNodeAgentBootstrapByTokenHashAsync(
            HashToken(request.EnrollmentToken),
            utcNow,
            cancellationToken);
        if (bootstrap is null)
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(
                new UnauthorizedError("Node Agent bootstrap credential is invalid, expired, or revoked."));
        }

        var observation = await ValidateSwarmNodeObservationAsync(
            bootstrap.PlatformId,
            request.ClusterId,
            request.DockerNodeId,
            request.DaemonId,
            request.DockerHostname,
            request.SwarmRole,
            request.ServiceId,
            request.TaskId,
            unitOfWork,
            cancellationToken);
        if (observation.IsFailure(out var observationError))
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(observationError!);

        if (!string.Equals(bootstrap.ClusterId, request.ClusterId, StringComparison.Ordinal))
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(
                new UnauthorizedError("Node Agent bootstrap credential belongs to a different Swarm cluster."));
        }

        var dockerNodeId = request.DockerNodeId!.Trim();
        var existingNodeBinding = await unitOfWork.EdgeAgents.GetNodeBindingAsync(
            bootstrap.PlatformId,
            dockerNodeId,
            cancellationToken);
        if (existingNodeBinding is not null)
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(
                new ConflictError("This Swarm node already has an active Agent identity."));
        }

        var dockerDaemonId = request.DaemonId.Trim();
        var daemonBinding = await unitOfWork.EdgeAgents.GetActiveBindingByDockerDaemonIdAsync(
            dockerDaemonId,
            cancellationToken);
        if (daemonBinding is not null)
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(
                new ConflictError("This Docker daemon is already claimed by an active Agent identity."));
        }

        byte[] publicKeyBytes;
        try
        {
            publicKeyBytes = Convert.FromBase64String(request.AgentPublicKey);
        }
        catch (FormatException)
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(
                new BadRequestError("Agent public key is invalid."));
        }

        var fingerprint = GetFingerprint(publicKeyBytes);
        if (!string.IsNullOrWhiteSpace(request.AgentFingerprint)
            && !string.Equals(request.AgentFingerprint, fingerprint, StringComparison.Ordinal))
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(
                new BadRequestError("Agent fingerprint does not match the provided public key."));
        }

        var agentId = Guid.CreateVersion7();
        var binding = new EdgeAgentBinding(
            Id: Guid.CreateVersion7(),
            PlatformId: bootstrap.PlatformId,
            ResourceType: EdgeAgentResourceType.Platform,
            ResourceId: bootstrap.PlatformId,
            AgentId: agentId,
            AgentPublicKey: request.AgentPublicKey,
            AgentFingerprint: fingerprint,
            ConnectionStatus: EdgeAgentConnectionStatus.Offline,
            LastConnectedAtUtc: null,
            LastDisconnectedAtUtc: null,
            LastHeartbeatAtUtc: null,
            LastSeenVersion: request.AgentVersion,
            LastSeenHostname: request.Hostname,
            CapabilitiesJson: string.IsNullOrWhiteSpace(request.CapabilitiesJson) ? "{}" : request.CapabilitiesJson,
            ProtocolVersion: request.ProtocolVersion,
            RevokedAtUtc: null,
            CreatedAtUtc: utcNow,
            UpdatedAtUtc: utcNow,
            Profile: EdgeAgentProfile.SwarmNode,
            ClusterId: request.ClusterId!.Trim(),
            DockerNodeId: dockerNodeId,
            DockerDaemonId: dockerDaemonId,
            DockerHostname: request.DockerHostname!.Trim(),
            SwarmRole: request.SwarmRole!.Trim(),
            LastObservedServiceId: request.ServiceId!.Trim(),
            LastObservedTaskId: request.TaskId!.Trim(),
            FirstEnrolledAtUtc: utcNow,
            LastAuthenticatedAtUtc: null,
            RevocationReason: null);

        if (await unitOfWork.EdgeAgents.AddBindingAsync(binding, cancellationToken) != 1)
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(
                new ConflictError("Node identity conflicts with an active Agent binding."));
        }

        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success(new EdgeAgentEnrollmentCompleteResult(
            bootstrap.PlatformId,
            agentId,
            EdgeAgentResourceType.Platform,
            bootstrap.PlatformId,
            EdgeAgentProfile.SwarmNode,
            dockerNodeId));
    }

    public async Task<Result<EdgeAgentBinding>> GetReconnectBindingAsync(Guid platformId, Guid agentId, string agentFingerprint, string daemonId, CancellationToken cancellationToken)
        => await GetReconnectBindingAsync(EdgeAgentResourceType.Platform, platformId, agentId, agentFingerprint, daemonId, cancellationToken);

    public async Task<Result<EdgeAgentBinding>> GetReconnectBindingAsync(
        EdgeAgentResourceType resourceType,
        Guid resourceId,
        Guid agentId,
        string agentFingerprint,
        string daemonId,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var binding = await unitOfWork.EdgeAgents.GetBindingByAgentAsync(resourceType, resourceId, agentId, cancellationToken);
        if (binding is null)
        {
            return Result.Failure<EdgeAgentBinding>(new UnauthorizedError("Edge Agent binding was not found."));
        }

        if (binding.IsRevoked)
        {
            return Result.Failure<EdgeAgentBinding>(new UnauthorizedError("Edge Agent binding is revoked."));
        }

        if (!string.Equals(binding.AgentFingerprint, agentFingerprint, StringComparison.Ordinal))
        {
            return Result.Failure<EdgeAgentBinding>(new UnauthorizedError("Edge Agent fingerprint does not match."));
        }

        if (resourceType == EdgeAgentResourceType.Platform)
        {
            var daemonValidation = await ValidatePlatformDaemonAsync(
                unitOfWork,
                resourceId,
                daemonId,
                cancellationToken);
            if (daemonValidation.IsFailure(out var daemonError))
            {
                return Result.Failure<EdgeAgentBinding>(daemonError!);
            }
        }

        return Result.Success(binding);
    }

    public async Task<Result<EdgeAgentBinding>> GetSwarmNodeReconnectBindingAsync(
        Guid platformId,
        Guid agentId,
        string agentFingerprint,
        EdgeAgentHeartbeatSnapshot identity,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var binding = await unitOfWork.EdgeAgents.GetBindingByAgentAsync(
            EdgeAgentResourceType.Platform,
            platformId,
            agentId,
            cancellationToken);
        if (binding is null || binding.IsRevoked || binding.Profile != EdgeAgentProfile.SwarmNode)
            return Result.Failure<EdgeAgentBinding>(new UnauthorizedError("Node Agent binding was not found or is revoked."));

        if (!string.Equals(binding.AgentFingerprint, agentFingerprint, StringComparison.Ordinal))
            return Result.Failure<EdgeAgentBinding>(new UnauthorizedError("Node Agent fingerprint does not match."));
        if (!string.Equals(binding.DockerDaemonId, identity.DockerDaemonId, StringComparison.Ordinal)
            || !string.Equals(binding.ClusterId, identity.ClusterId, StringComparison.Ordinal))
        {
            return Result.Failure<EdgeAgentBinding>(
                new ConflictError("Node Agent local Docker identity does not match its persisted binding."));
        }

        if (!string.Equals(binding.DockerNodeId, identity.DockerNodeId, StringComparison.Ordinal))
        {
            return await TryRebindSwarmNodeAsync(
                binding,
                identity,
                utcNow: DateTime.UtcNow,
                unitOfWork,
                cancellationToken);
        }

        var observation = await ValidateSwarmNodeObservationAsync(
            platformId,
            identity.ClusterId,
            identity.DockerNodeId,
            identity.DockerDaemonId,
            identity.DockerHostname,
            identity.SwarmRole,
            identity.ServiceId,
            identity.TaskId,
            unitOfWork,
            cancellationToken);
        return observation.IsFailure(out var error)
            ? Result.Failure<EdgeAgentBinding>(error!)
            : Result.Success(binding);
    }

    private async Task<Result<EdgeAgentBinding>> TryRebindSwarmNodeAsync(
        EdgeAgentBinding binding,
        EdgeAgentHeartbeatSnapshot identity,
        DateTime utcNow,
        IUnitOfWork unitOfWork,
        CancellationToken cancellationToken)
    {
        var observation = await ValidateSwarmNodeObservationAsync(
            binding.PlatformId,
            identity.ClusterId,
            identity.DockerNodeId,
            identity.DockerDaemonId,
            identity.DockerHostname,
            identity.SwarmRole,
            identity.ServiceId,
            identity.TaskId,
            unitOfWork,
            cancellationToken);
        if (observation.IsFailure(out var observationError))
            return Result.Failure<EdgeAgentBinding>(observationError!);

        var nodes = await unitOfWork.Swarm.GetNodesAsync(binding.PlatformId, cancellationToken);
        if (nodes.Any(static node => node.IsStale))
        {
            return Result.Failure<EdgeAgentBinding>(
                new ConflictError("Node Agent identity cannot be rebound while Swarm membership is stale."));
        }
        if (nodes.Any(node => string.Equals(node.DockerNodeId, binding.DockerNodeId, StringComparison.Ordinal)))
        {
            return Result.Failure<EdgeAgentBinding>(
                new ConflictError("The previous Swarm node is still a current cluster member."));
        }

        var lastObserved = GetLastObservedAtUtc(binding);
        var removalGrace = TimeSpan.FromMinutes(Math.Clamp(
            edgeAgentOptions.Value.NodeAgentRemovalGraceMinutes,
            1,
            1_440));
        if (lastObserved > utcNow.Subtract(removalGrace))
        {
            return Result.Failure<EdgeAgentBinding>(
                new ConflictError("The previous Swarm node has not been absent beyond the configured removal grace period."));
        }

        var newNodeId = identity.DockerNodeId!.Trim();
        var nodeBinding = await unitOfWork.EdgeAgents.GetNodeBindingAsync(
            binding.PlatformId,
            newNodeId,
            cancellationToken);
        if (nodeBinding is not null && nodeBinding.Id != binding.Id)
        {
            return Result.Failure<EdgeAgentBinding>(
                new ConflictError("The current Swarm node already has an active Agent identity."));
        }

        var daemonBinding = await unitOfWork.EdgeAgents.GetActiveBindingByDockerDaemonIdAsync(
            identity.DockerDaemonId!.Trim(),
            cancellationToken);
        if (daemonBinding is not null && daemonBinding.Id != binding.Id)
        {
            return Result.Failure<EdgeAgentBinding>(
                new ConflictError("The Docker daemon is already claimed by another active Agent identity."));
        }

        var dockerHostname = identity.DockerHostname!.Trim();
        var swarmRole = identity.SwarmRole!.Trim();
        var serviceId = identity.ServiceId!.Trim();
        var taskId = identity.TaskId!.Trim();
        if (await unitOfWork.EdgeAgents.RebindNodeAsync(
                binding.Id,
                binding.DockerNodeId!,
                newNodeId,
                dockerHostname,
                swarmRole,
                serviceId,
                taskId,
                utcNow,
                cancellationToken) != 1)
        {
            return Result.Failure<EdgeAgentBinding>(
                new ConflictError("Node Agent identity changed while the rebind was being validated."));
        }

        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success(binding with
        {
            DockerNodeId = newNodeId,
            DockerHostname = dockerHostname,
            SwarmRole = swarmRole,
            LastObservedServiceId = serviceId,
            LastObservedTaskId = taskId,
            ConnectionStatus = EdgeAgentConnectionStatus.Offline,
            LastDisconnectedAtUtc = null,
            UpdatedAtUtc = utcNow
        });
    }

    private static DateTime GetLastObservedAtUtc(EdgeAgentBinding binding)
    {
        var result = binding.UpdatedAtUtc;
        if (binding.LastConnectedAtUtc is { } connected && connected > result)
            result = connected;
        if (binding.LastDisconnectedAtUtc is { } disconnected && disconnected > result)
            result = disconnected;
        if (binding.LastHeartbeatAtUtc is { } heartbeat && heartbeat > result)
            result = heartbeat;
        return result;
    }

    public async Task MarkConnectedAsync(Guid platformId, string hostname, string agentVersion, string capabilitiesJson, DateTime utcNow, CancellationToken cancellationToken)
        => await MarkConnectedAsync(EdgeAgentResourceType.Platform, platformId, hostname, agentVersion, capabilitiesJson, utcNow, cancellationToken);

    public async Task MarkConnectedAsync(EdgeAgentResourceType resourceType, Guid resourceId, string hostname, string agentVersion, string capabilitiesJson, DateTime utcNow, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var state = resourceType == EdgeAgentResourceType.Platform
            ? await unitOfWork.EdgeAgents.GetPlatformStateByPlatformIdAsync(resourceId, cancellationToken)
            : null;
        var platform = state?.Platform;
        var previousBinding = state?.Binding;
        var previousStatus = platform?.Status;
        var wasConnected = previousBinding?.ConnectionStatus == EdgeAgentConnectionStatus.Connected;

        await unitOfWork.EdgeAgents.UpdateBindingConnectedAsync(resourceType, resourceId, utcNow, hostname, agentVersion, capabilitiesJson, cancellationToken);

        if (platform is not null)
        {
            platform.PartialUpdate(platformStatus: PlatformStatus.Online, agentVersion: agentVersion);
            await unitOfWork.Platforms.UpdateAsync(platform, cancellationToken);
        }

        var activity = platform is null || (previousStatus == PlatformStatus.Online && wasConnected)
            ? null
            : PlatformActivity.Connected(platform, previousStatus!.Value, Constants.SystemId);

        if (activity is not null)
        {
            await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        }

        await unitOfWork.CommitAsync(cancellationToken);

        if (platform is not null)
        {
            await notificationQueue.EnqueueAsync(new PushPlatformUpdateNotificationWorkItem(platformStreamManager, platform), cancellationToken);
        }

        if (activity is not null)
        {
            await notificationQueue.EnqueueAsync(
                new ActivityNotificationWorkItem(activityStreamManager, await activity.AssignActor(unitOfWork, cancellationToken)),
                cancellationToken);
        }
    }

    public async Task MarkSwarmNodeConnectedAsync(
        Guid platformId,
        string dockerNodeId,
        string hostname,
        string agentVersion,
        string capabilitiesJson,
        string? serviceId,
        string? taskId,
        DateTime utcNow,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await unitOfWork.EdgeAgents.UpdateNodeBindingConnectedAsync(
            platformId,
            dockerNodeId,
            utcNow,
            hostname,
            agentVersion,
            capabilitiesJson,
            serviceId,
            taskId,
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await hubDispatcher.SendSwarmNodeAgentCoverageChanged(platformId, cancellationToken);
    }

    public async Task MarkHeartbeatAsync(Guid platformId, EdgeAgentHeartbeatSnapshot heartbeat, DateTime utcNow, CancellationToken cancellationToken)
        => await MarkHeartbeatAsync(EdgeAgentResourceType.Platform, platformId, heartbeat, utcNow, cancellationToken);

    public async Task MarkHeartbeatAsync(EdgeAgentResourceType resourceType, Guid resourceId, EdgeAgentHeartbeatSnapshot heartbeat, DateTime utcNow, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await unitOfWork.EdgeAgents.UpdateBindingHeartbeatAsync(resourceType, resourceId, utcNow, heartbeat.Hostname, heartbeat.AgentVersion, heartbeat.CapabilitiesJson, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
    }

    public async Task MarkSwarmNodeHeartbeatAsync(
        Guid platformId,
        string dockerNodeId,
        EdgeAgentHeartbeatSnapshot heartbeat,
        DateTime utcNow,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var binding = await unitOfWork.EdgeAgents.GetNodeBindingAsync(
            platformId,
            dockerNodeId,
            cancellationToken);
        if (binding is null
            || binding.IsRevoked
            || !string.Equals(heartbeat.DockerNodeId, binding.DockerNodeId, StringComparison.Ordinal)
            || !string.Equals(heartbeat.DockerDaemonId, binding.DockerDaemonId, StringComparison.Ordinal)
            || !string.Equals(heartbeat.ClusterId, binding.ClusterId, StringComparison.Ordinal))
        {
            throw new InvalidOperationException(
                "NodeIdentityConflict: the heartbeat identity does not match the authenticated Swarm Node binding.");
        }

        await unitOfWork.EdgeAgents.UpdateNodeBindingHeartbeatAsync(
            platformId,
            dockerNodeId,
            utcNow,
            heartbeat.DockerHostname ?? heartbeat.Hostname,
            heartbeat.AgentVersion,
            heartbeat.CapabilitiesJson,
            heartbeat.ServiceId,
            heartbeat.TaskId,
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
    }

    public async Task MarkDisconnectedAsync(Guid platformId, DateTime utcNow, CancellationToken cancellationToken)
        => await MarkDisconnectedAsync(EdgeAgentResourceType.Platform, platformId, utcNow, cancellationToken);

    public async Task MarkDisconnectedAsync(EdgeAgentResourceType resourceType, Guid resourceId, DateTime utcNow, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var state = resourceType == EdgeAgentResourceType.Platform
            ? await unitOfWork.EdgeAgents.GetPlatformStateByPlatformIdAsync(resourceId, cancellationToken)
            : null;
        var platform = state?.Platform;
        var previousBinding = state?.Binding;
        var previousStatus = platform?.Status;
        var wasConnected = previousBinding?.ConnectionStatus == EdgeAgentConnectionStatus.Connected;

        await unitOfWork.EdgeAgents.UpdateBindingDisconnectedAsync(resourceType, resourceId, utcNow, cancellationToken);

        if (platform is not null)
        {
            platform.PartialUpdate(platformStatus: PlatformStatus.Offline);
            await unitOfWork.Platforms.UpdateAsync(platform, cancellationToken);
        }

        var activity = platform is null || (previousStatus == PlatformStatus.Offline && !wasConnected)
            ? null
            : PlatformActivity.Disconnected(platform, previousStatus!.Value, Constants.SystemId);

        if (activity is not null)
        {
            await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        }

        await unitOfWork.CommitAsync(cancellationToken);

        if (platform is not null)
        {
            await notificationQueue.EnqueueAsync(new PushPlatformUpdateNotificationWorkItem(platformStreamManager, platform), cancellationToken);
        }

        if (activity is not null)
        {
            await notificationQueue.EnqueueAsync(
                new ActivityNotificationWorkItem(activityStreamManager, await activity.AssignActor(unitOfWork, cancellationToken)),
                cancellationToken);
        }
    }

    public async Task MarkSwarmNodeDisconnectedAsync(Guid platformId, string dockerNodeId, DateTime utcNow, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await unitOfWork.EdgeAgents.UpdateNodeBindingDisconnectedAsync(
            platformId,
            dockerNodeId,
            utcNow,
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await hubDispatcher.SendSwarmNodeAgentCoverageChanged(platformId, cancellationToken);
    }

    public async Task<Result> RevokeAsync(Guid platformId, DateTime utcNow, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
        if (platform is null)
        {
            return Result.Failure(new NotFoundError("Platform not found."));
        }

        if (platform.ConnectorType != PlatformConnectorType.EdgeAgent)
        {
            return Result.Failure(new BadRequestError("Platform is not an Edge Agent platform."));
        }

        await unitOfWork.EdgeAgents.RevokeBindingAsync(platformId, utcNow, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success();
    }

    public async Task<Result> RevokeBuildAgentPoolAsync(Guid buildAgentPoolId, DateTime utcNow, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var pool = await unitOfWork.BuildAgentPools.GetAsync(buildAgentPoolId, cancellationToken, includeArchived: true);
        if (pool is null)
        {
            return Result.Failure(new NotFoundError("Build pool not found."));
        }

        await unitOfWork.EdgeAgents.RevokeBindingAsync(EdgeAgentResourceType.BuildAgentPool, buildAgentPoolId, utcNow, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success();
    }

    private static async Task<Result> ValidateSwarmNodeObservationAsync(
        Guid platformId,
        string? clusterId,
        string? dockerNodeId,
        string? dockerDaemonId,
        string? dockerHostname,
        string? swarmRole,
        string? serviceId,
        string? taskId,
        IUnitOfWork unitOfWork,
        CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(clusterId)
            || string.IsNullOrWhiteSpace(dockerNodeId)
            || string.IsNullOrWhiteSpace(dockerDaemonId)
            || string.IsNullOrWhiteSpace(dockerHostname)
            || string.IsNullOrWhiteSpace(swarmRole)
            || string.IsNullOrWhiteSpace(serviceId)
            || string.IsNullOrWhiteSpace(taskId))
        {
            return Result.Failure(new BadRequestError("Node Agent did not report its complete Docker and Swarm identity."));
        }

        var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
        if (platform?.PlatformDescriptor is not DockerSwarmPlatformDescriptor descriptor)
            return Result.Failure(new BadRequestError("Node Agent target is not a Docker Swarm platform."));
        if (!string.Equals(platform.ClusterId, clusterId.Trim(), StringComparison.Ordinal))
            return Result.Failure(new ConflictError("Node Agent reported a different Swarm cluster."));

        var installation = await unitOfWork.EdgeAgents.GetNodeAgentInstallationAsync(platformId, cancellationToken);
        if (installation is null
            || installation.DesiredState != SwarmNodeAgentDesiredState.Installed
            || string.IsNullOrWhiteSpace(installation.DockerServiceId))
        {
            return Result.Failure(new UnauthorizedError("Swarm node agents are not installed for this platform."));
        }
        if (!string.Equals(installation.ClusterId, clusterId.Trim(), StringComparison.Ordinal)
            || !string.Equals(installation.DockerServiceId, serviceId.Trim(), StringComparison.Ordinal))
        {
            return Result.Failure(new UnauthorizedError("Node Agent Service identity is not owned by this platform."));
        }
        if (string.Equals(descriptor.NodeID, dockerNodeId.Trim(), StringComparison.Ordinal))
            return Result.Failure(new ConflictError("The pinned manager must use the control-plane connector, not a satellite Agent."));

        var node = await unitOfWork.Swarm.GetNodeAsync(platformId, dockerNodeId.Trim(), cancellationToken);
        if (node is null || node.IsStale)
            return Result.Failure(new UnauthorizedError("Node Agent did not report a current Swarm node."));
        if (!string.Equals(node.Status, "ready", StringComparison.OrdinalIgnoreCase)
            || !string.Equals(node.Availability, "active", StringComparison.OrdinalIgnoreCase)
            || !string.Equals(node.OperatingSystem, "linux", StringComparison.OrdinalIgnoreCase))
        {
            return Result.Failure(new ConflictError("Swarm node is not currently eligible for a satellite Agent."));
        }
        if (!string.Equals(node.Hostname, dockerHostname.Trim(), StringComparison.Ordinal)
            || !string.Equals(node.Role, swarmRole.Trim(), StringComparison.OrdinalIgnoreCase))
        {
            return Result.Failure(new ConflictError("Node Agent hostname or role does not match manager inventory."));
        }

        var service = await unitOfWork.Swarm.GetServiceAsync(platformId, serviceId.Trim(), cancellationToken);
        if (service is null || service.IsStale
            || !service.Labels.TryGetValue("com.citadel.system", out var systemValue)
            || !string.Equals(systemValue, "true", StringComparison.OrdinalIgnoreCase)
            || !service.Labels.TryGetValue("com.citadel.system-role", out var roleValue)
            || !string.Equals(roleValue, "swarm-node-agent", StringComparison.Ordinal)
            || !service.Labels.TryGetValue("com.citadel.platform-id", out var servicePlatformId)
            || !string.Equals(servicePlatformId, platformId.ToString("D"), StringComparison.OrdinalIgnoreCase))
        {
            return Result.Failure(new UnauthorizedError("Node Agent Service is missing valid Citadel ownership labels."));
        }

        var task = await unitOfWork.Swarm.GetTaskAsync(platformId, taskId.Trim(), cancellationToken);
        if (task is null || task.IsStale
            || !string.Equals(task.DockerServiceId, serviceId.Trim(), StringComparison.Ordinal)
            || !string.Equals(task.DockerNodeId, dockerNodeId.Trim(), StringComparison.Ordinal)
            || !string.Equals(task.DesiredState, "running", StringComparison.OrdinalIgnoreCase)
            || !string.Equals(task.State, "running", StringComparison.OrdinalIgnoreCase))
        {
            return Result.Failure(new UnauthorizedError("Node Agent task is not a current running task on the reported Swarm node."));
        }

        return Result.Success();
    }

    private static async Task<Result> ValidateTargetAsync(
        EdgeAgentTarget target,
        IUnitOfWork unitOfWork,
        CancellationToken cancellationToken)
    {
        if (target.ResourceType == EdgeAgentResourceType.Platform)
        {
            var platform = await unitOfWork.Platforms.GetByIdAsync(target.ResourceId, cancellationToken);
            if (platform is null)
                return Result.Failure(new NotFoundError("Platform not found."));

            return platform.ConnectorType == PlatformConnectorType.EdgeAgent
                ? Result.Success()
                : Result.Failure(new BadRequestError("Platform is not an Edge Agent platform."));
        }

        if (target.ResourceType == EdgeAgentResourceType.BuildAgentPool)
        {
            var pool = await unitOfWork.BuildAgentPools.GetAsync(target.ResourceId, cancellationToken, includeArchived: false);
            if (pool is null)
                return Result.Failure(new NotFoundError("Build pool not found."));

            if (pool.ProviderSpec is not SelfManagedVmBuildAgentPoolProviderSpec { ConnectionMode: BuildAgentPoolConnectionMode.EdgeAgent })
                return Result.Failure(new BadRequestError("Build pool is not configured for Edge Agent mode."));

            return Result.Success();
        }

        return Result.Failure(new BadRequestError("Edge Agent target type is invalid."));
    }

    private static async Task<Result<Platform>> ValidatePlatformDaemonAsync(
        IUnitOfWork unitOfWork,
        Guid platformId,
        string daemonId,
        CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(daemonId))
        {
            return Result.Failure<Platform>(
                new BadRequestError(
                    "Edge Agent did not report a Docker daemon id."));
        }

        var platform = await unitOfWork.Platforms.GetByIdAsync(
            platformId,
            cancellationToken);
        if (platform is null)
        {
            return Result.Failure<Platform>(
                new NotFoundError("Platform not found."));
        }

        if (platform.PlatformDescriptor is not DockerPlatformDescriptor descriptor)
        {
            return Result.Failure<Platform>(
                new BadRequestError(
                    "Platform does not have a Docker daemon identity."));
        }

        var normalizedDaemonId = daemonId.Trim();
        var storedDaemonId = descriptor.DaemonId?.Trim() ?? string.Empty;
        if (storedDaemonId.Length > 0
            && !string.Equals(
                storedDaemonId,
                normalizedDaemonId,
                StringComparison.Ordinal))
        {
            return Result.Failure<Platform>(
                new ConflictError(
                    "This Edge Agent is connected to a different Docker engine than the platform."));
        }

        var existingPlatform =
            await unitOfWork.Platforms.GetByDaemonIdAsync(
                normalizedDaemonId,
                platformId,
                cancellationToken);
        if (existingPlatform is not null)
        {
            return Result.Failure<Platform>(
                new ConflictError(
                    $"This Docker engine is already registered as platform '{existingPlatform.Name}'."));
        }

        return Result.Success(platform);
    }

    public static string HashToken(string token)
    {
        var hash = SHA256.HashData(Encoding.UTF8.GetBytes(token));
        return Base64Url(hash);
    }

    public static string GetFingerprint(ReadOnlySpan<byte> publicKey)
    {
        var hash = SHA256.HashData(publicKey);
        return $"SHA256:{Convert.ToHexString(hash).ToLowerInvariant()}";
    }

    public static string ShortFingerprint(string fingerprint)
    {
        const int prefixLength = 7;
        if (!fingerprint.StartsWith("SHA256:", StringComparison.Ordinal) || fingerprint.Length <= prefixLength + 16)
        {
            return fingerprint;
        }

        var value = fingerprint[prefixLength..];
        return $"SHA256:{value[..8]}...{value[^4..]}";
    }

    private static string GenerateToken()
    {
        Span<byte> bytes = stackalloc byte[32];
        RandomNumberGenerator.Fill(bytes);
        return Base64Url(bytes);
    }

    private static string Base64Url(ReadOnlySpan<byte> bytes)
    {
        return Convert.ToBase64String(bytes)
            .TrimEnd('=')
            .Replace('+', '-')
            .Replace('/', '_');
    }

    private static bool HasRequiredCommands(string? capabilitiesJson, IReadOnlyCollection<string> requiredCommands, out string missingCommand)
    {
        missingCommand = string.Empty;
        if (string.IsNullOrWhiteSpace(capabilitiesJson))
        {
            missingCommand = requiredCommands.FirstOrDefault() ?? string.Empty;
            return false;
        }

        try
        {
            using var document = JsonDocument.Parse(capabilitiesJson);
            if (document.RootElement.ValueKind != JsonValueKind.Object ||
                !document.RootElement.TryGetProperty("commands", out var commandsElement) ||
                commandsElement.ValueKind != JsonValueKind.Array)
            {
                missingCommand = requiredCommands.FirstOrDefault() ?? string.Empty;
                return false;
            }

            var commands = new HashSet<string>(StringComparer.Ordinal);
            foreach (var command in commandsElement.EnumerateArray())
            {
                if (command.ValueKind == JsonValueKind.String &&
                    !string.IsNullOrWhiteSpace(command.GetString()))
                {
                    commands.Add(command.GetString()!);
                }
            }

            missingCommand = requiredCommands.FirstOrDefault(command => !commands.Contains(command)) ?? string.Empty;
            return missingCommand.Length == 0;
        }
        catch (JsonException)
        {
            missingCommand = requiredCommands.FirstOrDefault() ?? string.Empty;
            return false;
        }
    }

    private sealed record EdgeAgentTarget(
        EdgeAgentResourceType ResourceType,
        Guid ResourceId,
        Guid PlatformId,
        string ContainerName,
        string KeyPath,
        string IdentityPath);
}
