using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Entities.Platforms;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal sealed class SwarmTaskContainerPruner(
    IServiceScopeFactory scopeFactory,
    ILogger<SwarmTaskContainerPruner> logger)
{
    private const int BatchSize = 100;

    public async Task PruneAsync(
        PlatformHealth platform,
        IContainerConnector connector,
        IEnumerable<DockerContainer> containers,
        CancellationToken cancellationToken)
    {
        var historicalContainerIds = containers
            .Where(IsPrunable)
            .Select(static container => container.Id)
            .Distinct(StringComparer.Ordinal)
            .Take(BatchSize)
            .ToArray();
        if (historicalContainerIds.Length == 0)
            return;

        bool pruningEnabled;
        try
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var configuredPlatform = await unitOfWork.Platforms.GetByIdAsync(platform.Id, cancellationToken);
            pruningEnabled = configuredPlatform is
            {
                PruneHistoricalSwarmTaskContainers: true,
                PlatformDescriptor: DockerSwarmPlatformDescriptor
            };
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception ex)
        {
            logger.LogWarning(
                ex,
                "Unable to read historical Swarm task pruning configuration for platform {PlatformId}.",
                platform.Id);
            return;
        }

        if (!pruningEnabled)
            return;

        var deleteResult = await connector.DeleteAsync(
            new DeleteContainerCommand(
                historicalContainerIds,
                platform.Address,
                Volume: false,
                Force: false,
                Link: false),
            cancellationToken);
        if (deleteResult.IsFailure(out var error))
        {
            logger.LogWarning(
                "Failed to prune {Count} historical Swarm task containers for platform {PlatformId}: {Error}",
                historicalContainerIds.Length,
                platform.Id,
                error.Message);
            return;
        }

        logger.LogInformation(
            "Pruned {Count} historical Swarm task containers for platform {PlatformId}.",
            historicalContainerIds.Length,
            platform.Id);
    }

    private static bool IsPrunable(DockerContainer container)
        => container.IsSwarmTask
           && container.State is ContainerStateStatus.Exited
               or ContainerStateStatus.Dead;
}
