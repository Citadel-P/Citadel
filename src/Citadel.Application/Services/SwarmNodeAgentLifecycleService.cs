using System.Security.Cryptography;
using System.Text;
using Application.Configs;
using Application.Features.Platforms;
using Application.Services.Abstractions;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Platforms;
using Domain.Contracts.Resources.Swarm;
using Domain.Configs;
using Domain.Entities.Platforms;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.Services;

internal interface ISwarmNodeAgentLifecycleService
{
    Task<Result<Guid>> ExecuteAsync(
        Guid platformId,
        SwarmNodeAgentOperationKind kind,
        string coreUrl,
        Guid actorId,
        Action<SwarmNodeAgentProgressItem> report,
        CancellationToken cancellationToken);
}

internal sealed class SwarmNodeAgentLifecycleService(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> swarmConnectorFactory,
    IConnectorFactory<IImageConnector> imageConnectorFactory,
    ISwarmManagerIdentityValidator managerIdentityValidator,
    ISwarmReconciliationCoordinator reconciliationCoordinator,
    IEdgeAgentSessionStatus sessionStatus,
    IEdgeAgentSessionTerminator sessionTerminator,
    IApplicationHubDispatcher hubDispatcher,
    IOptions<EdgeAgentOptions> options,
    IOptions<AgentTransportOptions> transportOptions,
    ILogger<SwarmNodeAgentLifecycleService> logger) : ISwarmNodeAgentLifecycleService
{
    public async Task<Result<Guid>> ExecuteAsync(
        Guid platformId,
        SwarmNodeAgentOperationKind kind,
        string coreUrl,
        Guid actorId,
        Action<SwarmNodeAgentProgressItem> report,
        CancellationToken cancellationToken)
    {
        var operationId = Guid.CreateVersion7();
        var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
        if (platform is null)
            return Result.Failure<Guid>(new NotFoundError("Platform does not exist."));
        if (platform.PlatformDescriptor is not DockerSwarmPlatformDescriptor descriptor
            || string.IsNullOrWhiteSpace(platform.ClusterId)
            || string.IsNullOrWhiteSpace(descriptor.NodeID)
            || string.IsNullOrWhiteSpace(descriptor.DaemonId))
        {
            return Result.Failure<Guid>(new BadRequestError("The Swarm manager identity must be validated before managing node agents."));
        }
        if (platform.Status != PlatformStatus.Online)
            return Result.Failure<Guid>(new ConflictError("The Swarm manager is offline."));
        var managerIdentity = await managerIdentityValidator.ValidateAsync(platform, cancellationToken);
        if (managerIdentity.IsFailure(out var managerIdentityError))
            return Result.Failure<Guid>(managerIdentityError!);
        if (kind != SwarmNodeAgentOperationKind.Remove && !TryValidateCoreUrl(coreUrl, out var coreUrlError))
            return Result.Failure<Guid>(new BadRequestError(coreUrlError));

        var current = await unitOfWork.EdgeAgents.GetNodeAgentInstallationAsync(platformId, cancellationToken);
        if (kind != SwarmNodeAgentOperationKind.Remove
            && current is { DesiredState: SwarmNodeAgentDesiredState.Installed }
            && !string.Equals(current.ClusterId, platform.ClusterId, StringComparison.Ordinal))
        {
            return Result.Failure<Guid>(new ConflictError(
                "The Swarm cluster identity changed. Restore the original cluster connection before managing node agents."));
        }
        if (kind is SwarmNodeAgentOperationKind.Install
            && current is { DesiredState: SwarmNodeAgentDesiredState.Installed }
            && (!string.Equals(current.ManagerDockerNodeId, descriptor.NodeID, StringComparison.Ordinal)
                || !string.Equals(current.ManagerDockerDaemonId, descriptor.DaemonId, StringComparison.Ordinal)))
        {
            return Result.Failure<Guid>(new ConflictError(
                "The authorized Swarm manager changed. Run Repair coverage to hand over the node data plane."));
        }
        var now = DateTime.UtcNow;
        var resumeInterruptedOperation = current is
        {
            OperationState: SwarmNodeAgentOperationState.Running,
            OperationStartedAtUtc: { } startedAt
        } && startedAt < now.AddMinutes(-30);
        var imageReference = options.Value.GetAgentImage();
        var serviceName = $"citadel-node-agent-{platformId:N}";
        var running = new SwarmNodeAgentInstallation(
            platformId,
            platform.ClusterId,
            descriptor.NodeID,
            descriptor.DaemonId,
            current?.DockerServiceId,
            current?.DockerServiceName ?? serviceName,
            current?.AgentImageReference ?? imageReference,
            current?.AgentImageDigest ?? string.Empty,
            current?.DockerCaConfigId,
            current?.DockerCaConfigName,
            kind == SwarmNodeAgentOperationKind.Remove
                ? SwarmNodeAgentDesiredState.Removed
                : SwarmNodeAgentDesiredState.Installed,
            operationId,
            kind,
            SwarmNodeAgentOperationState.Running,
            now,
            actorId,
            null,
            current?.CreatedAtUtc ?? now,
            now);
        if (await unitOfWork.EdgeAgents.TryStartNodeAgentOperationAsync(running, cancellationToken) != 1)
        {
            return Result.Failure<Guid>(new ConflictError("A node-agent lifecycle operation is already running for this platform."));
        }
        await unitOfWork.ActivityEventRepository.AddAsync(
            PlatformActivity.NodeAgentLifecycle(
                platform,
                actorId,
                kind,
                operationId,
                SwarmNodeAgentOperationState.Running,
                "Node-agent lifecycle operation requested."),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await TryNotifyCoverageChangedAsync(platformId, cancellationToken);

        report(new(platformId, operationId, "validation", "Validated the pinned Swarm manager and cluster identity."));
        try
        {
            var result = kind == SwarmNodeAgentOperationKind.Remove
                ? await RemoveAsync(platform, current, running, report, cancellationToken)
                : await InstallOrRepairAsync(
                    platform,
                    running,
                    coreUrl,
                    resumeInterruptedOperation,
                    report,
                    cancellationToken);
            if (result.IsFailure(out var error))
            {
                if (kind != SwarmNodeAgentOperationKind.Remove)
                    await RevokeBootstrapsAfterFailureAsync(platform.Id);
                await TryCompleteFailedOperationAsync(platform, running, error!.Message);
                return Result.Failure<Guid>(error);
            }

            return Result.Success(operationId);
        }
        catch (OperationCanceledException)
        {
            if (kind != SwarmNodeAgentOperationKind.Remove)
                await RevokeBootstrapsAfterFailureAsync(platform.Id);
            await TryCompleteFailedOperationAsync(
                platform,
                running,
                "Node-agent operation was canceled; Repair coverage can safely reconcile the partial state.");
            throw;
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Swarm node-agent {OperationKind} failed for platform {PlatformId}", kind, platformId);
            if (kind != SwarmNodeAgentOperationKind.Remove)
                await RevokeBootstrapsAfterFailureAsync(platform.Id);
            await TryCompleteFailedOperationAsync(platform, running, ex.Message);
            return Result.Failure<Guid>(new InternalServerError("Node-agent lifecycle operation failed."));
        }
    }

    private async Task<Result> InstallOrRepairAsync(
        Platform platform,
        SwarmNodeAgentInstallation running,
        string coreUrl,
        bool resumeInterruptedOperation,
        Action<SwarmNodeAgentProgressItem> report,
        CancellationToken cancellationToken)
    {
        var descriptor = (DockerSwarmPlatformDescriptor)platform.PlatformDescriptor;
        await reconciliationCoordinator.RefreshAsync(platform.Id, cancellationToken);
        var nodes = await unitOfWork.Swarm.GetNodesAsync(platform.Id, cancellationToken);
        var managerBinding = await unitOfWork.EdgeAgents.GetNodeBindingAsync(
            platform.Id,
            descriptor.NodeID,
            cancellationToken);
        if (managerBinding is not null)
        {
            await unitOfWork.EdgeAgents.RevokeNodeBindingAsync(
                platform.Id,
                descriptor.NodeID,
                DateTime.UtcNow,
                "The node became the pinned control-plane manager.",
                cancellationToken);
            await unitOfWork.CommitAsync(cancellationToken);
            sessionTerminator.Disconnect(
                platform.Id,
                descriptor.NodeID,
                "The node became the pinned control-plane manager.");
            report(new(
                platform.Id,
                running.OperationId!.Value,
                "manager",
                "Moved the selected manager to the control-plane data source."));
        }
        var supportedArchitectures = new HashSet<string>(
            options.Value.SupportedNodeArchitectures
                .Where(static value => !string.IsNullOrWhiteSpace(value))
                .Select(NormalizeArchitecture),
            StringComparer.OrdinalIgnoreCase);
        var eligible = nodes
            .Where(node => !node.IsStale
                           && !string.Equals(node.DockerNodeId, descriptor.NodeID, StringComparison.Ordinal)
                           && string.Equals(node.Status, "ready", StringComparison.OrdinalIgnoreCase)
                           && string.Equals(node.Availability, "active", StringComparison.OrdinalIgnoreCase)
                           && string.Equals(node.OperatingSystem, "linux", StringComparison.OrdinalIgnoreCase)
                           && supportedArchitectures.Contains(NormalizeArchitecture(node.Architecture)))
            .ToArray();

        if (eligible.Length == 0)
        {
            report(new(platform.Id, running.OperationId!.Value, "coverage", "The pinned manager already provides complete node-local coverage; no satellite Service is required."));
            await CompleteOperationAsync(platform, running, SwarmNodeAgentOperationState.Completed, null, cancellationToken);
            return Result.Success();
        }

        var bindings = (await unitOfWork.EdgeAgents.GetNodeBindingsAsync(platform.Id, cancellationToken))
            .Where(static binding => !binding.IsRevoked && binding.DockerNodeId is not null)
            .ToDictionary(static binding => binding.DockerNodeId!, StringComparer.Ordinal);
        var missing = eligible
            .Where(node => !bindings.ContainsKey(node.DockerNodeId)
                           || !sessionStatus.IsNodeConnected(platform.Id, node.DockerNodeId))
            .ToArray();

        var eligibleArchitectures = eligible
            .Select(static node => NormalizeArchitecture(node.Architecture))
            .ToHashSet(StringComparer.OrdinalIgnoreCase);
        var imageResult = await ResolveImageAsync(platform, eligibleArchitectures, cancellationToken);
        if (!imageResult.IsSuccess(out var image, out var imageError))
            return Result.Failure(imageError!);
        report(new(platform.Id, running.OperationId!.Value, "image", $"Resolved node Agent image to {image.PinnedReference}."));

        var installation = running with
        {
            AgentImageReference = image.Reference,
            AgentImageDigest = image.Digest,
            UpdatedAtUtc = DateTime.UtcNow
        };
        var services = await unitOfWork.Swarm.GetServicesAsync(platform.Id, cancellationToken);
        var projectedService = string.IsNullOrWhiteSpace(installation.DockerServiceId)
            ? null
            : services.FirstOrDefault(service => string.Equals(
                service.DockerServiceId,
                installation.DockerServiceId,
                StringComparison.Ordinal));
        if (projectedService is null)
        {
            var serviceWithStableName = services.FirstOrDefault(service => string.Equals(
                service.Name,
                installation.DockerServiceName,
                StringComparison.Ordinal));
            if (serviceWithStableName is not null)
            {
                if (!SwarmNodeAgentInfrastructure.HasOwnership(serviceWithStableName, platform))
                {
                    return Result.Failure(new ConflictError(
                        $"A Docker Service named '{installation.DockerServiceName}' exists without Citadel node-agent ownership labels."));
                }

                projectedService = serviceWithStableName;
                installation = installation with
                {
                    DockerServiceId = serviceWithStableName.DockerServiceId,
                    UpdatedAtUtc = DateTime.UtcNow
                };
            }
        }

        SwarmNodeAgentBootstrap? bootstrap = null;
        byte[]? clearBootstrapBytes = null;
        if (missing.Length > 0)
        {
            if (resumeInterruptedOperation)
            {
                var latest = await unitOfWork.EdgeAgents.GetLatestNodeAgentBootstrapAsync(platform.Id, cancellationToken);
                if (latest is not null && latest.IsActive(DateTime.UtcNow))
                    bootstrap = await RecoverBootstrapSecretAsync(platform, latest, cancellationToken);
            }

            if (bootstrap is null)
            {
                var created = await CreateBootstrapAsync(platform, running.OperationActorId!.Value, cancellationToken);
                bootstrap = created.Bootstrap;
                clearBootstrapBytes = created.TokenBytes;
            }
            report(new(platform.Id, running.OperationId.Value, "bootstrap", $"Prepared a bounded enrollment window for {missing.Length} missing node(s)."));
        }
        else
        {
            bootstrap = await unitOfWork.EdgeAgents.GetLatestNodeAgentBootstrapAsync(platform.Id, cancellationToken);
            if (bootstrap is not null && bootstrap.DockerSecretId is null)
                bootstrap = await RecoverBootstrapSecretAsync(platform, bootstrap, cancellationToken);
            if (bootstrap?.DockerSecretId is null)
                return Result.Failure(new ConflictError("The installed node-agent Service has no owned bootstrap Secret. Run Repair coverage."));
        }

        try
        {
            var secretId = bootstrap.DockerSecretId;
            string? createdSecretId = null;
            if (secretId is null)
            {
                var secretResult = await swarmConnectorFactory.GetConnector(platform.ConnectorType).CreateSecretAsync(
                    new CreateSwarmSecretCommand(
                        platform.Address,
                        bootstrap.DockerSecretName,
                        clearBootstrapBytes!,
                        SwarmNodeAgentInfrastructure.OwnershipLabels(platform)),
                    cancellationToken);
                if (!secretResult.IsSuccess(out var createdSecret, out var secretError))
                    return Result.Failure(secretError!);
                secretId = createdSecret.ResourceId;
                createdSecretId = secretId;
                await unitOfWork.EdgeAgents.UpdateNodeAgentBootstrapSecretAsync(
                    bootstrap.Id,
                    secretId,
                    DateTime.UtcNow,
                    cancellationToken);
                await unitOfWork.CommitAsync(cancellationToken);
                bootstrap = bootstrap with { DockerSecretId = secretId, UpdatedAtUtc = DateTime.UtcNow };
            }

            var caConfigResult = await ResolveCaConfigAsync(platform, installation, report, cancellationToken);
            if (!caConfigResult.IsSuccess(out var caConfig, out var caConfigError))
            {
                if (createdSecretId is not null)
                    await DeleteUnreferencedBootstrapSecretAsync(platform, createdSecretId);
                return Result.Failure(caConfigError!);
            }
            installation = installation with
            {
                DockerCaConfigId = caConfig.ConfigId,
                DockerCaConfigName = caConfig.ConfigName,
                UpdatedAtUtc = DateTime.UtcNow
            };

            var serviceSpec = BuildSystemSpec(
                platform,
                descriptor,
                image.PinnedReference,
                coreUrl,
                bootstrap,
                eligibleArchitectures,
                caConfig.ConfigId,
                caConfig.ConfigName);
            var connector = swarmConnectorFactory.GetConnector(platform.ConnectorType);
            ManagedSwarmServiceMutationResult mutation;
            if (projectedService is null)
            {
                var create = await connector.CreateSystemServiceAsync(
                    new CreateSystemSwarmServiceCommand(
                        platform.Address,
                        running.OperationId.Value,
                        installation.DockerServiceName,
                        serviceSpec,
                        SwarmNodeAgentInfrastructure.OwnershipLabels(platform),
                        SwarmNodeAgentInfrastructure.OwnershipLabels(platform)),
                    cancellationToken);
                if (!create.IsSuccess(out mutation!, out var createError))
                    return Result.Failure(createError!);
            }
            else
            {
                if (!SwarmNodeAgentInfrastructure.HasOwnership(projectedService, platform))
                    return Result.Failure(new ConflictError("The persisted node-agent Service no longer has valid Citadel ownership labels."));
                var update = await connector.UpdateSystemServiceAsync(
                    new UpdateSystemSwarmServiceCommand(
                        platform.Address,
                        running.OperationId.Value,
                        projectedService.DockerServiceId,
                        projectedService.VersionIndex,
                        serviceSpec,
                        SwarmNodeAgentInfrastructure.OwnershipLabels(platform),
                        SwarmNodeAgentInfrastructure.OwnershipLabels(platform)),
                    cancellationToken);
                if (!update.IsSuccess(out mutation!, out var updateError))
                    return Result.Failure(updateError!);
            }

            installation = installation with
            {
                DockerServiceId = mutation.ServiceId ?? projectedService?.DockerServiceId ?? installation.DockerServiceId,
                UpdatedAtUtc = DateTime.UtcNow
            };
            await unitOfWork.EdgeAgents.UpsertNodeAgentInstallationAsync(installation, cancellationToken);
            if (clearBootstrapBytes is not null)
            {
                await unitOfWork.EdgeAgents.RevokeOtherNodeAgentBootstrapsAsync(
                    platform.Id,
                    bootstrap.Id,
                    DateTime.UtcNow,
                    cancellationToken);
            }
            await unitOfWork.CommitAsync(cancellationToken);
            report(new(platform.Id, running.OperationId.Value, "service", "Docker accepted the Citadel node-agent global Service."));

            if (!string.IsNullOrWhiteSpace(caConfig.PreviousConfigId)
                && !string.Equals(caConfig.PreviousConfigId, caConfig.ConfigId, StringComparison.Ordinal))
            {
                try
                {
                    using var cleanupTimeout = new CancellationTokenSource(TimeSpan.FromSeconds(15));
                    await DeleteOwnedCaConfigAsync(
                        platform,
                        caConfig.PreviousConfigId,
                        running.OperationId.Value,
                        report,
                        cleanupTimeout.Token);
                }
                catch (OperationCanceledException)
                {
                    report(new(
                        platform.Id,
                        running.OperationId.Value,
                        "cleanup",
                        $"The old CA Config '{caConfig.PreviousConfigId}' could not be cleaned up within the timeout.",
                        IsWarning: true));
                }
            }

            var convergence = await WaitForCoverageAsync(platform, eligible, installation, report, cancellationToken);
            using var revokeTimeout = new CancellationTokenSource(TimeSpan.FromSeconds(15));
            await unitOfWork.EdgeAgents.RevokeNodeAgentBootstrapsAsync(
                platform.Id,
                DateTime.UtcNow,
                revokeTimeout.Token);
            await unitOfWork.CommitAsync(revokeTimeout.Token);
            if (convergence.IsFailure(out var convergenceError))
                return Result.Failure(convergenceError!);

            try
            {
                using var secretCleanupTimeout = new CancellationTokenSource(TimeSpan.FromSeconds(15));
                await DeletePriorBootstrapSecretsAsync(
                    platform,
                    bootstrap.Id,
                    running.OperationId.Value,
                    report,
                    secretCleanupTimeout.Token);
            }
            catch (OperationCanceledException)
            {
                report(new(
                    platform.Id,
                    running.OperationId.Value,
                    "cleanup",
                    "Prior bootstrap Secret cleanup did not finish within the timeout.",
                    IsWarning: true));
            }

            await CompleteOperationAsync(platform, installation, SwarmNodeAgentOperationState.Completed, null, cancellationToken);
            return Result.Success();
        }
        finally
        {
            if (clearBootstrapBytes is not null)
            {
                CryptographicOperations.ZeroMemory(clearBootstrapBytes);
            }
        }
    }

    private async Task<Result> RemoveAsync(
        Platform platform,
        SwarmNodeAgentInstallation? current,
        SwarmNodeAgentInstallation running,
        Action<SwarmNodeAgentProgressItem> report,
        CancellationToken cancellationToken)
    {
        if (current is null || current.DesiredState == SwarmNodeAgentDesiredState.Removed)
        {
            await CompleteOperationAsync(platform, running, SwarmNodeAgentOperationState.Completed, null, cancellationToken);
            return Result.Success();
        }

        if (!string.IsNullOrWhiteSpace(current.DockerServiceId))
        {
            await reconciliationCoordinator.RefreshAsync(platform.Id, cancellationToken);
            var service = await unitOfWork.Swarm.GetServiceAsync(platform.Id, current.DockerServiceId, cancellationToken);
            if (service is not null)
            {
                if (!SwarmNodeAgentInfrastructure.HasOwnership(service, platform))
                    return Result.Failure(new ConflictError("Refusing to delete a Service without the persisted Citadel ownership labels."));
                var delete = await swarmConnectorFactory.GetConnector(platform.ConnectorType).DeleteServiceAsync(
                    new DeleteManagedSwarmServiceCommand(platform.Address, running.OperationId!.Value, service.DockerServiceId),
                    cancellationToken);
                if (delete.IsFailure(out var deleteError))
                    return Result.Failure(deleteError!);
            }
        }

        var bootstraps = await unitOfWork.EdgeAgents.GetNodeAgentBootstrapsAsync(platform.Id, cancellationToken);
        var caConfigId = current.DockerCaConfigId;
        await unitOfWork.EdgeAgents.RevokeNodeAgentBootstrapsAsync(platform.Id, DateTime.UtcNow, cancellationToken);
        var bindings = await unitOfWork.EdgeAgents.GetNodeBindingsAsync(platform.Id, cancellationToken);
        foreach (var binding in bindings.Where(static binding => !binding.IsRevoked && binding.DockerNodeId is not null))
        {
            await unitOfWork.EdgeAgents.RevokeNodeBindingAsync(
                platform.Id,
                binding.DockerNodeId!,
                DateTime.UtcNow,
                "Node-agent data plane removed by an administrator.",
                cancellationToken);
            sessionTerminator.Disconnect(platform.Id, binding.DockerNodeId!, "Node-agent data plane was removed.");
        }

        var removed = current with
        {
            DesiredState = SwarmNodeAgentDesiredState.Removed,
            DockerServiceId = null,
            DockerCaConfigId = null,
            DockerCaConfigName = null,
            OperationId = running.OperationId,
            OperationKind = running.OperationKind,
            OperationState = SwarmNodeAgentOperationState.Completed,
            OperationStartedAtUtc = running.OperationStartedAtUtc,
            OperationActorId = running.OperationActorId,
            OperationError = null,
            UpdatedAtUtc = DateTime.UtcNow
        };
        await unitOfWork.EdgeAgents.UpsertNodeAgentInstallationAsync(removed, cancellationToken);
        await CompleteOperationAsync(
            platform,
            removed,
            SwarmNodeAgentOperationState.Completed,
            null,
            cancellationToken);

        var connector = swarmConnectorFactory.GetConnector(platform.ConnectorType);
        foreach (var secretId in bootstraps.Select(static value => value.DockerSecretId).Where(static value => !string.IsNullOrWhiteSpace(value)).Distinct(StringComparer.Ordinal))
        {
            var deleteSecret = await connector.DeleteSecretAsync(
                new DeleteSwarmSecretCommand(platform.Address, secretId!),
                cancellationToken);
            if (deleteSecret.IsFailure(out var deleteSecretError))
            {
                logger.LogWarning(
                    "Could not delete owned node-agent bootstrap Secret {SecretId} for platform {PlatformId}: {Error}",
                    secretId,
                    platform.Id,
                    deleteSecretError!.Message);
                report(new(
                    platform.Id,
                    running.OperationId!.Value,
                    "cleanup",
                    $"The node-agent Service was removed, but bootstrap Secret '{secretId}' could not be deleted.",
                    IsWarning: true));
            }
        }
        if (!string.IsNullOrWhiteSpace(caConfigId))
        {
            await DeleteOwnedCaConfigAsync(
                platform,
                caConfigId,
                running.OperationId!.Value,
                report,
                cancellationToken);
        }
        report(new(platform.Id, running.OperationId!.Value, "removed", "Removed the Citadel node-agent Service and revoked its credentials and sessions."));
        return Result.Success();
    }

    private async Task<Result> WaitForCoverageAsync(
        Platform platform,
        IReadOnlyCollection<SwarmNodeProjection> eligible,
        SwarmNodeAgentInstallation installation,
        Action<SwarmNodeAgentProgressItem> report,
        CancellationToken cancellationToken)
    {
        var deadline = DateTime.UtcNow.AddMinutes(Math.Clamp(options.Value.NodeAgentSetupMinutes, 1, 15));
        while (DateTime.UtcNow < deadline)
        {
            await reconciliationCoordinator.RefreshAsync(platform.Id, cancellationToken);
            var connected = eligible.Count(node => sessionStatus.IsNodeConnected(platform.Id, node.DockerNodeId));
            if (connected == eligible.Count)
            {
                report(new(platform.Id, installation.OperationId!.Value, "coverage", $"Node-agent coverage converged on {connected} satellite node(s)."));
                return Result.Success();
            }

            if (!string.IsNullOrWhiteSpace(installation.DockerServiceId))
            {
                var service = await unitOfWork.Swarm.GetServiceAsync(platform.Id, installation.DockerServiceId, cancellationToken);
                if (service is not null && string.Equals(service.UpdateState, "paused", StringComparison.OrdinalIgnoreCase))
                    return Result.Failure(new ConflictError(service.UpdateMessage ?? "The node-agent Service rollout paused."));
            }
            await Task.Delay(TimeSpan.FromSeconds(2), cancellationToken);
        }

        return Result.Failure(new ConflictError("Node-agent setup timed out with partial coverage. Run Repair coverage after checking the missing nodes."));
    }

    private async Task<(SwarmNodeAgentBootstrap Bootstrap, byte[] TokenBytes)> CreateBootstrapAsync(
        Platform platform,
        Guid actorId,
        CancellationToken cancellationToken)
    {
        var previous = await unitOfWork.EdgeAgents.GetLatestNodeAgentBootstrapAsync(platform.Id, cancellationToken);
        var tokenBytes = RandomNumberGenerator.GetBytes(32);
        var token = Convert.ToBase64String(tokenBytes).TrimEnd('=').Replace('+', '-').Replace('/', '_');
        CryptographicOperations.ZeroMemory(tokenBytes);
        var now = DateTime.UtcNow;
        var version = (previous?.Version ?? 0) + 1;
        var bootstrap = new SwarmNodeAgentBootstrap(
            Guid.CreateVersion7(),
            platform.Id,
            platform.ClusterId!,
            version,
            EdgeAgentManagementService.HashToken(token),
            null,
            $"citadel-node-agent-{platform.Id:N}-bootstrap-{version}",
            now.AddMinutes(Math.Clamp(options.Value.NodeAgentBootstrapMinutes, 1, 30)),
            null,
            actorId,
            now,
            now);
        await unitOfWork.EdgeAgents.AddNodeAgentBootstrapAsync(bootstrap, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        return (bootstrap, Encoding.UTF8.GetBytes(token));
    }

    private async Task<SwarmNodeAgentBootstrap?> RecoverBootstrapSecretAsync(
        Platform platform,
        SwarmNodeAgentBootstrap bootstrap,
        CancellationToken cancellationToken)
    {
        var secret = (await unitOfWork.Swarm.GetSecretsAsync(platform.Id, cancellationToken))
            .FirstOrDefault(value => string.Equals(value.Name, bootstrap.DockerSecretName, StringComparison.Ordinal));
        if (secret is null || !SwarmNodeAgentInfrastructure.HasOwnership(secret.Labels, platform))
            return null;

        await unitOfWork.EdgeAgents.UpdateNodeAgentBootstrapSecretAsync(
            bootstrap.Id,
            secret.DockerSecretId,
            DateTime.UtcNow,
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        return bootstrap with
        {
            DockerSecretId = secret.DockerSecretId,
            UpdatedAtUtc = DateTime.UtcNow
        };
    }

    private async Task DeletePriorBootstrapSecretsAsync(
        Platform platform,
        Guid currentBootstrapId,
        Guid operationId,
        Action<SwarmNodeAgentProgressItem> report,
        CancellationToken cancellationToken)
    {
        var connector = swarmConnectorFactory.GetConnector(platform.ConnectorType);
        var priorSecrets = (await unitOfWork.EdgeAgents.GetNodeAgentBootstrapsAsync(platform.Id, cancellationToken))
            .Where(value => value.Id != currentBootstrapId && !string.IsNullOrWhiteSpace(value.DockerSecretId))
            .Select(static value => value.DockerSecretId!)
            .Distinct(StringComparer.Ordinal)
            .ToArray();
        foreach (var secretId in priorSecrets)
        {
            var delete = await connector.DeleteSecretAsync(
                new DeleteSwarmSecretCommand(platform.Address, secretId),
                cancellationToken);
            if (delete.IsFailure(out var error))
            {
                logger.LogWarning(
                    "Could not delete prior node-agent bootstrap Secret {SecretId} for platform {PlatformId}: {Error}",
                    secretId,
                    platform.Id,
                    error!.Message);
                report(new(
                    platform.Id,
                    operationId,
                    "cleanup",
                    $"Prior bootstrap Secret '{secretId}' is no longer active but could not yet be deleted.",
                    IsWarning: true));
            }
        }
    }

    private async Task DeleteUnreferencedBootstrapSecretAsync(Platform platform, string secretId)
    {
        try
        {
            using var cleanupTimeout = new CancellationTokenSource(TimeSpan.FromSeconds(15));
            var deleted = await swarmConnectorFactory.GetConnector(platform.ConnectorType).DeleteSecretAsync(
                new DeleteSwarmSecretCommand(platform.Address, secretId),
                cleanupTimeout.Token);
            if (deleted.IsFailure(out var error))
            {
                logger.LogWarning(
                    "Could not delete unreferenced node-agent bootstrap Secret {SecretId} for platform {PlatformId}: {Error}",
                    secretId,
                    platform.Id,
                    error!.Message);
            }
        }
        catch (Exception exception)
        {
            logger.LogWarning(
                exception,
                "Could not delete unreferenced node-agent bootstrap Secret {SecretId} for platform {PlatformId}.",
                secretId,
                platform.Id);
        }
    }

    private async Task RevokeBootstrapsAfterFailureAsync(Guid platformId)
    {
        try
        {
            using var cleanupTimeout = new CancellationTokenSource(TimeSpan.FromSeconds(15));
            await unitOfWork.EdgeAgents.RevokeNodeAgentBootstrapsAsync(
                platformId,
                DateTime.UtcNow,
                cleanupTimeout.Token);
            await unitOfWork.CommitAsync(cleanupTimeout.Token);
        }
        catch (Exception exception)
        {
            logger.LogError(
                exception,
                "Failed to revoke node-agent bootstrap credentials after a lifecycle failure for platform {PlatformId}.",
                platformId);
        }
    }

    private async Task<Result<ResolvedAgentImage>> ResolveImageAsync(
        Platform platform,
        IReadOnlySet<string> requiredArchitectures,
        CancellationToken cancellationToken)
    {
        var reference = options.Value.GetAgentImage();
        var inspected = await imageConnectorFactory.GetConnector(platform.ConnectorType).DistributionInspectAsync(
            new DistributionInspectCommand(platform.Address, reference, null),
            cancellationToken);
        if (!inspected.IsSuccess(out var distribution, out var error))
            return Result.Failure<ResolvedAgentImage>(error!);
        if (string.IsNullOrWhiteSpace(distribution.Descriptor.Digest))
            return Result.Failure<ResolvedAgentImage>(new BadGatewayError("The Agent image registry returned no immutable digest."));

        var imageArchitectures = GetLinuxArchitectures(distribution);
        var missingArchitectures = requiredArchitectures
            .Where(architecture => !imageArchitectures.Contains(architecture))
            .Order(StringComparer.OrdinalIgnoreCase)
            .ToArray();
        if (missingArchitectures.Length > 0)
        {
            return Result.Failure<ResolvedAgentImage>(new BadRequestError(
                $"The Agent image manifest does not support: {string.Join(", ", missingArchitectures)}."));
        }

        var repository = RemoveTag(reference);
        return Result.Success(new ResolvedAgentImage(reference, distribution.Descriptor.Digest, $"{repository}@{distribution.Descriptor.Digest}"));
    }

    internal static HashSet<string> GetLinuxArchitectures(DistributionResult distribution)
    {
        var architectures = (distribution.Platforms ?? [])
            .Where(static platform => string.Equals(platform.Os, "linux", StringComparison.OrdinalIgnoreCase)
                                      && !string.IsNullOrWhiteSpace(platform.Architecture))
            .Select(static platform => NormalizeArchitecture(platform.Architecture))
            .ToHashSet(StringComparer.OrdinalIgnoreCase);
        var descriptorPlatform = distribution.Descriptor.Platform;
        if (descriptorPlatform is not null
            && string.Equals(descriptorPlatform.Os, "linux", StringComparison.OrdinalIgnoreCase)
            && !string.IsNullOrWhiteSpace(descriptorPlatform.Architecture))
        {
            architectures.Add(NormalizeArchitecture(descriptorPlatform.Architecture));
        }

        return architectures;
    }

    private SystemSwarmServiceSpec BuildSystemSpec(
        Platform platform,
        DockerSwarmPlatformDescriptor descriptor,
        string pinnedImage,
        string coreUrl,
        SwarmNodeAgentBootstrap bootstrap,
        IReadOnlySet<string> supportedArchitectures,
        string? caConfigId,
        string? caConfigName)
    {
        var environment = new List<string>
        {
            "CITADEL_AGENT_MODE=edge",
            "CITADEL_EDGE_AGENT_PROFILE=swarm-node",
            $"CITADEL_CORE_URL={coreUrl.TrimEnd('/')}",
            "CITADEL_EDGE_BOOTSTRAP_FILE=/run/secrets/citadel-edge-bootstrap",
            "CITADEL_EDGE_AGENT_KEY_PATH=/app/data/edge-agent.key",
            "CITADEL_EDGE_IDENTITY_PATH=/app/data/edge-agent.identity.json",
            $"CITADEL_PLATFORM_ID={platform.Id:D}",
            "CITADEL_SWARM_SERVICE_ID={{.Service.ID}}",
            "CITADEL_SWARM_TASK_ID={{.Task.ID}}",
            "CITADEL_SWARM_NODE_ID={{.Node.ID}}",
            "CITADEL_SWARM_NODE_HOSTNAME={{.Node.Hostname}}"
        };
        if (!string.IsNullOrWhiteSpace(caConfigId))
            environment.Add("CITADEL_EDGE_CORE_CA_CERTIFICATE_PATH=/run/configs/citadel-core-ca.crt");

        return new SystemSwarmServiceSpec(
            pinnedImage,
            environment,
            descriptor.NodeID,
            $"citadel_swarm_node_agent_{platform.Id:N}",
            bootstrap.DockerSecretId!,
            bootstrap.DockerSecretName,
            caConfigId,
            caConfigName,
            Math.Clamp(options.Value.NodeAgentLimitNanoCpus, 100_000_000, 2_000_000_000),
            Math.Clamp(options.Value.NodeAgentLimitMemoryBytes, 128L * 1024 * 1024, 2L * 1024 * 1024 * 1024),
            Math.Clamp(options.Value.NodeAgentPidsLimit, 64, 1024),
            30_000_000_000,
            supportedArchitectures.Order(StringComparer.OrdinalIgnoreCase).ToArray());
    }

    private async Task<Result<ResolvedCaConfig>> ResolveCaConfigAsync(
        Platform platform,
        SwarmNodeAgentInstallation installation,
        Action<SwarmNodeAgentProgressItem> report,
        CancellationToken cancellationToken)
    {
        var previousId = installation.DockerCaConfigId;
        var certificatePath = transportOptions.Value.CaCertificatePath;
        if (string.IsNullOrWhiteSpace(certificatePath))
            return Result.Success(new ResolvedCaConfig(null, null, previousId));

        var file = new FileInfo(certificatePath);
        if (!file.Exists || file.Length is <= 0 or > 1024 * 1024)
        {
            return Result.Failure<ResolvedCaConfig>(new BadRequestError(
                "AgentTransport:CaCertificatePath must reference a non-empty PEM bundle no larger than 1 MiB."));
        }

        byte[] certificate;
        try
        {
            certificate = await File.ReadAllBytesAsync(file.FullName, cancellationToken);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            return Result.Failure<ResolvedCaConfig>(new BadRequestError(
                "AgentTransport:CaCertificatePath could not be read."));
        }

        try
        {
            var connector = swarmConnectorFactory.GetConnector(platform.ConnectorType);
            var configs = await unitOfWork.Swarm.GetConfigsAsync(platform.Id, cancellationToken);
            var current = string.IsNullOrWhiteSpace(previousId)
                ? null
                : configs.FirstOrDefault(value => string.Equals(
                    value.DockerConfigId,
                    previousId,
                    StringComparison.Ordinal));
            if (current is not null)
            {
                if (!SwarmNodeAgentInfrastructure.HasOwnership(current.Labels, platform))
                {
                    return Result.Failure<ResolvedCaConfig>(new ConflictError(
                        "The persisted node-agent CA Config no longer has valid Citadel ownership labels."));
                }

                var currentData = await connector.GetConfigDataAsync(
                    new InspectSwarmConfigCommand(platform.Address, current.DockerConfigId),
                    cancellationToken);
                if (currentData.IsSuccess(out var data, out _) && data.AsSpan().SequenceEqual(certificate))
                    return Result.Success(new ResolvedCaConfig(current.DockerConfigId, current.Name, previousId));
            }

            var contentHash = Convert.ToHexString(SHA256.HashData(certificate)).ToLowerInvariant()[..12];
            var name = $"citadel-node-agent-{platform.Id:N}-ca-{contentHash}";
            var recovered = configs.FirstOrDefault(value => string.Equals(value.Name, name, StringComparison.Ordinal));
            if (recovered is not null)
            {
                if (!SwarmNodeAgentInfrastructure.HasOwnership(recovered.Labels, platform))
                {
                    return Result.Failure<ResolvedCaConfig>(new ConflictError(
                        $"A Docker Config named '{name}' exists without Citadel node-agent ownership labels."));
                }
                return Result.Success(new ResolvedCaConfig(recovered.DockerConfigId, recovered.Name, previousId));
            }

            var created = await connector.CreateConfigAsync(
                new CreateSwarmConfigCommand(
                    platform.Address,
                    name,
                    certificate,
                    SwarmNodeAgentInfrastructure.OwnershipLabels(platform)),
                cancellationToken);
            if (!created.IsSuccess(out var result, out var error))
                return Result.Failure<ResolvedCaConfig>(error!);

            report(new(
                platform.Id,
                installation.OperationId!.Value,
                "trust",
                "Mounted the configured Citadel Core CA certificate for node Agents."));
            return Result.Success(new ResolvedCaConfig(result.ResourceId, name, previousId));
        }
        finally
        {
            CryptographicOperations.ZeroMemory(certificate);
        }
    }

    private async Task DeleteOwnedCaConfigAsync(
        Platform platform,
        string configId,
        Guid operationId,
        Action<SwarmNodeAgentProgressItem> report,
        CancellationToken cancellationToken)
    {
        var config = await unitOfWork.Swarm.GetConfigAsync(platform.Id, configId, cancellationToken);
        if (config is null)
            return;
        if (!SwarmNodeAgentInfrastructure.HasOwnership(config.Labels, platform))
        {
            report(new(
                platform.Id,
                operationId,
                "cleanup",
                $"Config '{configId}' was preserved because its Citadel ownership labels do not match.",
                IsWarning: true));
            return;
        }

        var deleted = await swarmConnectorFactory.GetConnector(platform.ConnectorType).DeleteConfigAsync(
            new DeleteSwarmConfigCommand(platform.Address, configId),
            cancellationToken);
        if (deleted.IsFailure(out var error))
        {
            logger.LogWarning(
                "Could not delete owned node-agent CA Config {ConfigId} for platform {PlatformId}: {Error}",
                configId,
                platform.Id,
                error!.Message);
            report(new(
                platform.Id,
                operationId,
                "cleanup",
                $"The old CA Config '{configId}' could not yet be deleted.",
                IsWarning: true));
        }
    }

    private async Task CompleteOperationAsync(
        Platform platform,
        SwarmNodeAgentInstallation installation,
        SwarmNodeAgentOperationState state,
        string? error,
        CancellationToken cancellationToken)
    {
        var current = await unitOfWork.EdgeAgents.GetNodeAgentInstallationAsync(installation.PlatformId, cancellationToken)
                      ?? installation;
        await unitOfWork.EdgeAgents.UpsertNodeAgentInstallationAsync(current with
        {
            OperationId = installation.OperationId,
            OperationKind = installation.OperationKind,
            OperationState = state,
            OperationStartedAtUtc = installation.OperationStartedAtUtc,
            OperationActorId = installation.OperationActorId,
            OperationError = error,
            UpdatedAtUtc = DateTime.UtcNow
        }, cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(
            PlatformActivity.NodeAgentLifecycle(
                platform,
                installation.OperationActorId!.Value,
                installation.OperationKind!.Value,
                installation.OperationId!.Value,
                state,
                error ?? "Node-agent lifecycle operation completed."),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await TryNotifyCoverageChangedAsync(installation.PlatformId, cancellationToken);
    }

    private async Task TryCompleteFailedOperationAsync(
        Platform platform,
        SwarmNodeAgentInstallation installation,
        string error)
    {
        try
        {
            using var cleanupTimeout = new CancellationTokenSource(TimeSpan.FromSeconds(15));
            await CompleteOperationAsync(
                platform,
                installation,
                SwarmNodeAgentOperationState.Failed,
                error,
                cleanupTimeout.Token);
        }
        catch (Exception exception)
        {
            logger.LogError(
                exception,
                "Failed to persist failed node-agent operation {OperationId} for platform {PlatformId}.",
                installation.OperationId,
                installation.PlatformId);
        }
    }

    private async Task TryNotifyCoverageChangedAsync(Guid platformId, CancellationToken cancellationToken)
    {
        try
        {
            await hubDispatcher.SendSwarmNodeAgentCoverageChanged(platformId, cancellationToken);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception exception)
        {
            logger.LogWarning(
                exception,
                "Could not publish the Swarm node-agent coverage change for platform {PlatformId}.",
                platformId);
        }
    }

    private static bool TryValidateCoreUrl(string coreUrl, out string error)
    {
        error = "EdgeAgent:PublicGrpcUrl must be an externally reachable HTTP or HTTPS origin.";
        if (!Uri.TryCreate(coreUrl, UriKind.Absolute, out var uri)
            || (uri.Scheme != Uri.UriSchemeHttp && uri.Scheme != Uri.UriSchemeHttps)
            || string.IsNullOrWhiteSpace(uri.Host)
            || uri.IsLoopback
            || uri.AbsolutePath != "/"
            || !string.IsNullOrEmpty(uri.Query)
            || !string.IsNullOrEmpty(uri.Fragment)
            || !string.IsNullOrEmpty(uri.UserInfo))
        {
            return false;
        }
        error = string.Empty;
        return true;
    }

    private static string RemoveTag(string reference)
    {
        var digestIndex = reference.IndexOf('@');
        if (digestIndex >= 0)
            return reference[..digestIndex];
        var slash = reference.LastIndexOf('/');
        var colon = reference.LastIndexOf(':');
        return colon > slash ? reference[..colon] : reference;
    }

    private static string NormalizeArchitecture(string architecture) => architecture.Trim().ToLowerInvariant() switch
    {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "armv7l" => "arm",
        var value => value
    };

    private sealed record ResolvedCaConfig(string? ConfigId, string? ConfigName, string? PreviousConfigId);
    private sealed record ResolvedAgentImage(string Reference, string Digest, string PinnedReference);
}
