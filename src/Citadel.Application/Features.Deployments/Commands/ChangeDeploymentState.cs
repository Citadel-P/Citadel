using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Entities.Deployments;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Write)]
public sealed record ChangeDeploymentState(IEnumerable<Guid> Ids, DeploymentAction Action) : ICommand<Result>;

internal sealed class ChangeDeploymentStateHandler(
    IDeploymentProcessingService deploymentProcessingService,
    IContainerProcessingService containerProcessingService,
    IPlatformContainerCache platformContainerCache,
    IUserContextAccessor userContext,
    IConnectorFactory<IContainerConnector> connectorFactory,
    IHostApplicationLifetime applicationLifetime,
    ILogger<ChangeDeploymentStateHandler> logger) : ICommandHandler<ChangeDeploymentState, Result>
{
    private static readonly TimeSpan CompletionTimeout = TimeSpan.FromSeconds(30);
    private static readonly TimeSpan RollbackTimeout = TimeSpan.FromSeconds(5);

    public async ValueTask<Result> Handle(ChangeDeploymentState command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var requestedIds = command.Ids.Distinct().ToArray();
        var deployments = await deploymentProcessingService.MarkProcessingAsync(requestedIds, actorId, cancellationToken);

        if (deployments.Count == 0)
        {
            return Result.Failure(new NotFoundError("No deployments found for the provided deployment ID(s)."));
        }

        if (deployments.Any(deployment => deployment.Platform?.PlatformDescriptor.Type == PlatformType.DockerSwarm))
        {
            await TryRollbackProcessingAsync(deployments);
            return Result.Failure(new BadRequestError("Container state actions are not available for Docker Swarm deployments."));
        }

        using var completionCancellation = CancellationTokenSource.CreateLinkedTokenSource(
            applicationLifetime.ApplicationStopping);
        completionCancellation.CancelAfter(CompletionTimeout);
        var completionToken = completionCancellation.Token;
        try
        {
            var containers = deployments.Select(s => s.Container).Where(s => s is not null)!.ToArray();
            if (containers.Length == 0)
            {
                await TryRollbackProcessingAsync(deployments);
                return Result.Failure(new NotFoundError("No containers found for the provided deployment ID(s)."));
            }

            if (!platformContainerCache.TryGetPlatformsWithContainers([.. containers.Select(s => s!.DockerContainerId)], out var platformContainers))
            {
                await TryRollbackProcessingAsync(deployments);
                return Result.Failure(new NotFoundError("Platform resolution failed for ID(s). Platform may be disconnected."));
            }

            await deploymentProcessingService.NotifyProcessingAsync(deployments, ct: completionToken);

            var containerAction = ActionMap.GetValueOrDefault(command.Action, ContainerAction.START);
            foreach (var platform in platformContainers)
            {
                var result = await PatchPlatformAsync(platform, containerAction, completionToken);
                if (result.IsFailure())
                {
                    await TryRollbackProcessingAsync(deployments);
                    return result;
                }
            }

            await containerProcessingService.CompleteProcessingAsync(
                new ProcessedResources([], deployments, []),
                platformContainers,
                actorId);
            return Result.Success();
        }
        catch
        {
            await TryRollbackProcessingAsync(deployments);
            throw;
        }
    }

    private async Task TryRollbackProcessingAsync(IReadOnlyCollection<Deployment> deployments)
    {
        using var rollbackCancellation = new CancellationTokenSource(RollbackTimeout);
        try
        {
            await deploymentProcessingService.RollbackProcessingAsync(
                deployments,
                rollbackCancellation.Token);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to roll back deployment command claims");
        }
    }

    private Task<Result> PatchPlatformAsync(PlatformCacheEntry platform, ContainerAction containerAction, CancellationToken ct)
    {
        var cmd = new PatchContainerCommand(
            Action: containerAction,
            PlatformAddress: platform.Address,
            ContainerIds: platform.Containers.Select(s => s.Key));

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        return connector.PatchAsync(cmd, ct);
    }

    private static readonly IReadOnlyDictionary<DeploymentAction, ContainerAction> ActionMap =
        new Dictionary<DeploymentAction, ContainerAction>
        {
            [DeploymentAction.RESTART] = ContainerAction.RESTART,
            [DeploymentAction.PAUSE] = ContainerAction.PAUSE,
            [DeploymentAction.UNPAUSE] = ContainerAction.UNPAUSE,
            [DeploymentAction.STOP] = ContainerAction.STOP,
            [DeploymentAction.START] = ContainerAction.START
        };
}
