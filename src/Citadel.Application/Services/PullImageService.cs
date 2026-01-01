using Application.Mappers;
using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Images;
using Domain.Entities;
using Domain.Entities.Registries;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Runtime.CompilerServices;
using static Application.Services.PullImageService;

namespace Application.Services;

internal interface IPullImageService
{
    IAsyncEnumerable<PullImageStreamItem> PullAsync(PullImageInput command, CancellationToken cancellationToken);
}

internal class PullImageService(
    IDbWorkQueue dbWorkQueue,
    IImageStreamManager imageStream,
    INotificationQueue notificationQueue,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IImageConnector> connectorFactory,
    IServiceScopeFactory scopeFactory) : IPullImageService
{
    public async IAsyncEnumerable<PullImageStreamItem> PullAsync(PullImageInput command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(command.PlatformId, out var platform, out _))
        {
            var message = $"Platform with ID {command.PlatformId} not found or disconnected.";
            yield return new PullImageStreamItem(ErrorMessage: message, Error: new ImagePullError(404, message));
            yield break;
        }

        Registry? registry;
        await using (var scope = scopeFactory.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            registry = await uow.Registries.GetAsync(command.RegistryId, cancellationToken);
        }

        if (registry == null)
        {
            var message = $"Registry '{command.RegistryId}' not found.";
            yield return new PullImageStreamItem(ErrorMessage: message, Error: new ImagePullError(404, message));
            yield break;
        }

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        var streamCmd = command.ToConnectorCommand(platform.Address, registry);
        await foreach (var reply in connector.PullImageProgressStreamAsync(streamCmd, cancellationToken))
        {
            yield return reply;
        }

        // Fetch metadata
        var imageResult = await connector.GetAsync(platform.Address, streamCmd.FromImage, cancellationToken);
        if (!imageResult.IsSuccess(out var image, out var error))
        {
            var message = $"Pull completed, but failed to retrieve image info: {error?.Message}.";
            yield return new PullImageStreamItem(ErrorMessage: message, Error: new ImagePullError(404, message));
            yield break;
        }

        var imageEntity = image.Map(platform.Id, registry);
        yield return new PullImageStreamItem(DockerImageId: imageEntity.DockerImageId);

        var workItem = new PersistPulledImageWorkItem(
            imageEntity,
            platform,
            imageStream,
            notificationQueue
        );

        await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
    }

    internal sealed class PersistPulledImageWorkItem(
        Image imageEntity,
        PlatformCacheEntry platform,
        IImageStreamManager imageStream,
        INotificationQueue notificationQueue)
        : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            // Write to DB
            await uow.Images.AddOrUpdateAsync(imageEntity, cancellationToken);
            await uow.CommitAsync(cancellationToken);

            // Notify clients
            var imageNotification = new SendImageNotificationWorkItem(imageStream, imageEntity, platform.Id);
            await notificationQueue.EnqueueAsync(imageNotification, cancellationToken);
        }
    }

    internal class SendImageNotificationWorkItem(IImageStreamManager imageStream, Image image, Guid platformId) : INotificationWorkItem
    {
        public Task ExecuteAsync(CancellationToken cancellationToken)
            => imageStream.SendImageInfo(platformId, image);
    }

    internal sealed record PullImageInput(Guid PlatformId, Guid RegistryId, string ImageTag)
    {

        internal PullImageCommand ToConnectorCommand(string platformAddress, Registry registry)
        {
            string domainName = registry.RegistryHost.Replace("https://", "").ToLower();
            switch (registry.Configuration)
            {
                case GitHubRegistry ghCfg:
                    return new PullImageCommand
                        (
                            PlatformAddress: platformAddress,
                            FromImage: $"{domainName}/{ghCfg.NameSpace}/{BuildImageAndTag()}",
                            Auth: ghCfg.GhcrAuthEnabled == true ? ghCfg.GetRegistryAuth(domainName) : null
                        );

                case CustomRegistry customCfg:
                    return new PullImageCommand
                    (
                        PlatformAddress: platformAddress,
                        FromImage: $"{domainName}/{BuildImageAndTag()}",
                        Auth: customCfg.AuthEnabled == true ? customCfg.GetRegistryAuth(domainName) : null
                    );

                case DockerHubRegistry dockerCfg:
                    if (RegistryId == Constants.DefaultRegistryId)
                    {
                        return new PullImageCommand
                            (
                                PlatformAddress: platformAddress,
                                FromImage: BuildImageAndTag()
                            );
                    }
                    else
                    {
                        return new PullImageCommand
                            (
                                PlatformAddress: platformAddress,
                                FromImage: $"{domainName}/{dockerCfg.UserName}/{ImageTag}".ToLower(),
                                Auth: dockerCfg.GetRegistryAuth(domainName)
                            );
                    }

                default:
                    throw new NotSupportedException("Unsupported registry configuration");
            }
        }

        private string BuildImageAndTag()
            => (ImageTag.Split(':').Length == 1 ? $"{ImageTag}:latest" : ImageTag).ToLower();
    }

}
