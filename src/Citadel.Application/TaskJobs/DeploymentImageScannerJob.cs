using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal sealed class DeploymentImageScannerJob(
    IImageScanScheduler imageScanScheduler,
    IPlatformContainerCache platformContainerCache,
    IDelayWithJitterService delayWithJitterService,
    IConnectorFactory<IImageConnector> connectorFactory,
    ImageDigestCache imageDigestCache,
    ILogger<DeploymentImageScannerJob> logger) : BackgroundService
{
    private const int CheckIntervalInMinutes = 90;

    protected override Task ExecuteAsync(CancellationToken stoppingToken)
        => delayWithJitterService.DelayWithJitterForAsync(RunPeriodicScanAsync, cancellationToken: stoppingToken);

    private async Task RunPeriodicScanAsync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                var scanTasks = await imageScanScheduler.LoadScanTasksAsync(cancellationToken);
                var connectorsByType = new Dictionary<PlatformConnectorType, IImageConnector>();

                foreach (var scanTask in scanTasks)
                {
                    try
                    {
                        await ScanImageAsync(scanTask, connectorsByType, cancellationToken);
                    }
                    catch (Exception ex)
                    {
                        logger.LogError(ex, "Scan failed for image {ImageKey}", scanTask.Key);
                    }
                }
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Image scan failed.");
            }

            await Task.Delay(TimeSpan.FromMinutes(CheckIntervalInMinutes), cancellationToken);
        }
    }

    private async Task ScanImageAsync(
        ImageScanTask scanTask,
        Dictionary<PlatformConnectorType, IImageConnector> connectorsByType,
        CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(scanTask.PlatformId, out var platform, out _))
            return;

        if (!connectorsByType.TryGetValue(platform.ConnectorType, out var connector))
        {
            connector = connectorFactory.GetConnector(platform.ConnectorType);
            connectorsByType[platform.ConnectorType] = connector;
        }

        var registryHost = scanTask.Registry.RegistryHost.Contains("://", StringComparison.Ordinal)
            ? scanTask.Registry.RegistryHost
            : $"https://{scanTask.Registry.RegistryHost}";
        var registryDomain = new Uri(registryHost).Host.ToLowerInvariant();
        var auth = scanTask.Registry.Configuration.GetRegistryAuth(registryDomain);
        var imageName = $"{scanTask.Key.Repository}:{scanTask.Key.Tag}";

        var cmd = new DistributionInspectCommand(platform.Address, imageName, auth);
        var result = await connector.DistributionInspectAsync(cmd, cancellationToken);
        if (result.IsFailure(out var error, out var inspect))
        {
            logger.LogError("Registry inspect failed for {Image}: {Error}", imageName, error);
            return;
        }

        imageDigestCache.Set(scanTask.Key, inspect.Descriptor.Digest);
    }
}
