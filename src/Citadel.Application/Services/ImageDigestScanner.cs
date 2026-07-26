using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.Logging;

namespace Application.Services;

internal interface IImageDigestScanner
{
    Task<Result<string>> ScanAsync(ImageScanTask task, CancellationToken cancellationToken);
}

internal sealed class ImageDigestScanner(
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IImageConnector> connectorFactory,
    ILogger<ImageDigestScanner> logger) : IImageDigestScanner
{
    public async Task<Result<string>> ScanAsync(
        ImageScanTask task,
        CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(task.PlatformId, out var platform, out _))
        {
            return Result.Failure<string>(
                new ConflictError("The platform is disconnected or unavailable."));
        }

        var registryHost = task.Registry.RegistryHost.Contains("://", StringComparison.Ordinal)
            ? task.Registry.RegistryHost
            : $"https://{task.Registry.RegistryHost}";
        if (!Uri.TryCreate(registryHost, UriKind.Absolute, out var registryUri)
            || string.IsNullOrWhiteSpace(registryUri.Host))
        {
            return Result.Failure<string>(
                new BadRequestError("The configured registry host is invalid."));
        }

        var imageName = $"{task.Key.Repository}:{task.Key.Tag}";

        try
        {
            var connector = connectorFactory.GetConnector(platform.ConnectorType);
            var registryDomain = registryUri.Host.ToLowerInvariant();
            var auth = task.Registry.Configuration.GetRegistryAuth(registryDomain);
            var command = new DistributionInspectCommand(platform.Address, imageName, auth);
            var result = await connector.DistributionInspectAsync(command, cancellationToken);

            if (result.IsFailure(out var error, out var inspect))
            {
                logger.LogWarning(
                    "Registry inspect failed for image {Image} on platform {PlatformId}: {Error}",
                    imageName,
                    task.PlatformId,
                    error.Message);
                return RemoteFailure();
            }

            var digest = inspect.Descriptor.Digest;
            if (string.IsNullOrWhiteSpace(digest))
            {
                logger.LogWarning(
                    "Registry inspect returned no digest for image {Image} on platform {PlatformId}",
                    imageName,
                    task.PlatformId);
                return RemoteFailure();
            }

            return Result.Success(digest);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception ex)
        {
            logger.LogWarning(
                ex,
                "Registry inspect failed for image {Image} on platform {PlatformId}",
                imageName,
                task.PlatformId);
            return RemoteFailure();
        }
    }

    private static Result<string> RemoteFailure()
        => Result.Failure<string>(
            new BadGatewayError("The registry update check failed. Verify registry connectivity and credentials."));
}
