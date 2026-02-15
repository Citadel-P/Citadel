using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Domain.Entities;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal sealed class DeploymentAutoUpdateJob(
    IServiceScopeFactory scopeFactory,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IImageConnector> connectorFactory,
    IDbWorkQueue dbWorkQueue,
    ILogger<DeploymentAutoUpdateJob> logger) : BackgroundService
{
    private const int CheckIntervalInHours = 6;

    protected override Task ExecuteAsync(CancellationToken stoppingToken)
        => Helpers.DelayWithJitterFor(RunPeriodicAutoUpdate, cancellationToken: stoppingToken);

    private async Task RunPeriodicAutoUpdate(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                var deployments = await GetDeploymentsAsync(cancellationToken);

                foreach (var deployment in deployments)
                {
                    var result = await CheckDeploymentAsync(deployment, cancellationToken);

                    if (result is not null)
                    {
                        await dbWorkQueue.EnqueueAsync(result, cancellationToken);
                    }
                }
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error occurred while running deployment auto-update job.");
            }

            await Task.Delay(TimeSpan.FromHours(CheckIntervalInHours), cancellationToken);
        }
    }

    private async Task<IEnumerable<Deployment>> GetDeploymentsAsync(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        return await uow.Deployments.GetAllAsync(cancellationToken) ?? [];
    }

    private async Task<IDbWorkItem?> CheckDeploymentAsync(Deployment deployment, CancellationToken cancellationToken)
    {
        if (deployment.Spec?.UpdateBehavior == UpdateBehavior.Disabled)
            return null;

        if (deployment.Spec?.Image is not ExternalImage external)
            return null;

        if (!TrySplitImageTag(external.ImageTag, out var repository, out _))
        {
            return new DeploymentAutoUpdateFailedWorkItem(deployment.Id, "Invalid image tag.");
        }

        if (!platformContainerCache.TryGetCacheEntry(deployment.PlatformId, out var platform, out _))
        {
            return new DeploymentAutoUpdateFailedWorkItem(deployment.Id, "Platform unavailable or disconnected.");
        }

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var registry = await uow.Registries.GetAsync(external.RegistryId, cancellationToken);
        if (registry == null || registry.Status != RegistryStatus.Active)
        {
            return new DeploymentAutoUpdateFailedWorkItem(deployment.Id, "Registry unavailable or inactive.");
        }

        string registryDomain = registry.RegistryHost.Replace("https://", "").ToLower();
        string? auth = registry.Configuration.GetRegistryAuth(registryDomain);

        var repositoryName = repository.Split('/', StringSplitOptions.RemoveEmptyEntries).Last();

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        var cmd = new DistributionInspectCommand(platform.Address, repositoryName, auth);

        var inspectResult = await connector.DistributionInspectAsync(cmd, cancellationToken);
        if (inspectResult.IsFailure(out var error, out var inspect))
        {
            return new DeploymentAutoUpdateFailedWorkItem(deployment.Id, error.Message);
        }

        var remoteDigest = inspect.Descriptor.Digest;
        var currentDigest = deployment.Container?.DockerImageId;
        var matchedTag = string.Compare(remoteDigest, currentDigest, StringComparison.OrdinalIgnoreCase) == 0;

        var now = DateTime.UtcNow;

        if (matchedTag)
        {
            return new DeploymentAutoUpdateStateWorkItem(
                deployment.Id,
                new AutoUpdateState(now, AutoUpdateStatus.UpToDate, remoteDigest, remoteDigest)
            );
        }

        logger.LogInformation(
            "Auto-update available for deployment {DeploymentId}: new digest detected.",
            deployment.Id);

        return new DeploymentAutoUpdateStateWorkItem(
            deployment.Id,
            new AutoUpdateState(now, AutoUpdateStatus.UpdateAvailable, currentDigest, remoteDigest)
        );
    }

    private static bool TrySplitImageTag(string imageTag, out string repository, out string tag)
    {
        repository = string.Empty;
        tag = string.Empty;

        if (string.IsNullOrWhiteSpace(imageTag) || imageTag.Contains('@'))
            return false;

        var parts = imageTag.Split(':', 2, StringSplitOptions.RemoveEmptyEntries);
        repository = parts[0];
        tag = parts.Length > 1 ? parts[1] : "latest";
        return !string.IsNullOrWhiteSpace(repository);
    }
}

internal sealed class DeploymentAutoUpdateStateWorkItem(
    Guid deploymentId,
    AutoUpdateState state) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        var deployment = await uow.Deployments.GetAsync(deploymentId, cancellationToken);
        if (deployment is null) return;

        deployment.SetAutoUpdateState(state);
        await uow.Deployments.UpdateAsync(deployment, cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }
}

internal sealed class DeploymentAutoUpdateFailedWorkItem(
    Guid deploymentId,
    string message) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        var deployment = await uow.Deployments.GetAsync(deploymentId, cancellationToken);
        if (deployment is null) return;

        var now = DateTime.UtcNow;
        deployment.SetAutoUpdateState(
            new AutoUpdateState(
                now,
                AutoUpdateStatus.Failed,
                deployment.AutoUpdateState?.CurrentDigest,
                deployment.AutoUpdateState?.RemoteDigest,
                message));

        await uow.Deployments.UpdateAsync(deployment, cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }
}