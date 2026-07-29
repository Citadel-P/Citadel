using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Application.Configs;
using Application.Features.Deployments.Notifications;
using Application.Features.Platforms;
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
    IOptions<EdgeAgentOptions> edgeAgentOptions) : IEdgeAgentManagementService
{
    private static readonly string[] RequiredBuildAgentPoolCommands =
    [
        "images.build",
        "images.push",
        "images.checkBuildHost"
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

        await unitOfWork.EdgeAgents.AddBindingAsync(binding, cancellationToken);
        await unitOfWork.EdgeAgents.MarkEnrollmentUsedAsync(enrollment.Id, now, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new EdgeAgentEnrollmentCompleteResult(
            enrollment.PlatformId,
            agentId,
            enrollment.NormalizedResourceType,
            enrollment.NormalizedResourceId));
    }

    public async Task<Result<EdgeAgentBinding>> GetReconnectBindingAsync(Guid platformId, Guid agentId, string agentFingerprint, CancellationToken cancellationToken)
        => await GetReconnectBindingAsync(EdgeAgentResourceType.Platform, platformId, agentId, agentFingerprint, cancellationToken);

    public async Task<Result<EdgeAgentBinding>> GetReconnectBindingAsync(
        EdgeAgentResourceType resourceType,
        Guid resourceId,
        Guid agentId,
        string agentFingerprint,
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

        return Result.Success(binding);
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

    public async Task MarkHeartbeatAsync(Guid platformId, EdgeAgentHeartbeatSnapshot heartbeat, DateTime utcNow, CancellationToken cancellationToken)
        => await MarkHeartbeatAsync(EdgeAgentResourceType.Platform, platformId, heartbeat, utcNow, cancellationToken);

    public async Task MarkHeartbeatAsync(EdgeAgentResourceType resourceType, Guid resourceId, EdgeAgentHeartbeatSnapshot heartbeat, DateTime utcNow, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await unitOfWork.EdgeAgents.UpdateBindingHeartbeatAsync(resourceType, resourceId, utcNow, heartbeat.Hostname, heartbeat.AgentVersion, heartbeat.CapabilitiesJson, cancellationToken);
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
